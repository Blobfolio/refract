/*!
# `Refract GTK` - Window
*/

use atomic::Atomic;
use crate::{
	Candidate,
	gtk_obj,
	gtk_sensitive,
	Share,
	ShareFeedback,
	SharePayload,
};
use dactyl::{
	NicePercent,
	NiceU64,
};
use dowser::{
	Dowser,
	Extension,
};
use gdk_pixbuf::Pixbuf;
use gtk::{
	prelude::*,
	FileChooserAction,
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
	convert::TryFrom,
	ffi::OsStr,
	num::NonZeroUsize,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
	sync::{
		Arc,
		mpsc,
	},
};



// The extensions we're going to be looking for.
const E_AVIF: Extension = Extension::new4(*b"avif");
const E_JPEG: Extension = Extension::new4(*b"jpeg");
const E_JPG: Extension = Extension::new3(*b"jpg");
const E_JXL: Extension = Extension::new3(*b"jxl");
const E_PNG: Extension = Extension::new3(*b"png");
const E_WEBP: Extension = Extension::new4(*b"webp");

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
pub(super) struct WindowSource {
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
			Cow::Owned(format!("{} #{}", self.quality.kind(), self.count))
		}
	}

	#[inline]
	/// # Quality Label.
	///
	/// This returns a value suitable for the `lbl_quality` widget. Currently
	/// it always reads "Quality" or "Quantizer" (for AVIF).
	fn quality(&self) -> String {
		format!("{}:", self.quality.label_title())
	}

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
	pub(super) flags: Cell<u8>,
	pub(super) paths: RefCell<Vec<PathBuf>>,
	pub(super) dir: RefCell<Option<PathBuf>>,
	pub(super) status: RefCell<String>,
	pub(super) source: RefCell<Option<WindowSource>>,
	pub(super) candidate: RefCell<Option<WindowSource>>,

	pub(super) flt_image: gtk::FileFilter,
	pub(super) flt_avif: gtk::FileFilter,
	pub(super) flt_jxl: gtk::FileFilter,
	pub(super) flt_webp: gtk::FileFilter,

	pub(super) wnd_main: gtk::ApplicationWindow,
	pub(super) wnd_image: gtk::ScrolledWindow,
	pub(super) wnd_status: gtk::ScrolledWindow,

	pub(super) img_main: gtk::Image,
	pub(super) box_preview: gtk::Box,
	pub(super) box_ab: gtk::Box,
	pub(super) box_menu: gtk::MenuBar,

	pub(super) btn_discard: gtk::Button,
	pub(super) btn_keep: gtk::Button,
	pub(super) btn_toggle: gtk::Switch,

	pub(super) chk_avif: gtk::CheckMenuItem,
	pub(super) chk_jxl: gtk::CheckMenuItem,
	pub(super) chk_webp: gtk::CheckMenuItem,
	pub(super) chk_lossless: gtk::CheckMenuItem,
	pub(super) chk_lossy: gtk::CheckMenuItem,
	pub(super) chk_ycbcr: gtk::CheckMenuItem,

	pub(super) lbl_format: gtk::Label,
	pub(super) lbl_format_val: gtk::Label,
	pub(super) lbl_quality: gtk::Label,
	pub(super) lbl_quality_val: gtk::Label,

	pub(super) lbl_status: gtk::Label,

	pub(super) mnu_about: gtk::MenuItem,
	pub(super) mnu_fopen: gtk::MenuItem,
	pub(super) mnu_dopen: gtk::MenuItem,
	pub(super) mnu_quit: gtk::MenuItem,

	pub(super) spn_loading: gtk::Spinner,
}

impl TryFrom<&gtk::Application> for Window {
	type Error = RefractError;
	fn try_from(app: &gtk::Application) -> Result<Self, Self::Error> {
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
			box_preview: gtk_obj!(builder, "box_preview"),
			box_ab: gtk_obj!(builder, "box_ab"),
			box_menu: gtk_obj!(builder, "box_menu"),

			btn_discard: gtk_obj!(builder, "btn_discard"),
			btn_keep: gtk_obj!(builder, "btn_keep"),
			btn_toggle: gtk_obj!(builder, "btn_toggle"),

			chk_avif: gtk_obj!(builder, "chk_avif"),
			chk_jxl: gtk_obj!(builder, "chk_jxl"),
			chk_webp: gtk_obj!(builder, "chk_webp"),
			chk_lossless: gtk_obj!(builder, "chk_lossless"),
			chk_lossy: gtk_obj!(builder, "chk_lossy"),
			chk_ycbcr: gtk_obj!(builder, "chk_ycbcr"),

			lbl_format: gtk_obj!(builder, "lbl_format"),
			lbl_format_val: gtk_obj!(builder, "lbl_format_val"),
			lbl_quality: gtk_obj!(builder, "lbl_quality"),
			lbl_quality_val: gtk_obj!(builder, "lbl_quality_val"),

			lbl_status: gtk_obj!(builder, "lbl_status"),

			mnu_about: gtk_obj!(builder, "mnu_about"),
			mnu_fopen: gtk_obj!(builder, "mnu_fopen"),
			mnu_dopen: gtk_obj!(builder, "mnu_dopen"),
			mnu_quit: gtk_obj!(builder, "mnu_quit"),

			spn_loading: gtk_obj!(builder, "spn_loading"),
		};

