/*!
# `Refract GTK` - Window
*/

use crate::{
	Candidate,
	CLI_FORMATS,
	CLI_MODES,
	CLI_NO_AVIF,
	CLI_NO_JXL,
	CLI_NO_LOSSLESS,
	CLI_NO_LOSSY,
	CLI_NO_WEBP,
	CLI_NO_YCBCR,
	MainTx,
	Share,
	ShareFeedback,
	SharePayload,
	SisterRx,
	SisterTx,
};
use dactyl::{
	NicePercent,
	NiceU64,
	NiceU8,
	traits::NiceInflection,
};
use dowser::{
	Dowser,
	Extension,
};
use gtk::{
	FileChooserAction,
	FileFilter,
	gdk_pixbuf::Pixbuf,
	prelude::*,
	ResponseType,
};
use refract_core::{
	EncodeIter,
	FLAG_NO_AVIF_YCBCR,
	FLAG_NO_LOSSLESS,
	FLAG_NO_LOSSY,
	ImageKind,
	Input,
	Output,
	Quality,
	RefractError,
};
use std::{
	borrow::Cow,
	cell::{
		Cell,
		RefCell,
	},
	ffi::OsStr,
	num::NonZeroUsize,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



// The E_AVIF, E_JPEG, E_JPG, E_JXL, E_PNG, and E_WEBP constants are generated
// by build.rs.
include!(concat!(env!("OUT_DIR"), "/refract-extensions.rs"));

// State flags.
const FLAG_LOCK_ENCODING: u8 = 0b0000_0001; // We're in the middle of encoding.
const FLAG_LOCK_FEEDBACK: u8 = 0b0000_0010; // Candidate feedback is required.
const FLAG_LOCK_PAINT: u8 =    0b0000_0100; // Don't paint.
const FLAG_TICK_IMAGE: u8 =    0b0000_1000; // We need to repaint the image.
const FLAG_TICK_STATUS: u8 =   0b0001_0000; // We need to repaint the status.
const FLAG_TICK_AB: u8 =       0b0010_0000; // We need to repaint format labels.



/// # Helper: Pango-Formatted Span.
macro_rules! log_colored {
	($color:literal, $inner:expr) => (
		concat!("<span foreground=\"", $color, "\">", $inner, "</span>")
	);
	($color:literal, $inner:expr, true) => (
		concat!("<span foreground=\"", $color, "\" weight=\"bold\">", $inner, "</span>")
	);
}

/// # Helper: Pango-Formatted Span (for log message prefix).
///
/// This works like [`log_colored`] bold but adds a trailing space, and
/// optionally leading whitespace.
macro_rules! log_prefix {
	($color:literal, $prefix:literal) => (
		concat!(log_colored!($color, $prefix, true), " ")
	);
	($before:literal, $color:literal, $prefix:literal) => (
		concat!($before, log_colored!($color, $prefix, true), " ")
	);
}

/// # Helper: GTK Objects From Builder.
macro_rules! gtk_obj {
	($builder:ident, $key:literal) => (
		$builder.object($key).ok_or(RefractError::GtkInit)?
	);
}

/// # Helper: Toggle GTK Widget Sensitivity En Masse.
macro_rules! gtk_sensitive {
	($sensitive:expr, $($obj:expr),+) => ($(
		if $obj.is_sensitive() != $sensitive {
			$obj.set_sensitive($sensitive);
		}
	)+);
}

/// # Helper: GTK Resource Path.
macro_rules! gtk_src {
	($file:literal) => (concat!("/gtk/refract/", $file));
}




#[derive(Debug, Clone)]
/// # Image Source.
///
/// This is yet another image middleware object, embedding a `Pixbuf` along
/// with the encoding quality, encoding iteration count, and the raw file size.
///
/// If only we could share `Pixbuf` across threads...
struct WindowSource {
	buf: Pixbuf,
	quality: Quality,
	count: u8,
	size: usize,
}

impl From<Candidate> for WindowSource {
	#[inline]
	fn from(src: Candidate) -> Self {
		let quality = src.quality;
		let count = src.count;
		let size = src.size;

		Self {
			buf: Pixbuf::from(src),
			quality,
			count,
			size,
		}
	}
}

impl WindowSource {
	/// # Format Value.
	///
	/// This returns a value suitable for the `lbl_format_val` widget. It is
	/// the image kind, optionally with an iteration number (for candidates).
	fn format_val(&self) -> Cow<str> {
		if self.count == 0 { Cow::Borrowed(self.quality.kind().as_str()) }
		else {
			Cow::Owned([
				self.quality.kind().as_str(),
				" #",
				NiceU8::from(self.count).as_str(),
			].concat())
		}
	}

	#[inline]
	/// # Quality Label.
	///
	/// This returns a value suitable for the `lbl_quality` widget. Currently
	/// it always reads "Quality" or "Quantizer" (for AVIF).
	fn quality(&self) -> String { [self.quality.label_title(), ":"].concat() }

	/// # Quality.
	///
	/// This returns a value suitable for the `lbl_quality_val` widget. This
	/// will be a normalized quality value like "1.0" unless encoding was
	/// lossless, in which case it will be a word.
	fn quality_val(&self) -> Cow<str> {
		if self.quality.is_lossless() {
			if self.count == 0 { Cow::Borrowed("Original") }
			else { Cow::Borrowed("Lossless") }
		}
		else { Cow::Owned(self.quality.quality().to_string()) }
	}
}



#[derive(Debug, Clone)]
/// # Window.
///
/// This holds the various GTK widgets we need to access after initialization,
/// along with a little bit of useful data.
///
/// It's pretty monstrous, but what can you do?
pub(super) struct Window {
	flags: Cell<u8>,
	paths: RefCell<Vec<PathBuf>>,
	dir: RefCell<Option<PathBuf>>,
	status: RefCell<String>,
	source: RefCell<Option<WindowSource>>,
	candidate: RefCell<Option<WindowSource>>,

	flt_image: FileFilter,
	flt_avif: FileFilter,
	flt_jxl: FileFilter,
	flt_webp: FileFilter,

	pub(super) wnd_main: gtk::ApplicationWindow,
	wnd_image: gtk::ScrolledWindow,
	pub(super) wnd_status: gtk::ScrolledWindow,

	pub(super) img_main: gtk::Image,
	pub(super) box_ab: gtk::Box,

	pub(super) btn_discard: gtk::Button,
	pub(super) btn_keep: gtk::Button,
	pub(super) btn_toggle: gtk::Switch,

	pub(super) chk_avif: gtk::CheckMenuItem,
	pub(super) chk_jxl: gtk::CheckMenuItem,
	pub(super) chk_webp: gtk::CheckMenuItem,
	pub(super) chk_lossless: gtk::CheckMenuItem,
	pub(super) chk_lossy: gtk::CheckMenuItem,
	pub(super) chk_ycbcr: gtk::CheckMenuItem,
	pub(super) chk_dark: gtk::CheckMenuItem,

	pub(super) lbl_format: gtk::Label,
	pub(super) lbl_format_val: gtk::Label,
	pub(super) lbl_quality: gtk::Label,
	pub(super) lbl_quality_val: gtk::Label,

	pub(super) lbl_status: gtk::Label,

	pub(super) mnu_file: gtk::MenuItem,
	pub(super) mnu_settings: gtk::MenuItem,
	pub(super) mnu_help: gtk::MenuItem,

	pub(super) mnu_about: gtk::MenuItem,
	pub(super) mnu_fopen: gtk::MenuItem,
	pub(super) mnu_dopen: gtk::MenuItem,
	pub(super) mnu_quit: gtk::MenuItem,

	spn_loading: gtk::Spinner,
}

/// ## Instantiation.
impl Window {
	/// # New Instance.
	pub(super) fn new(app: &gtk::Application, flags: u8) -> Result<Self, RefractError> {
		// Start the builder.
		let builder = gtk::Builder::new();
		builder.add_from_resource(gtk_src!("refract.glade"))
			.map_err(|_| RefractError::GtkInit)?;

		// Create the main UI shell.
		let out = Self {
			flags: Cell::new(FLAG_TICK_STATUS),
			paths: RefCell::new(Vec::new()),
			dir: RefCell::new(None),
			status: RefCell::new(String::from(concat!(
				log_prefix!("#9b59b6", "Refract GTK"),
				log_colored!("#ff3596", concat!("v", env!("CARGO_PKG_VERSION")), true),
				"\n",
				log_colored!("#999", "Tweak the settings (if you want to), then select an image or directory to encode!"),
				"\n",
				log_colored!("#999", "----"),
			))),
			source: RefCell::new(None),
			candidate: RefCell::new(None),

			flt_image: gtk_obj!(builder, "flt_image"),
			flt_avif: gtk_obj!(builder, "flt_avif"),
			flt_jxl: gtk_obj!(builder, "flt_jxl"),
			flt_webp: gtk_obj!(builder, "flt_webp"),

			wnd_main: gtk_obj!(builder, "wnd_main"),
			wnd_image: gtk_obj!(builder, "wnd_image"),
			wnd_status: gtk_obj!(builder, "wnd_status"),

			img_main: gtk_obj!(builder, "img_main"),
			box_ab: gtk_obj!(builder, "box_ab"),

			btn_discard: gtk_obj!(builder, "btn_discard"),
			btn_keep: gtk_obj!(builder, "btn_keep"),
			btn_toggle: gtk_obj!(builder, "btn_toggle"),

			chk_avif: gtk_obj!(builder, "chk_avif"),
			chk_jxl: gtk_obj!(builder, "chk_jxl"),
			chk_webp: gtk_obj!(builder, "chk_webp"),
			chk_lossless: gtk_obj!(builder, "chk_lossless"),
			chk_lossy: gtk_obj!(builder, "chk_lossy"),
			chk_ycbcr: gtk_obj!(builder, "chk_ycbcr"),

			chk_dark: gtk_obj!(builder, "chk_dark"),

			lbl_format: gtk_obj!(builder, "lbl_format"),
			lbl_format_val: gtk_obj!(builder, "lbl_format_val"),
			lbl_quality: gtk_obj!(builder, "lbl_quality"),
			lbl_quality_val: gtk_obj!(builder, "lbl_quality_val"),

			lbl_status: gtk_obj!(builder, "lbl_status"),

			mnu_file: gtk_obj!(builder, "mnu_file"),
			mnu_settings: gtk_obj!(builder, "mnu_settings"),
			mnu_help: gtk_obj!(builder, "mnu_help"),

			mnu_about: gtk_obj!(builder, "mnu_about"),
			mnu_fopen: gtk_obj!(builder, "mnu_fopen"),
			mnu_dopen: gtk_obj!(builder, "mnu_dopen"),
			mnu_quit: gtk_obj!(builder, "mnu_quit"),

			spn_loading: gtk_obj!(builder, "spn_loading"),
		};

		// Close down with the window.
		out.wnd_main.connect_delete_event(|_, _| {
			gtk::main_quit();
			Inhibit(false)
		});

		// Start with a fun image.
		out.img_main.set_from_resource(Some(gtk_src!("start.png")));

		// Hook up some styles.
		set_widget_style(&out.btn_discard, gtk_src!("btn-discard.css"));
		set_widget_style(&out.btn_keep, gtk_src!("btn-keep.css"));
		set_widget_style(&out.spn_loading, gtk_src!("spn-loading.css"));
		set_widget_style(&out.wnd_image, gtk_src!("wnd-image.css"));

		// Change encoder defaults?
		match flags & CLI_FORMATS {
			0 | CLI_FORMATS => {},
			_ => {
				if 0 != flags & CLI_NO_AVIF {
					out.chk_avif.set_active(false);
				}
				if 0 != flags & CLI_NO_JXL {
					out.chk_jxl.set_active(false);
				}
				if 0 != flags & CLI_NO_WEBP {
					out.chk_webp.set_active(false);
				}
			},
		}

		// Change encoding mode defaults?
		match flags & CLI_MODES {
			0 | CLI_MODES => {},
			_ => {
				if 0 != flags & CLI_NO_LOSSLESS {
					out.chk_lossless.set_active(false);
				}
				else if 0 != flags & CLI_NO_LOSSY {
					out.chk_lossy.set_active(false);
				}
			},
		}

		// One other mode, only this isn't affected by the other two.
		if 0 != flags & CLI_NO_YCBCR {
			out.chk_ycbcr.set_active(false);
		}

		// Enable drag-and-drop.
		out.toggle_drag_and_drop(true);

		// Start it up!
		out.wnd_main.set_application(Some(app));
		out.wnd_main.show_all();
		out.wnd_main.maximize();

		Ok(out)
	}
}

/// ## Flags.
impl Window {
	/// # Add Flag.
	///
	/// Returns `true` if changed.
	fn add_flag(&self, flag: u8) -> bool {
		let flags = self.flags.get();
		if flag == flags & flag { false }
		else {
			self.flags.replace(flags | flag);
			true
		}
	}

	#[inline]
	/// # Has Flag?
	fn has_flag(&self, flag: u8) -> bool {
		flag == self.flags.get() & flag
	}

	/// # Remove Flag.
	///
	/// Returns `true` if changed.
	fn remove_flag(&self, flag: u8) -> bool {
		let flags = self.flags.get();
		if 0 == flags & flag { false }
		else {
			self.flags.replace(flags & ! flag);
			true
		}
	}

	/// # Finish Encoding.
	///
	/// This removes the source and candidate images, if they exist, and
	/// optionally clears the encoder lock.
	fn finish(&self, unlock: bool) {
		self.remove_source();
		if unlock {
			self.remove_flag(FLAG_LOCK_ENCODING);
			self.toggle_drag_and_drop(true);
			self.spn_loading.stop();
		}
	}
}

/// ## Encoder Stuff.
impl Window {
	/// # Encode!
	///
	/// Encode any paths that are queued up, returning a bool to indicate
	/// whether or not anything is happening.
	///
	/// Encoding is actually done in a separate thread using a complicated
	/// system of channels to share data back and forth. The early setup,
	/// though, can be dealt with before that point.
	pub(super) fn encode(
		&self,
		tx: &SisterTx,
		rx: &SisterRx,
	) -> bool {
		// We can abort early if we have no paths or are already encoding.
		if ! self.has_paths() || ! self.add_flag(FLAG_LOCK_ENCODING) { return false; }

		// Disable drag-and-drop. This will be re-enabled once encoding
		// finishes.
		self.toggle_drag_and_drop(false);

		// Pull out the data we need.
		let paths: Vec<PathBuf> = self.paths.borrow_mut().split_off(0);
		let encoders: Box<[ImageKind]> = self.encoders();
		let flags: u8 = self.encoder_flags();

		// Mention that we're starting.
		self.log_start(paths.len(), &encoders);
		self.spn_loading.start();

		// Shove the actual work into a separate thread.
		let tx2 = tx.clone();
		let rx2 = rx.clone();
		std::thread::spawn(move || {
			_encode_outer(paths, &encoders, flags, &tx2, &rx2);
		});

		true
	}

	/// # Encoder Flags.
	///
	/// This maps the UI settings to the equivalent [`EncodeIter`] flags.
	fn encoder_flags(&self) -> u8 {
		let mut flags: u8 = 0;

		if ! self.chk_lossy.is_active() { flags |= FLAG_NO_LOSSY; }
		else if ! self.chk_lossless.is_active() { flags |= FLAG_NO_LOSSLESS; }

		if ! self.chk_ycbcr.is_active() { flags |= FLAG_NO_AVIF_YCBCR; }

		flags
	}

	/// # Enabled Encoders.
	///
	/// Return an array of the enabled encoders.
	fn encoders(&self) -> Box<[ImageKind]> {
		let mut out: Vec<ImageKind> = Vec::with_capacity(3);
		if self.chk_webp.is_active() { out.push(ImageKind::Webp); }
		if self.chk_avif.is_active() { out.push(ImageKind::Avif); }
		if self.chk_jxl.is_active() { out.push(ImageKind::Jxl); }
		out.into_boxed_slice()
	}

	/// # Process Feedback.
	pub(super) fn feedback(&self, tx: &MainTx, status: ShareFeedback) {
		self.remove_candidate();
		self.paint();
		tx.send(status).unwrap();
	}

	/// # Has Encoders.
	pub(super) fn has_encoders(&self) -> bool {
		self.chk_avif.is_active() ||
		self.chk_jxl.is_active() ||
		self.chk_webp.is_active()
	}

	/// # Has (Lossy/Lossless) Modes.
	pub(super) fn has_modes(&self) -> bool {
		self.chk_lossless.is_active() || self.chk_lossy.is_active()
	}

	#[inline]
	/// # Is Encoding?
	fn is_encoding(&self) -> bool { self.has_flag(FLAG_LOCK_ENCODING) }
}

/// ## Images.
impl Window {
	/// # Has Candidate.
	fn has_candidate(&self) -> bool { self.candidate.borrow().is_some() }

	/// # Has Source.
	fn has_source(&self) -> bool { self.source.borrow().is_some() }

	/// # Remove Candidate.
	fn remove_candidate(&self) {
		if self.has_candidate() {
			self.remove_flag(FLAG_LOCK_FEEDBACK);
			self.candidate.borrow_mut().take();
			gtk_sensitive!(false, self.btn_discard, self.btn_keep, self.btn_toggle);
			self.toggle_preview(false, false);
			self.add_flag(FLAG_TICK_AB);
		}
	}

	/// # Remove Source.
	fn remove_source(&self) {
		if self.has_source() {
			self.remove_candidate();
			self.source.borrow_mut().take();
			self.toggle_preview(false, true);
		}
	}

	/// # Set Best.
	fn set_best(&self, mut path: PathBuf, src: Output)
	-> Result<ShareFeedback, RefractError> {
		// We still need a source.
		if ! self.has_source() {
			return Err(RefractError::MissingSource);
		}

		// This should already be gone.
		self.remove_candidate();
		self.toggle_spinner(false);

		// Save it.
		path = self.maybe_save(&path, &src)?;

		// Record the happiness.
		let old_size: usize = self.source.borrow()
			.as_ref()
			.map(|x| x.size)
			.ok_or(RefractError::MissingSource)?;
		self.log_saved(
			path,
			src.quality(),
			old_size,
			src.size().map_or(old_size, NonZeroUsize::get),
		);

		drop(src);
		Ok(ShareFeedback::Continue)
	}

	/// # Set Candidate.
	fn set_candidate(&self, src: Candidate) -> Result<ShareFeedback, RefractError> {
		if self.has_source() {
			self.candidate.borrow_mut().replace(WindowSource::from(src));
			self.toggle_preview(true, false);
			gtk_sensitive!(true, self.btn_discard, self.btn_keep, self.btn_toggle);
			self.add_flag(FLAG_LOCK_FEEDBACK | FLAG_TICK_AB);
			Ok(ShareFeedback::Wait)
		}
		else { Err(RefractError::MissingSource) }
	}

	/// # Set Image.
	///
	/// This method updates the `Pixbuf` associated with the `img_main` widget.
	///
	/// As this is a relatively heavy operation, a flag is used to track when
	/// the image actually needs updating, and this method will no-op if no
	/// update is required.
	///
	/// If `None` is passed, the image is cleared.
	///
	/// For source/candidate switching, this will also update the background
	/// class associated with the `wnd_image` widget.
	fn set_image(&self, img: Option<&Pixbuf>) {
		if self.remove_flag(FLAG_TICK_IMAGE) {
			// Set the done image.
			if img.is_none() && ! self.is_encoding() {
				self.img_main.set_from_resource(Some(gtk_src!("stop.png")));
			}
			// Set/unset the image as instructed.
			else {
				self.img_main.set_from_pixbuf(img);
			}

			// Toggle the background class.
			if img.is_some() && self.btn_toggle.is_active() {
				add_widget_class(&self.wnd_image, "preview_b");
			}
			else {
				remove_widget_class(&self.wnd_image, "preview_b");
			}
		}
	}

	#[allow(clippy::unnecessary_wraps)] // This is needed for branch consistency.
	/// # Set Source.
	fn set_source(&self, src: Candidate) -> Result<ShareFeedback, RefractError> {
		self.remove_candidate();
		self.source.borrow_mut().replace(WindowSource::from(src));
		self.toggle_preview(false, true);
		self.add_flag(FLAG_LOCK_ENCODING | FLAG_TICK_AB);
		Ok(ShareFeedback::Continue)
	}

	/// # Toggle Dark Mode.
	pub(super) fn toggle_dark(&self) {
		// Toggle the background class.
		if self.chk_dark.is_active() {
			add_widget_class(&self.wnd_image, "dark");
		}
		else {
			remove_widget_class(&self.wnd_image, "dark");
		}
	}

	/// # Toggle Preview.
	///
	/// This is a special handler for the source/candidate `btn_toggle` widget.
	/// It tries to ensure the switch state is sane given the current data, and
	/// will recurse as necessary.
	///
	/// Paint operations come with a lock, so in theory this should avoid
	/// redundant paints from the [`Window`] struct, but GTK may or may not
	/// operate with similar consideration. At worst, though, this would just
	/// be a +1 operation.
	pub(super) fn toggle_preview(&self, val: bool, force: bool) {
		if self.btn_toggle.is_active() != val {
			self.add_flag(FLAG_TICK_IMAGE | FLAG_LOCK_PAINT | FLAG_TICK_AB);
			self.btn_toggle.set_active(val);
			self.remove_flag(FLAG_LOCK_PAINT);
		}
		else if force { self.add_flag(FLAG_TICK_IMAGE | FLAG_TICK_AB); }
	}

	#[inline]
	/// # Toggle Spinner.
	fn toggle_spinner(&self, val: bool) {
		if val != self.spn_loading.is_active() {
			self.spn_loading.set_active(val);
		}
	}
}

/// ## Paths.
impl Window {
	/// # Toggle Drag-and-Drop.
	///
	/// File(s) can be dragged-and-dropped directly onto the main image to kick
	/// off the encoding process, except when encoding is already underway.
	fn toggle_drag_and_drop(&self, val: bool) {
		if val {
			self.img_main.drag_dest_set(
				gtk::DestDefaults::ALL,
				&[gtk::TargetEntry::new(
					"text/uri-list",
					gtk::TargetFlags::OTHER_APP,
					0,
				)],
				gtk::gdk::DragAction::COPY
			);
		}
		else {
			self.img_main.drag_dest_unset();
		}
	}

	/// # Add File.
	pub(super) fn add_file<P>(&self, path: P) -> bool
	where P: AsRef<Path> {
		let Ok(path) = std::fs::canonicalize(path) else { return false; };

		if
			path.is_file() &&
			is_jpeg_png(&path)
		{
			self.paths.borrow_mut().push(path);
			true
		}
		else { false }
	}

	/// # Add Directory.
	fn add_directory<P>(&self, path: P) -> bool
	where P: AsRef<Path> {
		// And find the paths.
		let mut paths: Vec<PathBuf> = Dowser::from(path.as_ref())
			.into_vec_filtered(is_jpeg_png);

		if paths.is_empty() { false }
		else {
			paths.sort();
			self.paths.borrow_mut().append(&mut paths);
			true
		}
	}

	/// # Make File Chooser Dialogue.
	///
	/// This makes a new file chooser dialogue of the specified kind, and
	/// optionally sets the working directory and/or filter.
	fn file_chooser<P>(
		&self,
		title: &str,
		action: FileChooserAction,
		btn: &str,
		dir: Option<P>,
		filter: Option<&FileFilter>,
	) -> gtk::FileChooserDialog
	where P: AsRef<Path> {
		let out = gtk::FileChooserDialog::with_buttons(
			Some(title),
			Some(&self.wnd_main),
			action,
			&[("_Cancel", ResponseType::Cancel), (btn, ResponseType::Accept)]
		);

		if let Some(filter) = filter {
			out.set_filter(filter);
		}

		if let Some(parent) = dir {
			out.set_current_folder(parent);
		}

		out
	}

	/// # Has Paths?
	fn has_paths(&self) -> bool { ! self.paths.borrow().is_empty() }

	/// # Add File Handler.
	///
	/// This creates, spawns, and kills a file selection dialogue, saving the
	/// chosen path and returning `true` if (likely) valid.
	pub(super) fn maybe_add_file(&self) -> bool {
		if self.is_encoding() { return false; }

		let window = self.file_chooser(
			"Choose an Image to Encode",
			FileChooserAction::Open,
			"_Open",
			self.dir.borrow().as_ref(),
			Some(&self.flt_image),
		);

		// Run and close the dialogue.
		let res = window.run();
		if ResponseType::None == res { return false; }
		window.emit_close();

		if ResponseType::Accept == res {
			if let Some(file) = window.filename() {
				// Store the "last used" directory for next time.
				if let Some(parent) = file.parent() {
					self.dir.borrow_mut().replace(parent.to_path_buf());
				}

				// Push image to the queue, if valid.
				self.add_file(file);
			}
		}

		// True if we have stuff now.
		self.has_paths()
	}

	/// # Add Directory Handler.
	///
	/// This creates, spawns, and kills a directory selection dialogue, saving
	/// the chosen path and returning `true` if it contained any valid images.
	pub(super) fn maybe_add_directory(&self) -> bool {
		if self.is_encoding() { return false; }

		let window = self.file_chooser(
			"Choose a Directory to Encode",
			FileChooserAction::SelectFolder,
			"_Select",
			self.dir.borrow().as_ref(),
			None,
		);

		// Disable folder creation before we start.
		window.set_create_folders(false);

		// Run and close the dialogue.
		let res = window.run();
		if ResponseType::None == res { return false; }
		window.emit_close();

		if ResponseType::Accept == res {
			if let Some(dir) = window.filename() {
				// Store the "last used" directory for next time.
				self.dir.borrow_mut().replace(dir.clone());

				// Push images to the queue, if any.
				self.add_directory(dir);
			}
		}

		// True if we have stuff now.
		self.has_paths()
	}

	/// # Maybe Save Handler.
	///
	/// This creates, spawns, and kills a file save dialogue, and writes the
	/// image data to the chosen path.
	///
	/// This will bubble up any errors encountered, including failure to
	/// choose an output path.
	///
	/// If successful, the path the file was saved to is returned.
	fn maybe_save(&self, path: &Path, src: &Output) -> Result<PathBuf, RefractError> {
		let kind = src.kind();
		let (filter, ext) = match kind {
			ImageKind::Avif => (&self.flt_avif, E_AVIF),
			ImageKind::Jxl => (&self.flt_jxl, E_JXL),
			ImageKind::Webp => (&self.flt_webp, E_WEBP),
			// It should not be possible to trigger this.
			_ => { return Err(RefractError::NoSave); },
		};

		let window = self.file_chooser(
			&["Save the ", kind.as_str(), "!"].concat(),
			FileChooserAction::Save,
			"_Save",
			path.parent(),
			Some(filter),
		);

		// Warn about collisions.
		window.set_do_overwrite_confirmation(true);

		// Suggest a file name.
		window.set_current_name(&[
			path.file_name().map_or_else(|| Cow::Borrowed("image"), OsStr::to_string_lossy).as_ref(),
			".",
			src.kind().extension(),
		].concat());

		// Read the result!
		let res = window.run();
		if ResponseType::None == res { return Err(RefractError::NoSave); }
		window.emit_close();

		// Make sure the chosen path has an appropriate extension. If not, toss
		// it onto the end.
		let mut path: PathBuf =
			if ResponseType::Accept == res { window.filename() }
			else { None }
			.ok_or(RefractError::NoSave)?;
		if ext != path {
			path = PathBuf::from(OsStr::from_bytes(&[
				path.as_os_str().as_bytes(),
				b".",
				kind.extension().as_bytes()
			].concat()));
		}

		// Save it.
		write_atomic::write_file(&path, src).map_err(|_| RefractError::Write)?;

		Ok(path)
	}
}

/// ## Painting.
impl Window {
	/// # Paint.
	pub(super) fn paint(&self) {
		if self.add_flag(FLAG_LOCK_PAINT) {
			self.paint_settings();
			self.paint_preview();
			self.paint_status();
			self.remove_flag(FLAG_LOCK_PAINT);
		}
	}

	/// # Paint Settings.
	///
	/// Really we just need to disable these fields when encoding is underway.
	fn paint_settings(&self) {
		let sensitive: bool = ! self.is_encoding();
		gtk_sensitive!(sensitive, self.mnu_file);
		gtk_sensitive!(sensitive, self.mnu_settings);
		gtk_sensitive!(sensitive, self.mnu_help);
	}

	/// # Paint Preview.
	///
	/// This updates format/quality labels, the a/b action area, and the image
	/// being displayed.
	fn paint_preview(&self) {
		// Preview bits only apply if we have a source.
		if self.has_source() {
			if ! self.lbl_quality.is_visible() {
				self.lbl_quality.show();
			}

			// Show/hide spinner.
			self.toggle_spinner(! self.has_candidate());

			// Which image are we dealing with?
			if self.remove_flag(FLAG_TICK_AB) {
				let ptr =
					if self.btn_toggle.is_active() {
						self.candidate.borrow()
					}
					else {
						self.source.borrow()
					};
				let src = ptr.as_ref().unwrap();

				self.lbl_format_val.set_text(&src.format_val());
				self.lbl_quality.set_text(&src.quality());
				self.lbl_quality_val.set_text(&src.quality_val());
				self.set_image(Some(&src.buf));
			}
		}
		else if self.lbl_quality.is_visible() {
			self.lbl_quality.hide();
			gtk_sensitive!(false, self.btn_discard, self.btn_keep, self.btn_toggle);
			self.set_image(None);
		}
	}

	#[inline]
	/// # Paint Status.
	///
	/// This writes the status log. Easy enough.
	fn paint_status(&self) {
		if self.remove_flag(FLAG_TICK_STATUS) {
			self.lbl_status.set_markup(self.status.borrow().as_str());
		}
	}
}

/// ## Sending/Receiving.
impl Window {
	/// # Process Share.
	///
	/// This method receives and processes a [`SharePayload`] sent from a
	/// sister thread.
	///
	/// This is used for actions like setting a new source or candidate image,
	/// saving a new image, ending an encoding run, or logging an error.
	///
	/// A response is sent back to the sister thread when finished. Most of the
	/// time the response is simply [`ShareFeedback::Continue`], but sometimes the
	/// sister thread needs a specific answer (and will get one).
	pub(super) fn process_share(&self, res: SharePayload)
	-> Result<ShareFeedback, RefractError> {
		let res = match res {
			Ok(Share::Path(x)) => {
				self.log_source(x);
				Ok(ShareFeedback::Continue)
			},
			Ok(Share::Source(x)) => self.set_source(x),
			Ok(Share::Encoder(x)) => {
				self.log_encoder(x);
				Ok(ShareFeedback::Continue)
			},
			Ok(Share::Candidate(x)) => self.set_candidate(x),
			Ok(Share::Best(path, x)) => self.set_best(path, x),
			Ok(Share::DoneEncoding) => {
				self.finish(true);
				self.log_done();
				Ok(ShareFeedback::Continue)
			},
			Err(e) => { Err(e) },
		};

		// Log an error?
		if let Err(e) = res { self.log_error(e); }

		res
	}
}

/// ## Statuses.
impl Window {
	/// # Log Done.
	///
	/// This happens when an encoding session finishes.
	fn log_done(&self) {
		let mut buf = self.status.borrow_mut();
		buf.push_str(concat!(
			log_prefix!("\n", "#9b59b6", "Notice:"),
			"Encoding has finished! ",
			log_colored!("#999", "(That's all, folks!)"),
			"\n",
			log_colored!("#999", "----"),
		));
		self.add_flag(FLAG_TICK_STATUS);
	}

	/// # Log Encoder.
	///
	/// This triggers when starting a new encoder for a given source.
	fn log_encoder(&self, enc: ImageKind) {
		let mut buf = self.status.borrow_mut();
		buf.push_str(concat!(log_prefix!("\n    ", "#ff3596", "Encoder:"), "Firing up the <b>"));
		buf.push_str(enc.as_str());
		buf.push_str("</b> encoder!");
		self.add_flag(FLAG_TICK_STATUS);
	}

	/// # Log Error.
	///
	/// This will add a formatted error to the log, unless the error has no
	/// value or is a duplicate of the previous entry.
	fn log_error(&self, err: RefractError) {
		let err = err.as_str();
		if err.is_empty() { return; }

		let mut buf = self.status.borrow_mut();
		buf.push_str(log_prefix!("\n    ", "#e74c3c", "Error:"));
		buf.push_str(err);
		self.add_flag(FLAG_TICK_STATUS);
	}

	/// # Log Saved.
	///
	/// This is used to indicate a new image has been saved.
	fn log_saved<P>(&self, path: P, quality: Quality, old_size: usize, new_size: usize)
	where P: AsRef<Path> {
		use std::fmt::Write;

		if 0 == old_size || 0 == new_size || new_size >= old_size { return; }

		// Crunch some numbers.
		let diff = old_size - new_size;
		let per = dactyl::int_div_float(diff, old_size).unwrap_or(0.0);

		let mut buf = self.status.borrow_mut();
		buf.push_str(log_prefix!("\n    ", "#2ecc71", "Success:"));
		let _res = write!(
			buf,
			concat!("Created <b>{}</b> with {}.", log_colored!("#999", "(Saved {} bytes, {}.)")),
			path.as_ref().to_string_lossy(),
			quality,
			NiceU64::from(diff).as_str(),
			NicePercent::from(per).as_str(),
		);
		self.add_flag(FLAG_TICK_STATUS);
	}

	/// # Log Source.
	///
	/// This is used when a new source image is being processed.
	fn log_source<P>(&self, src: P)
	where P: AsRef<Path> {
		let src = src.as_ref();
		let mut buf = self.status.borrow_mut();
		buf.push_str(concat!(log_prefix!("\n  ", "#00abc0", "Source:"), "<b>"));
		buf.push_str(src.to_string_lossy().as_ref());
		buf.push_str("</b>.");
		self.add_flag(FLAG_TICK_STATUS);
	}

	/// # Log Start.
	///
	/// This triggers when an encoding session starts.
	fn log_start(&self, count: usize, encoders: &[ImageKind]) {
		use oxford_join::OxfordJoin;
		use std::fmt::Write;

		if encoders.is_empty() || count == 0 { return; }

		let mut buf = self.status.borrow_mut();
		let _res = write!(
			buf,
			concat!(
				log_prefix!("\n", "#9b59b6", "Notice:"),
				"Refracting {} using {}! ",
				log_colored!("#999", "({}.)"),
			),
			count.nice_inflect("image", "images"),
			encoders.len().nice_inflect("encoder", "encoders"),
			encoders.oxford_and(),
		);
		self.add_flag(FLAG_TICK_STATUS);
	}
}

/// ## Miscellaneous.
impl Window {
	/// # Generate About Dialogue.
	pub(super) fn about(&self) -> gtk::AboutDialog {
		let about = gtk::AboutDialog::builder()
			.attached_to(&self.wnd_main)
			.authors(vec![
				env!("CARGO_PKG_AUTHORS").to_string(),
				String::from("Blobfolio https://blobfolio.com")
			])
			.comments(concat!(
				env!("CARGO_PKG_DESCRIPTION"),
				"\nFor best results, optimize your source images prior to running any conversions."
			))
			.copyright("\u{a9}2022 Blobfolio, LLC.")
			.license(include_str!("../LICENSE"))
			.license_type(gtk::License::Custom)
			.logo(&Pixbuf::from_resource(gtk_src!("comic.png")).unwrap())
			.program_name("Refract GTK")
			.version(env!("CARGO_PKG_VERSION"))
			.website(env!("CARGO_PKG_REPOSITORY"))
			.website_label("Source Code")
			.wrap_license(true)
			.build();

		// Give a shout-out to all the direct dependencies. This list
		// is generated by `build.rs`.
		about.add_credit_section(
			"Using",
			include!(concat!(env!("OUT_DIR"), "/about-credits.txt")),
		);

		about
	}
}



/// ## Encode Wrapper.
///
/// This is an outer wrapper over the individual file path(s). After all paths
/// have finished, it asks for the encoding lock to be removed.
fn _encode_outer(
	paths: Vec<PathBuf>,
	encoders: &[ImageKind],
	flags: u8,
	tx: &SisterTx,
	rx: &SisterRx,
) {
	for path in paths {
		if let Err(e) = _encode(&path, encoders, flags, tx, rx) {
			Share::sync(tx, rx, Err(e));
		}
	}

	Share::sync(tx, rx, Ok(Share::DoneEncoding));
}

/// # Encode!
///
/// This encoding wrapper runs every requested encoder against a single source
/// image. It will abort early if there are problems with the path, otherwise
/// it will guide the user through various qualities and save any "best"
/// candidates found.
fn _encode(
	path: &Path,
	encoders: &[ImageKind],
	flags: u8,
	tx: &SisterTx,
	rx: &SisterRx,
) -> Result<(), RefractError> {
	// Abort if there are no encoders.
	if encoders.is_empty() {
		return Err(RefractError::NoEncoders);
	}

	// First, let's read the main input.
	Share::sync(tx, rx, Ok(Share::Path(path.to_path_buf())));
	let (src, can) = _encode_source(path)?;
	if ShareFeedback::Abort == Share::sync(tx, rx, Ok(Share::Source(can))) {
		// The status isn't actually OK, but errors are already known, so this
		// prevents resubmitting the same error later.
		return Ok(());
	}

	for &e in encoders {
		Share::sync(tx, rx, Ok(Share::Encoder(e)));
		if let Ok(mut guide) = EncodeIter::new(&src, e, flags) {
			let mut count: u8 = 0;
			while let Some(can) = guide.advance().and_then(|out| Candidate::try_from(out).ok()) {
				count += 1;
				let res = Share::sync(tx, rx, Ok(Share::Candidate(can.with_count(count))));
				match res {
					ShareFeedback::Keep => { guide.keep(); },
					ShareFeedback::Discard => { guide.discard(); },
					ShareFeedback::Abort => { break; },
					_ => {},
				}
			}

			// Save the best, if any!
			Share::sync(tx, rx, guide.take().map(|x| Share::Best(path.to_path_buf(), x)));
		}
	}

	Ok(())
}

/// # Encode: Load Source.
///
/// This generates an [`Input`] and [`Candidate`] object from a given file
/// path, or dies trying.
fn _encode_source(path: &Path) -> Result<(Input, Candidate), RefractError> {
	let raw: &[u8] = &std::fs::read(path).map_err(|_| RefractError::Read)?;
	let out = Input::try_from(raw)?;
	let can = Candidate::try_from(&out)?;
	Ok((out, can))
}

/// # Add Widget Class.
///
/// This adds a class to a widget.
fn add_widget_class<W>(widget: &W, class: &str)
where W: WidgetExt {
	let style_context = widget.style_context();
	style_context.add_class(class);
}

/// # Is JPEG/PNG File.
pub(super) fn is_jpeg_png(path: &Path) -> bool {
	Extension::try_from3(path).map_or_else(
		|| Extension::try_from4(path).map_or(false, |e| e == E_JPEG),
		|e| e == E_JPG || e == E_PNG
	)
}

/// # Remove Widget Class.
///
/// This removes a class from a widget.
fn remove_widget_class<W>(widget: &W, class: &str)
where W: WidgetExt {
	let style_context = widget.style_context();
	style_context.remove_class(class);
}

/// # Set Widget Style.
///
/// This adds a CSS resource (mini stylesheet) to the given widget.
fn set_widget_style<W>(widget: &W, src: &str)
where W: WidgetExt {
	let style_context = widget.style_context();
	let provider = gtk::CssProvider::new();
	provider.load_from_resource(src);
	style_context.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