		// Some window handlers.
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
		tx: &mpsc::Sender<SharePayload>,
		fb: &Arc<Atomic<ShareFeedback>>,
	) -> bool {
		// We can abort early if we have no paths or are already encoding.
		if ! self.has_paths() || ! self.add_flag(FLAG_LOCK_ENCODING) { return false; }

		// Pull out the data we need.
		let paths: Vec<PathBuf> = self.paths.borrow_mut().split_off(0);
		let encoders: Box<[ImageKind]> = self.encoders();
		let flags: u8 = self.encoder_flags();

		// Mention that we're starting.
		self.log_start(paths.len(), &encoders);
		self.spn_loading.start();

		// Shove the actual work into a separate thread.
		let tx2 = tx.clone();
		let fb2 = fb.clone();
		std::thread::spawn(move || {
			_encode_outer(paths, &encoders, flags, &tx2, &fb2);
		});

		true
	}

	/// # Encoder Flags.
	///
	/// This maps the UI settings to the equivalent [`EncodeIter`] flags.
	fn encoder_flags(&self) -> u8 {
		let mut flags: u8 = 0;

		if ! self.chk_lossy.get_active() { flags |= FLAG_NO_LOSSY; }
		else if ! self.chk_lossless.get_active() { flags |= FLAG_NO_LOSSLESS; }

		if ! self.chk_ycbcr.get_active() { flags |= FLAG_NO_AVIF_YCBCR; }

		flags
	}

	/// # Enabled Encoders.
	///
	/// Return an array of the enabled encoders.
	fn encoders(&self) -> Box<[ImageKind]> {
		let mut out: Vec<ImageKind> = Vec::with_capacity(3);
		if self.chk_webp.get_active() { out.push(ImageKind::Webp); }
		if self.chk_avif.get_active() { out.push(ImageKind::Avif); }
		if self.chk_jxl.get_active() { out.push(ImageKind::Jxl); }
		out.into_boxed_slice()
	}

	#[inline]
	/// # Has Encoders?
	pub(super) fn has_encoders(&self) -> bool {
		self.chk_webp.get_active() ||
		self.chk_avif.get_active() ||
		self.chk_jxl.get_active()
	}

	#[inline]
	/// # Has Modes?
	pub(super) fn has_modes(&self) -> bool {
		self.chk_lossless.get_active() ||
		self.chk_lossy.get_active()
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
	pub(super) fn remove_candidate(&self) {
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
	fn set_best(&self, mut path: PathBuf, src: Output) -> Result<ShareFeedback, RefractError> {
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
		let old_size: usize = self.source.borrow().as_ref().map(|x| x.size).ok_or(RefractError::MissingSource)?;
		self.log_saved(
			path,
			src.quality(),
			old_size,
			src.size().map_or(old_size, NonZeroUsize::get),
		);

		drop(src);
		Ok(ShareFeedback::Ok)
	}

	/// # Set Candidate.
	fn set_candidate(&self, src: Candidate) -> Result<ShareFeedback, RefractError> {
		if self.has_source() {
			self.candidate.borrow_mut().replace(WindowSource::from(src));
			self.toggle_preview(true, false);
			gtk_sensitive!(true, self.btn_discard, self.btn_keep, self.btn_toggle);
			self.add_flag(FLAG_LOCK_FEEDBACK | FLAG_TICK_AB);
			Ok(ShareFeedback::WantsFeedback)
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
			if img.is_some() && self.btn_toggle.get_active() {
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
		Ok(ShareFeedback::Ok)
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
		if self.btn_toggle.get_active() != val {
			self.add_flag(FLAG_TICK_IMAGE | FLAG_LOCK_PAINT | FLAG_TICK_AB);
			self.btn_toggle.set_active(val);
			self.remove_flag(FLAG_LOCK_PAINT);
		}
		else if force { self.add_flag(FLAG_TICK_IMAGE | FLAG_TICK_AB); }
	}

	#[inline]
	/// # Toggle Spinner.
	fn toggle_spinner(&self, val: bool) {
		if val != self.spn_loading.get_property_active() {
			self.spn_loading.set_property_active(val);
		}
	}
}

/// ## Paths.
impl Window {
	/// # Add File.
	fn add_file<P>(&self, path: P) -> bool
	where P: AsRef<Path> {
		let path = match std::fs::canonicalize(path) {
			Ok(p) => p,
			Err(_) => { return false; },
		};

		if
			path.is_file() &&
			Extension::try_from3(&path).map_or_else(
				|| Extension::try_from4(&path).map_or(false, |e| e == E_JPEG),
				|e| e == E_JPG || e == E_PNG
			)
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
		if let Ok(mut paths) = Vec::<PathBuf>::try_from(
			Dowser::filtered(|p|
				Extension::try_from3(p).map_or_else(
					|| Extension::try_from4(p).map_or(false, |e| e == E_JPEG),
					|e| e == E_JPG || e == E_PNG
				)
			)
				.with_paths(&[path])
		) {
			paths.sort();
			self.paths.borrow_mut().append(&mut paths);
			true
		}
		else { false }
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
		filter: Option<&gtk::FileFilter>,
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

		let res = window.run();
		if ResponseType::None == res { return false; }
		else if ResponseType::Accept == res {
			if let Some(file) = window.get_filename() {
				// Store the "last used" directory for next time.
				if let Some(parent) = file.parent() {
					self.dir.borrow_mut().replace(parent.to_path_buf());
				}

				// Push image to the queue, if valid.
				self.add_file(file);
			}
		}

		// Close the window.
		window.emit_close();

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

		// Disable folder creation.
		window.set_create_folders(false);

		let res = window.run();
		if ResponseType::None == res { return false; }
		else if ResponseType::Accept == res {
			if let Some(dir) = window.get_filename() {
				// Store the "last used" directory for next time.
				self.dir.borrow_mut().replace(dir.clone());

				// Push images to the queue, if any.
				self.add_directory(dir);
			}
		}

		// Close the window.
		window.emit_close();

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
		use std::io::Write;

		let kind = src.kind();
		let (filter, ext) = match kind {
			ImageKind::Avif => (&self.flt_avif, E_AVIF),
			ImageKind::Jxl => (&self.flt_jxl, E_JXL),
			ImageKind::Webp => (&self.flt_webp, E_WEBP),
			// It should not be possible to trigger this.
			_ => { return Err(RefractError::NoSave); },
		};

		let window = self.file_chooser(
			&format!("Save the {}!", kind),
			FileChooserAction::Save,
			"_Save",
			path.parent(),
			Some(filter),
		);

		// Warn about overwrites.
		window.set_do_overwrite_confirmation(true);

		// Suggest a file name.
		window.set_current_name(OsStr::from_bytes(&[
			path.file_name().map_or_else(|| &b"image"[..], OsStr::as_bytes),
			b".",
			src.kind().extension().as_bytes(),
		].concat()));

		// Read the result!
		let path: Option<PathBuf> = match window.run() {
			ResponseType::Accept => window.get_filename(),
			ResponseType::None => { return Err(RefractError::NoSave); },
			_ => None,
		};

		// Close the window.
		window.emit_close();

		// Make sure the chosen path has an appropriate extension. If not, toss
		// it onto the end.
		let mut path = path.ok_or(RefractError::NoSave)?;
		if ext != path {
			path = PathBuf::from(OsStr::from_bytes(&[
				path.as_os_str().as_bytes(),
				b".",
				kind.extension().as_bytes()
			].concat()));
		}

		// Touch the file to set sane default permissions.
		if ! path.exists() {
			std::fs::File::create(&path).map_err(|_| RefractError::Write)?;
		}

		// Save it.
		tempfile_fast::Sponge::new_for(&path)
			.and_then(|mut out| out.write_all(src).and_then(|_| out.commit()))
			.map_err(|_| RefractError::Write)?;

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
		gtk_sensitive!(sensitive, self.box_menu);
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
					if self.btn_toggle.get_active() {
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
	/// time the response is simply [`ShareFeedback::Ok`], but sometimes the
	/// sister thread needs a specific answer (and will get one).
	pub(super) fn process_share(&self, res: SharePayload)
	-> Result<ShareFeedback, RefractError> {
		let res = match res {
			Ok(Share::Path(x)) => {
				self.log_source(x);
				Ok(ShareFeedback::Ok)
			},
			Ok(Share::Source(x)) => self.set_source(x),
			Ok(Share::Encoder(x)) => {
				self.log_encoder(x);
				Ok(ShareFeedback::Ok)
			},
			Ok(Share::Candidate(x)) => self.set_candidate(x),
			Ok(Share::Best(path, x)) => self.set_best(path, x),
			Ok(Share::DoneEncoding) => {
				self.finish(true);
				self.log_done();
				Ok(ShareFeedback::Ok)
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
		if 0 == old_size || 0 == new_size || new_size >= old_size { return; }

		// Crunch some numbers.
		let diff = old_size - new_size;
		let per = dactyl::int_div_float(diff, old_size).unwrap_or(0.0);

		let mut buf = self.status.borrow_mut();
		buf.push_str(log_prefix!("\n    ", "#2ecc71", "Success:"));
		buf.push_str(&format!(
			concat!("Created <b>{}</b> with {}.", log_colored!("#999", "(Saved {} bytes, {}.)")),
			path.as_ref().to_string_lossy(),
			quality,
			NiceU64::from(diff).as_str(),
			NicePercent::from(per).as_str(),
		));
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
		use crate::l10n::{inflect, oxford_join};

		if encoders.is_empty() || count == 0 { return; }

		let mut buf = self.status.borrow_mut();
		buf.push_str(&format!(
			concat!(
				log_prefix!("\n", "#9b59b6", "Notice:"),
				"Refracting {} using {}! ",
				log_colored!("#999", "({}.)"),
			),
			inflect(count, "image", "images"),
			inflect(encoders.len(), "encoder", "encoders"),
			oxford_join(encoders, "and"),
		));
		self.add_flag(FLAG_TICK_STATUS);
	}
}

/// ## Miscellaneous.
impl Window {
	/// # About Dialog.
	pub(super) fn about(&self) -> Result<gtk::AboutDialog, RefractError> {
		let comic: Pixbuf = Pixbuf::from_resource(gtk_src!("comic.png"))
			.map_err(|_| RefractError::NothingDoing)?;

		let about = gtk::AboutDialogBuilder::new()
			.attached_to(&self.wnd_main)
			.authors(vec![env!("CARGO_PKG_AUTHORS").to_string(), String::from("Blobfolio https://blobfolio.com")])
			.comments(env!("CARGO_PKG_DESCRIPTION"))
			.copyright("\u{a9}2021 Blobfolio, LLC.")
			.license(include_str!("../skel/license.txt"))
			.license_type(gtk::License::Custom)
			.logo(&comic)
			.program_name("Refract GTK")
			.version(env!("CARGO_PKG_VERSION"))
			.website(env!("CARGO_PKG_REPOSITORY"))
			.website_label("Source Code")
			.build();

		// Give a shout-out to all the direct dependencies. This list is
		// generated by `build.rs`.
		about.add_credit_section(
			"Using",
			include!(concat!(env!("OUT_DIR"), "/about-credits.txt")),
		);

		Ok(about)
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
	tx: &mpsc::Sender<SharePayload>,
	fb: &Arc<Atomic<ShareFeedback>>,
) {
	paths.into_iter().for_each(|path| {
		if let Err(e) = _encode(&path, encoders, flags, tx, fb) {
			Share::sync(tx, fb, Err(e), false);
		}
	});

	Share::sync(tx, fb, Ok(Share::DoneEncoding), false);
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
	tx: &mpsc::Sender<SharePayload>,
	fb: &Arc<Atomic<ShareFeedback>>,
) -> Result<(), RefractError> {
	// Abort if there are no encoders.
	if encoders.is_empty() {
		return Err(RefractError::NoEncoders);
	}

	// First, let's read the main input.
	Share::sync(tx, fb, Ok(Share::Path(path.to_path_buf())), false);
	let (src, can) = _encode_source(path)?;
	if ShareFeedback::Err == Share::sync(tx, fb, Ok(Share::Source(can)), true) {
		// The status isn't actually OK, but errors are already known, so this
		// prevents resubmitting the same error later.
		return Ok(());
	}

	encoders.iter().for_each(|&e| {
		Share::sync(tx, fb, Ok(Share::Encoder(e)), false);
		if let Ok(mut guide) = EncodeIter::new(&src, e, flags) {
			let mut count: u8 = 0;
			while let Some(can) = guide.advance().and_then(|out| Candidate::try_from(out).ok()) {
				count += 1;
				let res = Share::sync(tx, fb, Ok(Share::Candidate(can.with_count(count))), true);
				match res {
					ShareFeedback::Keep => { guide.keep(); },
					ShareFeedback::Discard => { guide.discard(); },
					_ => {},
				}
			}

			// Save the best, if any!
			Share::sync(tx, fb, guide.take().map(|x| Share::Best(path.to_path_buf(), x)), true);
		}
	});

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
where W: gtk::WidgetExt {
	let style_context = widget.get_style_context();
	style_context.add_class(class);
}

/// # Remove Widget Class.
///
/// This removes a class from a widget.
fn remove_widget_class<W>(widget: &W, class: &str)
where W: gtk::WidgetExt {
	let style_context = widget.get_style_context();
	style_context.remove_class(class);
}

/// # Set Widget Style.
///
/// This adds a CSS resource (mini stylesheet) to the given widget.
fn set_widget_style<W>(widget: &W, src: &str)
where W: gtk::WidgetExt {
	let style_context = widget.get_style_context();
	let provider = gtk::CssProvider::new();
	provider.load_from_resource(src);
	style_context.add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
