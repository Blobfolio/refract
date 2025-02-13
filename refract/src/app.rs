/*!
# Refract: App
*/

use argyle::Argument;
use crate::{
	border_style,
	button_style,
	Candidate,
	DARK_PALETTE,
	DARK_THEME,
	FONT_BOLD,
	LIGHT_PALETTE,
	LIGHT_THEME,
	NiceColors,
	tooltip_style,
};
use dactyl::{
	NiceFloat,
	NiceU64,
	traits::IntDivFloat,
};
use dowser::Dowser;
use iced::{
	alignment::{
		Horizontal,
		Vertical,
	},
	Background,
	Color,
	ContentFit,
	Element,
	Fill,
	keyboard::{
		Key,
		key::Named,
		Modifiers,
	},
	Padding,
	Shrink,
	Subscription,
	Task,
	Theme,
	widget::{
		button,
		checkbox,
		column,
		Column,
		container,
		Container,
		container::bordered_box,
		image,
		rich_text,
		row,
		Row,
		scrollable,
		span,
		Stack,
		text,
		text::Rich,
		tooltip,
		toggler,
	},
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
	QualityValueFmt,
	RefractError,
};
use rfd::AsyncFileDialog;
use std::{
	borrow::Cow,
	collections::BTreeSet,
	ffi::OsStr,
	num::NonZeroUsize,
	path::{
		Path,
		PathBuf,
	},
	time::Duration,
};
use utc2k::FmtUtc2k;



/// # Format: AVIF.
const FMT_AVIF: u16 =         0b0000_0000_0001;

/// # Format: JPEG XL.
const FMT_JXL: u16 =          0b0000_0000_0010;

/// # Format: WebP.
const FMT_WEBP: u16 =         0b0000_0000_0100;

/// # Mode: Lossless.
const MODE_LOSSLESS: u16 =    0b0000_0000_1000;

/// # Mode: Lossy.
const MODE_LOSSY: u16 =       0b0000_0001_0000;

/// # Mode: Lossy + YCBCR.
///
/// This only applies for AVIF conversions.
const MODE_LOSSY_YCBCR: u16 = 0b0000_0010_0000;

/// # Show B (Candidate) Image.
const OTHER_BSIDE: u16 =      0b0000_0100_0000;

/// # Exit After.
const OTHER_EXIT_AUTO: u16 =  0b0000_1000_0000;

/// # Night Mode.
const OTHER_NIGHT: u16 =      0b0001_0000_0000;

/// # Save w/o Prompt.
const OTHER_SAVE_AUTO: u16 =  0b0010_0000_0000;

/// # New Encoder.
const SWITCHED_ENCODER: u16 = 0b0100_0000_0000;

/// # All Formats.
const FMT_FLAGS: u16 =
	FMT_AVIF | FMT_JXL | FMT_WEBP;

/// # All Modes.
const MODE_FLAGS: u16 =
	MODE_LOSSLESS | MODE_LOSSY;

/// # Default Flags.
const DEFAULT_FLAGS: u16 =
	FMT_FLAGS | MODE_FLAGS | MODE_LOSSY_YCBCR;

/// # Check Size.
const CHK_SIZE: u16 = 12_u16;

/// # Button Padding.
const BTN_PADDING: Padding = Padding {
	top: 10.0,
	right: 20.0,
	bottom: 10.0,
	left: 20.0,
};



/// # Helper: Refract Button.
macro_rules! btn {
	($label:literal, $color:ident) => (btn!($label, $color, BTN_PADDING));
	($label:literal, $color:ident, $pad:expr) => (
		button(text($label).size(18).font(FONT_BOLD))
			.style(|_, status| button_style(status, NiceColors::$color))
			.padding($pad)
	);
}

/// # Helper: Colorize and Embolden.
macro_rules! emphasize {
	($el:expr) => ($el.font(FONT_BOLD));
	($el:expr, $color:ident) => (emphasize!($el, NiceColors::$color));
	($el:expr, $color:expr) => ($el.color($color).font(FONT_BOLD));
}

/// # Helper: Image Kind.
macro_rules! kind {
	($kind:expr, $color:ident) => (kind!($kind, NiceColors::$color));
	($kind:expr, $color:expr) => (emphasize!(span($kind.as_str()), $color));
}



/// # Application.
///
/// This struct serves as a sort of universal state for `refract`. The
/// settings, tasks, logs, etc., are all kept here, as are the view/update
/// models required by `iced`.
pub(super) struct App {
	/// # Flags.
	flags: u16,

	/// # Paths (Queue).
	paths: BTreeSet<PathBuf>,

	/// # Current Job.
	current: Option<CurrentImage>,

	/// # Last Directory.
	///
	/// This holds the last directory — or at least one of them — that an
	/// image was enqueued from, for the sole purpose of being able to set it
	/// as the starting point should the user decide to later add more images.
	last_dir: Option<PathBuf>,

	/// # Activity Log.
	///
	/// This holds the image sources that have been loaded, along with any
	/// conversion results associated with them.
	done: Vec<ImageResults>,

	/// # (Last) Error.
	///
	/// This is used to clarify awkward situations resulting in nothing
	/// happening, such as after a user adds a directory that doesn't have any
	/// images in it.
	error: Option<MessageError>,

	/// # Widget Cache.
	cache: WidgetCache,
}

impl App {
	/// # New.
	///
	/// Parse the CLI arguments (if any) and return a new instance, unless
	/// `--help` or `--version` were requested instead, in which case the
	/// corresponding "error" is returned.
	pub(super) fn new() -> Result<Self, RefractError> {
		let mut paths = Dowser::default();
		let mut flags = DEFAULT_FLAGS;

		// Load CLI arguments, if any.
		let args = argyle::args()
			.with_keywords(include!(concat!(env!("OUT_DIR"), "/argyle.rs")));
		for arg in args {
			match arg {
				Argument::Key("-e" | "--exit-auto") => { flags |= OTHER_EXIT_AUTO; },
				Argument::Key("-h" | "--help") => return Err(RefractError::PrintHelp),
				Argument::Key("--no-avif") => { flags &= ! FMT_AVIF; },
				Argument::Key("--no-jxl") => { flags &= ! FMT_JXL; },
				Argument::Key("--no-webp") => { flags &= ! FMT_WEBP; },
				Argument::Key("--no-lossless") => { flags &= ! MODE_LOSSLESS; },
				Argument::Key("--no-lossy") => { flags &= ! MODE_LOSSY; },
				Argument::Key("--no-ycbcr") => { flags &= ! MODE_LOSSY_YCBCR; },
				Argument::Key("-s" | "--save-auto") => { flags |= OTHER_SAVE_AUTO; },
				Argument::Key("-V" | "--version") => return Err(RefractError::PrintVersion),

				Argument::KeyWithValue("-l" | "--list", s) => {
					let _res = paths.read_paths_from_file(s);
				},

				// Assume paths.
				Argument::Other(s) => { paths = paths.with_path(s); },
				Argument::InvalidUtf8(s) => { paths = paths.with_path(s); },

				// Nothing else is relevant.
				_ => {},
			}
		}

		// If the format or mode flags got completely unset, flip all bits
		// back on to teach the user a lesson! Haha.
		if 0 == flags & FMT_FLAGS { flags |= FMT_FLAGS; }
		if 0 == flags & MODE_FLAGS { flags |= MODE_FLAGS; }

		// We're almost done.
		let mut out = Self {
			flags,
			paths: BTreeSet::new(),
			current: None,
			last_dir: None,
			done: Vec::new(),
			error: None,
			cache: WidgetCache::default(),
		};

		// Digest the paths, if any.
		out.add_paths(paths);

		// Done!
		Ok(out)
	}
}

/// # Getters.
impl App {
	/// # Has Flag?
	///
	/// Returns true a given flag is set.
	const fn has_flag(&self, flag: u16) -> bool { flag == self.flags & flag }

	/// # Interactive?
	///
	/// Returns true if only crunching losslessly and auto-save is enabled.
	const fn automatic(&self) -> bool {
		! self.has_flag(MODE_LOSSY) && self.has_flag(OTHER_SAVE_AUTO)
	}

	/// # Count Encoders.
	const fn count_encoders(&self) -> u8 {
		(self.has_flag(FMT_AVIF) as u8) +
		(self.has_flag(FMT_JXL) as u8) +
		(self.has_flag(FMT_WEBP) as u8)
	}

	/// # Has Candidate?
	fn has_candidate(&self) -> bool {
		self.current.as_ref().is_some_and(CurrentImage::has_candidate)
	}

	/// # Theme.
	///
	/// Returns the current theme, i.e. light or dark.
	pub(super) fn theme(&self) -> Theme {
		if self.has_flag(OTHER_NIGHT) { DARK_THEME.clone() }
		else { LIGHT_THEME.clone() }
	}
}

/// # Setters.
impl App {
	/// # Digest Paths.
	///
	/// Traverse the provided paths, adding any `jpeg` or `png` files to
	/// the queue for later crunching.
	///
	/// This method will also set `last_dir` to the parent directory of the
	/// first such file, if any.
	fn add_paths(&mut self, paths: Dowser) {
		let mut paths = paths.filter(|p| crate::is_jpeg_png(p));

		// Grab the first path manually so we can note its parent directory
		// (for any subsequent file browsing needs).
		let Some(first) = paths.next() else { return; };
		if let Some(dir) = first.parent() {
			if self.last_dir.as_ref().is_none_or(|old| old != dir) {
				self.last_dir.replace(dir.to_path_buf());
			}
		}

		// Add the first and the rest.
		self.paths.insert(first);
		self.paths.extend(paths);
	}

	/// # Current Foreground Color.
	const fn fg(&self) -> Color {
		if self.has_flag(OTHER_NIGHT) { DARK_PALETTE.text }
		else { LIGHT_PALETTE.text }
	}

	/// # Toggle Flag.
	///
	/// Flip the bits corresponding to a given flag, except in cases where
	/// that would leave us without any formats or modes, in which case _all_
	/// formats or modes (respectively) will be flipped back _on_.
	fn toggle_flag(&mut self, flag: u16) {
		self.flags ^= flag;

		// Same as `new`, we need to make sure the format and mode flags aren't
		// totally unset as that would be silly.
		if 0 == self.flags & FMT_FLAGS { self.flags |= FMT_FLAGS; }
		if 0 == self.flags & MODE_FLAGS { self.flags |= MODE_FLAGS; }
	}
}

/// # Iced Controls.
impl App {
	/// # First Task.
	///
	/// This does nothing, unless paths happened to be added via CLI, in which
	/// case it lets `iced` know it should jump straight into conversion.
	pub(super) fn start(&self) -> Task<Message> {
		if self.paths.is_empty() { Task::none() }
		else { Task::done(Message::NextImage) }
	}

	#[expect(clippy::too_many_lines, reason = "There's a lot to update. Haha.")]
	/// # Update.
	///
	/// This method serves as the entrypoint for the application's
	/// "reactivity". Anytime a user checks a box or clicks a button, a
	/// `Message` gets generated and sent here.
	///
	/// Depending on the nature of the work, this method might "recurse" in a
	/// roundabout way by returning a new `Message` that will make its way
	/// back to itself.
	pub(super) fn update(&mut self, message: Message) -> Task<Message> {
		// Clear the last error, if any.
		let _res = self.error.take();

		match message {
			// Add File(s) or Directory.
			Message::AddPaths(paths) => {
				self.add_paths(paths);

				// If none of the path(s) were valid, record the "error"
				// so we can explain why nothing is happening.
				if self.paths.is_empty() {
					return Task::done(Message::Error(MessageError::NoImages));
				}
				// Otherwise we probably want to load up the first image,
				// but only if we aren't already processing something else.
				else if self.current.is_none() {
					return Task::done(Message::NextImage);
				}
			},

			// Record an "error" message so we can let the user know what's up.
			Message::Error(err) => {
				self.error.replace(err);
				cli_log_error(err);
			},

			// Process the user's yay/nay evaluation of a candidate image.
			Message::Feedback(feedback) => if let Some(current) = &mut self.current {
				if current.candidate.is_some() {
					self.flags &= ! OTHER_BSIDE;
					// Back around again!
					if current.feedback(feedback) {
						return Task::done(Message::NextStep);
					}
				}
			},

			// Switch to the next encoder.
			Message::NextEncoder =>
				if self.current.as_mut().is_some_and(CurrentImage::next_encoder) {
					return self.update_switch_encoder__();
				}
				// This image is done; move onto the next!
				else { return Task::done(Message::NextImage); },

			// If there are images in the queue, pull the first and start up
			// the conversion process for it.
			Message::NextImage => {
				self.flags &= ! OTHER_BSIDE;
				self.current = None;
				while let Some(src) = self.paths.pop_first() {
					if let Some(mut current) = CurrentImage::new(src.clone(), self.flags) {
						// Add an entry for it.
						cli_log(&current.src, None);
						self.done.push(ImageResults {
							src: current.src.clone(),
							src_kind: current.input.kind(),
							src_len: NonZeroUsize::new(current.input.size()).unwrap(),
							dst: Vec::new(),
						});

						// Make sure the encoder can be set before accepting
						// the result.
						if current.next_encoder() {
							self.current = Some(current);
							return self.update_switch_encoder__();
						}
					}
					// Decode error?
					else {
						cli_log_sad(&src);
						if self.paths.is_empty() {
							return Task::done(Message::Error(MessageError::NoImages));
						}
					}
				}

				// If we're here, there are no more images. If --exit-auto,
				// that means quittin' time!
				if self.has_flag(OTHER_EXIT_AUTO) { return iced::exit(); }
			},

			// Spawn a thread to get the next candidate image crunching or, if
			// there is none, save the best and move on.
			Message::NextStep => {
				self.flags &= ! OTHER_BSIDE;
				let confirm = ! self.has_flag(OTHER_SAVE_AUTO);
				if let Some(current) = &mut self.current {
					// Advance iterator and wait for feedback.
					if let Some(m) = current.next_candidate() { return m; }

					// Log and save the results, if any.
					if let Some(res) = current.finish_encoder() {
						if confirm { return res.open_fd(); }
						return Task::done(Message::SaveImage(res));
					}

					// Advance the encoder.
					return Task::done(Message::NextEncoder);
				}
				// This image is done; move onto the next!
				return Task::done(Message::NextImage);
			},

			// Reabsorb the encoder (stolen above) and either display the
			// candidate for feedback or, if none, save the best and move on.
			Message::NextStepDone(wrapper) => {
				let confirm = ! self.has_flag(OTHER_SAVE_AUTO);
				if let Some(current) = &mut self.current {
					// Advance iterator and wait for feedback.
					if current.next_candidate_done(wrapper) {
						self.flags |= OTHER_BSIDE;
						return Task::none();
					}

					// Log and save the results, if any.
					if let Some(res) = current.finish_encoder() {
						if confirm { return res.open_fd(); }
						return Task::done(Message::SaveImage(res));
					}

					// Advance the encoder.
					return Task::done(Message::NextEncoder);
				}
				// This image is done; move onto the next!
				return Task::done(Message::NextImage);
			},

			// Save the image and continue.
			Message::SaveImage(mut wrapper) =>
				if self.current.is_some() {
					// Actually save the image, if any.
					wrapper.save();

					// Log the results.
					if let Some(last) = self.done.last_mut() {
						if last.src == wrapper.src {
							last.dst.push(wrapper.into_result());
						}
					}

					// Advance the encoder.
					return Task::done(Message::NextEncoder);
				}
				// This image is done; move onto the next!
				else { return Task::done(Message::NextImage); },

			// Open File/Dir Dialogue.
			Message::OpenFd(dir) => return self.open_fd(dir),

			// Open a local image path using whatever (external) program the
			// desktop environment would normally use to open that file type.
			Message::OpenFile(src) => if open::that_detached(src).is_err() {
				return Task::done(Message::Error(MessageError::NoOpen));
			},

			// Open a URL in e.g. the system's default web browser.
			Message::OpenUrl(url) => if open::that_detached(url).is_err() {
				return Task::done(Message::Error(MessageError::NoOpen));
			},

			// Toggle a flag.
			Message::ToggleFlag(flag) => { self.toggle_flag(flag); },

			// Unset a flag.
			Message::UnsetFlag(flag) => { self.flags &= ! flag; },
		}

		Task::none()
	}

	/// # Update Helper: Switch Encoder.
	///
	/// Toggle the `SWITCHED_ENCODER` flag and return a chain of messages to
	/// get it up and running.
	///
	/// Note: this does not actually _select_ the encoder; that has to be
	/// handled beforehand by the caller as there is some contextual nuance.
	fn update_switch_encoder__(&mut self) -> Task<Message> {
		// If the user isn't involved or is only using one encoder, there's
		// no need to promote the change.
		if self.automatic() || self.count_encoders() < 2 {
			Task::done(Message::NextStep)
		}
		// Otherwise let's give them a quick heads up!
		else {
			self.flags |= SWITCHED_ENCODER;
			Task::future(async {
				async_std::task::sleep(Duration::from_millis(1500)).await;
				Message::UnsetFlag(SWITCHED_ENCODER)
			})
				.chain(Task::done(Message::NextStep))
		}
	}

	/// # Subscription.
	///
	/// This method sets up listeners for the program's keyboard shortcuts,
	/// bubbling up `Message`s as needed.
	pub(super) fn subscription(&self) -> Subscription<Message> {
		if self.has_candidate() { iced::keyboard::on_key_press(subscribe_ab) }
		else { iced::keyboard::on_key_press(subscribe_home) }
	}

	/// # View.
	///
	/// This method returns everything `iced` needs to draw the screen.
	///
	/// Under the hood, this defers to either `view_home`, `view_encoder`, or
	/// `view_ab` depending on the state of things.
	pub(super) fn view(&self) -> Container<Message> {
		// If we're processing an image, return the A/B screen.
		if self.current.as_ref().is_some_and(CurrentImage::active) {
			// Unless we _just_ switched encoders, in which case we should
			// announce it real quick.
			if self.has_flag(SWITCHED_ENCODER) {
				if let Some(kind) = self.current.as_ref().and_then(CurrentImage::output_kind) {
					return self.view_encoder(kind);
				}
			}
			self.view_ab()
		}
		// Otherwise the home screen.
		else { self.view_home() }
	}
}

/// # View: Normal.
impl App {
	/// # View: Normal.
	///
	/// This screen is shown when nothing else is going on. It comprises the
	/// main program settings, file/directory open buttons, version details,
	/// and either an activity log or decorative `refract` logo.
	///
	/// (A warning/error message may also be presented, but they're short-
	/// lived and uncommon.)
	fn view_home(&self) -> Container<Message> {
		container(
			column!(
				self.view_settings(),
				self.view_log(),
			)
				.push_maybe(self.view_error())
				.spacing(10)
		)
			.padding(10)
			.width(Fill)
	}

	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # View: About.
	///
	/// This returns the application name, version, and repository URL.
	fn view_about(&self) -> Column<Message> {
		column!(
			rich_text!(
				emphasize!(span("Refract "), PINK),
				emphasize!(span(concat!("v", env!("CARGO_PKG_VERSION"))), PURPLE),
			),

			rich_text!(
				emphasize!(span(env!("CARGO_PKG_REPOSITORY")), GREEN)
					.link(Message::OpenUrl(env!("CARGO_PKG_REPOSITORY")))
			),
		)
			.align_x(Horizontal::Right)
			.spacing(5)
			.width(Shrink)
	}

	/// # View: Last Error.
	///
	/// If the user did something that did nothing instead of something, this
	/// returns a message explaining why they got nothing instead of something,
	/// lest they think it's our fault!
	fn view_error(&self) -> Option<Container<Message>> {
		use iced::widget::container::Style;

		self.error.map(|err|
			container(row!(
				rich_text!(
					emphasize!(span("Warning: ")),
					span(err.as_str()),
				)
					.width(Shrink)
			))
				.padding(10.0)
				.center(Fill)
				.height(Shrink)
				.style(|_| Style {
					text_color: Some(NiceColors::WHITE),
					background: Some(Background::Color(NiceColors::ORANGE)),
					..Style::default()
				})
		)
	}

	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # View: Enqueue Buttons.
	///
	/// This returns button widgets for adding file(s) or directories, and
	/// some basic instructions for same.
	fn view_enqueue_buttons(&self) -> Container<Message> {
		container(
			column!(
				row!(
					btn!("File(s)", PURPLE).on_press(Message::OpenFd(false)),
					text("or").size(18),
					btn!("Directory", PINK).on_press(Message::OpenFd(true)),
				)
					.align_y(Vertical::Center)
					.spacing(10)
					.width(Shrink),

				rich_text!(
					span("Choose one or more "),
					emphasize!(span("JPEG")),
					span("/"),
					emphasize!(span("PNG")),
					span(" images."),
				),
			)
				.align_x(Horizontal::Center)
				.spacing(10)
		)
			.center_x(Fill)
			.width(Fill)
	}

	/// # View: Activity Log.
	///
	/// This returns a table containing detailed information about each of the
	/// source images and next-gen conversions that have been processed,
	/// successfully or otherwise.
	fn view_log(&self) -> Element<'_, Message> {
		use unicode_width::UnicodeWidthStr;

		// If there's no activity, display our logo instead.
		if self.done.is_empty() { return self.view_logo(); }

		// Reformat the data.
		let table = ActivityTable::from(self.done.as_slice());

		// Figure out some bounds.
		let widths = table.widths();
		let total_width = widths.iter().copied().sum::<usize>() + 5 * 3;
		let divider = "-".repeat(total_width);

		// Finally, add all the lines!
		let mut lines = column!(rich_text!(
			emphasize!(span(format!("{:<w$}", ActivityTable::HEADERS[0], w=widths[0])), PURPLE),
			span(" | ").color(NiceColors::PINK),
			emphasize!(span(format!("{:<w$}", ActivityTable::HEADERS[1], w=widths[1])), PURPLE),
			span(" | ").color(NiceColors::PINK),
			emphasize!(span(format!("{:>w$}", ActivityTable::HEADERS[2], w=widths[2])), PURPLE),
			span(" | ").color(NiceColors::PINK),
			emphasize!(span(format!("{:>w$}", ActivityTable::HEADERS[3], w=widths[3])), PURPLE),
			span(" | ").color(NiceColors::PINK),
			emphasize!(span(format!("{:>w$}", ActivityTable::HEADERS[4], w=widths[4])), PURPLE),
			span(" | ").color(NiceColors::PINK),
			emphasize!(span(format!("{:>w$}", ActivityTable::HEADERS[5], w=widths[5])), PURPLE),
		));

		// The rows, interspersed with dividers for each new source.
		let mut last_dir = OsStr::new("");
		for ActivityTableRow { src, kind, quality, len, ratio, time } in &table.0 {
			let Some((dir, file)) = split_path(src) else { continue; };
			let is_src = matches!(kind, ImageKind::Png | ImageKind::Jpeg);
			let skipped = is_src && time.is_some();
			let color =
				if is_src {
					if skipped { NiceColors::RED } else { self.fg() }
				}
				else if len.is_some() { NiceColors::GREEN }
				else { NiceColors::RED };

			if is_src {
				last_dir = OsStr::new("");
				lines = lines.push(text(divider.clone()).color(NiceColors::PINK));
			}

			lines = lines.push(rich_text!(
				// Path, pretty-formatted.
				span(format!("{}/", dir.to_string_lossy()))
					.color(
						if dir == last_dir { NiceColors::TRANSPARENT }
						else { NiceColors::GREY }
					),
				span(file.to_string_lossy().into_owned())
					.color(color)
					.link_maybe((len.is_some() && src.is_file()).then(|| Message::OpenFile(src.to_path_buf()))),
				span(format!(
					"{:<w$} | ",
					"",
					w=widths[0].saturating_sub(src.to_string_lossy().width())
				))
					.color(NiceColors::PINK),

				// Kind.
				span(format!("{kind:<w$}", w=widths[1])),
				span(" | ").color(NiceColors::PINK),

				// Quality.
				span(format!("{:>w$}", quality.as_str(), w=widths[2])),
				span(" | ").color(NiceColors::PINK),

				// Size.
				span(format!("{:>w$}", len.as_ref().map_or("", NiceU64::as_str), w=widths[3])),
				span(" | ").color(NiceColors::PINK),

				// Ratio.
				ratio.as_ref().map_or_else(
					|| span(" ".repeat(widths[4])),
					|n| {
						let nice = n.precise_str(4);
						span(format!("{nice:>w$}", w=widths[4]))
							.color_maybe((nice == "1.0000").then_some(NiceColors::GREY))
					},
				),
				span(" | ").color(NiceColors::PINK),

				// Time.
				time.as_ref().map_or_else(
					|| span(""),
					|n|
						if skipped {
							span(format!("{:>w$}", "skipped", w=widths[5]))
								.color(NiceColors::RED)
						}
						else {
							let nice = n.precise_str(3);
							span(format!("{nice:>w$}s", w=widths[5] - 1))
								.color_maybe((nice == "0.000").then_some(NiceColors::GREY))
						},
				),
			));

			// Update the last directory before leaving.
			last_dir = dir;
		}

		// Add footnotes.
		lines = lines.push(text(divider).color(NiceColors::PINK))
			.push(text(""))
			.push(text(""))
			.push(rich_text!(
				span(" *").color(NiceColors::PURPLE),
				span(" Compression ratio is ").color(NiceColors::GREY),
				emphasize!(span("src"), PURPLE),
				emphasize!(span(":"), GREY),
				emphasize!(span("dst"), PINK),
				span(".").color(NiceColors::GREY),
			))
			.push(rich_text!(
				span("**").color(NiceColors::PURPLE),
				span(" Total encoding time, rejects and all.").color(NiceColors::GREY),
			));

		scrollable(container(lines).width(Fill).padding(10))
			.height(Fill)
			.anchor_bottom()
			.into()
	}

	/// # View Logo.
	///
	/// This returns a simple program logo to fill the whitespace that would
	/// otherwise exist at startup owing to the lack of history to report.
	fn view_logo(&self) -> Element<'_, Message> {
		container(image(self.cache.logo.clone())).center(Fill).into()
	}

	/// # View: Settings.
	///
	/// This collects and returns the contents of the `view_settings_*`
	/// helpers, along with the add-file buttons and about information.
	fn view_settings(&self) -> Container<Message> {
		container(
			row!(
				self.view_settings_fmt(),
				self.view_settings_mode(),
				self.view_settings_other(),
				self.view_enqueue_buttons(),
				self.view_about(),
			)
				.padding(20)
				.spacing(20)
		)
			.style(|_| {
				let mut style = bordered_box(&self.theme());
				let _res = style.background.take();
				style
			})
			.width(Fill)
	}

	/// # View: Format Checkboxes.
	///
	/// This returns a list of checkboxes corresponding to the available
	/// next-gen image formats (the encoders that will be used).
	fn view_settings_fmt(&self) -> Column<Message> {
		column!(
			emphasize!(text("Formats"), PINK),
			checkbox("AVIF", self.has_flag(FMT_AVIF))
				.on_toggle(|_| Message::ToggleFlag(FMT_AVIF))
				.size(CHK_SIZE),
			checkbox("JPEG XL", self.has_flag(FMT_JXL))
				.on_toggle(|_| Message::ToggleFlag(FMT_JXL))
				.size(CHK_SIZE),
			checkbox("WebP", self.has_flag(FMT_WEBP))
				.on_toggle(|_| Message::ToggleFlag(FMT_WEBP))
				.size(CHK_SIZE),
		)
			.spacing(5)
	}

	/// # View: Mode Checkboxes.
	///
	/// This returns checkboxes for the various compression modes.
	fn view_settings_mode(&self) -> Column<Message> {
		column!(
			emphasize!(text("Compression"), PINK),
			checkbox("Lossless", self.has_flag(MODE_LOSSLESS))
				.on_toggle(|_| Message::ToggleFlag(MODE_LOSSLESS))
				.size(CHK_SIZE),
			checkbox("Lossy", self.has_flag(MODE_LOSSY))
				.on_toggle(|_| Message::ToggleFlag(MODE_LOSSY))
				.size(CHK_SIZE),
			tooltip(
				checkbox("Lossy YCbCr", self.has_flag(MODE_LOSSY_YCBCR))
					.on_toggle_maybe(self.has_flag(FMT_AVIF | MODE_LOSSY).then_some(|_| Message::ToggleFlag(MODE_LOSSY_YCBCR)))
					.size(CHK_SIZE),
				container(
					text("Repeat AVIF A/B tests in YCbCr colorspace to look for additional savings.")
						.size(12)
				)
					.padding(20)
					.max_width(300_u16)
					.style(|_| tooltip_style(! self.has_flag(OTHER_NIGHT))),
				tooltip::Position::Bottom,
			),
		)
			.spacing(5)
	}

	/// # View: Other Checkboxes.
	///
	/// This returns checkboxes for the program's one-off settings, i.e.
	/// night mode and automatic saving.
	fn view_settings_other(&self) -> Column<Message> {
		macro_rules! tip {
			($label:literal, $flag:ident, $help:literal) => (
				tooltip(
					checkbox($label, self.has_flag($flag))
						.on_toggle(|_| Message::ToggleFlag($flag))
						.size(CHK_SIZE),
					container(text($help).size(12))
						.padding(20)
						.max_width(300_u16)
						.style(|_| tooltip_style(! self.has_flag(OTHER_NIGHT))),
					tooltip::Position::Bottom,
				)
			);
		}

		column!(
			emphasize!(text("Other"), PINK),
			tip!(
				"Auto-Save", OTHER_SAVE_AUTO,
				"Automatically save successful conversions to their source paths — with new extensions appended — instead of popping file dialogues for confirmation."
			),
			tip!(
				"Auto-Exit", OTHER_SAVE_AUTO,
				"Close the program after the last image has been processed."
			),
			checkbox("Night Mode", self.has_flag(OTHER_NIGHT))
				.on_toggle(|_| Message::ToggleFlag(OTHER_NIGHT))
				.size(CHK_SIZE),
		)
			.spacing(5)
	}
}

/// # View: Encoder.
impl App {
	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # View: Encoder.
	///
	/// The constant format changes can get confusing. This screen is used to
	/// (very briefly) announce the changes.
	fn view_encoder(&self, kind: ImageKind) -> Container<Message> {
		use iced::widget::container::Style;

		container(
			column!(
				emphasize!(text("Up Next…").size(18)),
				match kind {
					ImageKind::Avif => rich_text!(
						emphasize!(span("A"), PURPLE),
						emphasize!(span("v"), TEAL),
						emphasize!(span("i"), BLUE),
						emphasize!(span("f"), YELLOW),
					),
					ImageKind::Jxl => rich_text!(
						emphasize!(span("J"), BLUE),
						emphasize!(span("P"), GREEN),
						emphasize!(span("E"), PINK),
						emphasize!(span("G"), YELLOW),
						emphasize!(span("X"), PURPLE),
						emphasize!(span("L"), RED),
					),
					ImageKind::Webp => rich_text!(
						emphasize!(span("W"), RED),
						emphasize!(span("e"), PURPLE),
						emphasize!(span("b"), BLUE),
						emphasize!(span("P"), GREEN),
					),
					_ => rich_text!(emphasize!(span("???"))),
				}
					.size(72),
			)
				.align_x(Horizontal::Left)
				.width(Shrink)
		)
			.padding(20)
			.center(Fill)
			.style(move |_| Style {
				text_color: Some(NiceColors::WHITE),
				background: Some(Background::Color(NiceColors::BLACK)),
				..Style::default()
			})
	}
}

/// # View: Images.
impl App {
	/// # View: Images.
	///
	/// This screen is shown when an image is being processed, whether
	/// actively or awaiting user feedback.
	///
	/// It comprises a title-like bar, image stack, and footer with
	/// instructions, progress, and action buttons.
	fn view_ab(&self) -> Container<Message> {
		container(
			column!(
				self.view_ab_header(),
				self.view_image(),
				container(
					row!(
						self.view_keyboard_shortcuts(),
						self.view_ab_progress(),
						self.view_ab_feedback(),
					)
						.align_y(Vertical::Center)
						.padding(20)
						.spacing(20)
				)
					.width(Fill)
			)
		)
			.width(Fill)
	}

	/// # View: Image A/B Feedback Buttons.
	///
	/// This returns the "Accept" and "Reject" buttons used for candidate image
	/// feedback, though they'll only be enabled if the program is ready to
	/// receive said feedback.
	fn view_ab_feedback(&self) -> Column<Message> {
		let Some(current) = &self.current else { return Column::new(); };
		let active = current.candidate.is_some();
		let b_side = active && self.has_flag(OTHER_BSIDE);
		let src_kind = current.input.kind();
		let dst_kind = current.output_kind().unwrap_or(src_kind);

		column!(
			// Buttons.
			row!(
				btn!("Reject", RED)
					.on_press_maybe(active.then_some(Message::Feedback(false))),

				btn!("Accept", GREEN)
					.on_press_maybe(active.then_some(Message::Feedback(true))),

				tooltip(
					btn!("?", GREY, Padding {
						top: 10.0,
						right: 15.0,
						bottom: 10.0,
						left: 15.0,
					}),
					container(
						rich_text!(
							span("Forget about images past. Are you happy with "),
							span("this").underline(true),
							span(" one? If yes, "),
							emphasize!(span("accept"), GREEN),
							span(" it. The best of the best will be saved at the very end."),
						)
							.size(12)
					)
						.padding(20)
						.max_width(300_u16)
						.style(|_| tooltip_style(! self.has_flag(OTHER_NIGHT))),
					tooltip::Position::Top,
				)
			)
				.width(Shrink)
				.align_y(Vertical::Center)
				.spacing(10),

			// A/B toggle.
			row!(
				rich_text!(
					kind!(
						src_kind,
						if b_side { NiceColors::GREY } else { NiceColors::PURPLE }
					)
						.link_maybe(b_side.then_some(Message::ToggleFlag(OTHER_BSIDE)))
				),

				toggler(b_side)
					.spacing(0)
					.on_toggle_maybe(active.then_some(|_| Message::ToggleFlag(OTHER_BSIDE))),

				rich_text!(
					kind!(
						dst_kind,
						if b_side { NiceColors::PINK } else { NiceColors::GREY }
					)
						.link_maybe((active && ! b_side).then_some(Message::ToggleFlag(OTHER_BSIDE)))
				),
			)
				.spacing(5)
				.align_y(Vertical::Center)
		)
			.spacing(10)
			.align_x(Horizontal::Center)
	}

	/// # View: Image Header.
	///
	/// This returns one of three possible pseudo-titlebars for use during
	/// image processing:
	///
	/// 1. In A/B mode, it contains the format and quality details for the image actively being displayed, i.e. the source or candidate.
	/// 2. In lossless-only mode, it lets the user know that no feedback will be required.
	/// 3. Otherwise a generic "reticulating splines" message, since there's nothing to do but wait.
	fn view_ab_header(&self) -> Container<Message> {
		use iced::widget::container::Style;

		let mut row = Row::new()
			.spacing(20)
			.align_y(Vertical::Center)
			.width(Shrink);

		let Some(current) = self.current.as_ref() else { return container(row); };
		let mut color = NiceColors::PURPLE;

		// If there's no candidate, print a stock message.
		if current.candidate.is_none() {
			color = NiceColors::BLUE;
			// Lossless/auto requires no feedback, so let's give a different
			// message.
			if self.automatic() {
				row = row.push(emphasize!(text(
					"Lossless conversion is automatic. Just sit back and wait!"
				)));
			}
			else if let Some(kind) = current.output_kind() {
				row = row.push(emphasize!(text(format!("Preparing the next {kind}; sit tight!"))));
			}
			else {
				row = row.push(emphasize!(text("Reticulating splines…")));
			}
		}
		else {
			let mut quality = None;
			let mut kind = current.input.kind();
			let mut count = 0;

			// Pull the candidate info if we're looking at that.
			if self.has_flag(OTHER_BSIDE) {
				if let Some(can) = current.candidate.as_ref() {
					kind = can.kind;
					count = can.count;
					color = NiceColors::PINK;
					quality.replace(can.quality);
				}
			}

			// Helper: key/value pair.
			macro_rules! kv {
				($k:expr, $v:expr) => (rich_text!(span($k), emphasize!(span($v))));
			}

			// Kind.
			row = row.push(kv!("Viewing: ", kind.as_str()));

			// Count.
			if count != 0 { row = row.push(kv!("Take: ", format!("#{count}"))); }

			// Quality.
			if let Some(quality) = quality {
				row = row.push(kv!(
					format!("{}: ", quality.label_title()),
					quality.quality().to_string()
				));
			}
			else { row = row.push(kv!("Quality: ", "Original")); }
		}

		container(row)
			.padding(10.0)
			.center(Fill)
			.height(Shrink)
			.style(move |_| Style {
				text_color: Some(NiceColors::WHITE),
				background: Some(Background::Color(color)),
				..Style::default()
			})
	}

	/// # View: Image Progress.
	///
	/// This returns some basic information about the current processing job,
	/// namely the source and target formats.
	///
	/// It also includes a checkbox to toggle night mode, since visually it
	/// fits better in this column than anywhere else.
	fn view_ab_progress(&self) -> Column<Message> {
		/// # Maybe Dim a Color.
		///
		/// Pass through the color if `cond`, otherwise dim it.
		const fn maybe_dim(color: Color, cond: bool) -> Color {
			if cond { color }
			else {
				Color { a: 0.5, ..color }
			}
		}

		let Some(current) = self.current.as_ref() else { return Column::new(); };

		let active = current.candidate.is_some();
		let new_kind = current.output_kind().unwrap_or(ImageKind::Png);

		let mut formats = Vec::with_capacity(5);
		for (flag, kind) in [
			(FMT_WEBP, ImageKind::Webp),
			(FMT_AVIF, ImageKind::Avif),
			(FMT_JXL, ImageKind::Jxl),
		] {
			if self.has_flag(flag) {
				if ! formats.is_empty() {
					formats.push(span(" + ").color(NiceColors::GREY));
				}
				if kind == new_kind { formats.push(kind!(kind, PINK)); }
				else { formats.push(kind!(kind, GREY)); }
			}
		}
		formats.insert(0, span(" > ").color(NiceColors::GREY));
		formats.insert(0, kind!(current.input.kind(), PURPLE));

		column!(
			// Path.
			split_path(&current.src).map_or_else(
				Rich::new,
				|(dir, name)| rich_text!(
					span(format!("{}/", dir.to_string_lossy())).color(NiceColors::GREY),
					span(name.to_string_lossy().into_owned()).color(self.fg()),
				),
			),

			// Formats.
			Rich::with_spans(formats),

			// Cancel.
			text(""),
			rich_text!(
				span("Ready for bed? ").color(maybe_dim(self.fg(), active)),
				emphasize!(
					span("Skip ahead!"),
					maybe_dim(NiceColors::ORANGE, active)
				)
					.link_maybe(active.then_some(Message::NextImage)),
			)
		)
			.spacing(5)
			.align_x(Horizontal::Center)
			.width(Fill)
	}

	/// # View: Image Stack.
	///
	/// This returns a fullscreen image — either the source or candidate,
	/// depending on A/B — overtop a static checked tile background (to make it
	/// easier to distinguish transparent regions).
	///
	/// The image itself is technically optional, but should always be present
	/// in practice.
	fn view_image(&self) -> Stack<Message> {
		Stack::with_capacity(3)
			.push(self.view_image_checkers_a())
			.push_maybe(self.view_image_checkers_b())
			.push_maybe(self.view_image_image())
			.width(Fill)
			.height(Fill)
	}

	/// # View: Image Checkers (A).
	///
	/// Produce a checkered background to make it easier to visualize image
	/// transparency.
	fn view_image_checkers_a(&self) -> Container<Message> {
		container(
			image(self.cache.checkers_a.clone())
			.content_fit(ContentFit::None)
			.width(Fill)
			.height(Fill)
		)
			.clip(true)
	}

	/// # View: Image Checkers (B).
	///
	/// This adds a "B" to every fourth square for added emphasis, but only
	/// when viewing a candidate image.
	fn view_image_checkers_b(&self) -> Option<Container<Message>> {
		if self.has_flag(OTHER_BSIDE) && self.has_candidate() {
			Some(
				container(
					image(self.cache.checkers_b.clone())
						.content_fit(ContentFit::None)
						.width(Fill)
						.height(Fill)
				)
					.clip(true)
			)
		}
		else { None }
	}

	#[expect(clippy::default_trait_access, reason = "Can't.")]
	/// # Image Layer.
	///
	/// Return a rendering of either the source image or candidate for
	/// display. When no candidate is available, the source image is returned
	/// in a semi-transparent state to help imply "loading".
	///
	/// This method is technically fallible, but in practice it should never
	/// not return something.
	fn view_image_image(&self) -> Option<Container<Message>> {
		use iced::widget::scrollable::{
			Direction,
			Rail,
			Scrollbar,
			Scroller,
			Style,
		};

		/// # Scroll paddle thingy.
		const RAIL: Rail = Rail {
			background: Some(Background::Color(NiceColors::YELLUCK)),
			border: border_style(NiceColors::TRANSPARENT, 0.0, 0.0),
			scroller: Scroller {
				color: NiceColors::YELLOW,
				border: border_style(NiceColors::BABYFOOD, 2.0, 0.0),
			},
		};

		let current = self.current.as_ref()?;
		let mut handle = None;

		// Show the new one?
		if self.has_flag(OTHER_BSIDE) {
			if let Some(can) = current.candidate.as_ref() {
				handle.replace(can.img.clone());
			}
		}

		// If we aren't showing the new one, show the old one.
		let handle = handle.unwrap_or_else(|| current.img.clone());

		Some(
			container(
				scrollable(
					image(handle)
						.content_fit(ContentFit::None)
						.width(Shrink)
						.height(Shrink)
						.opacity(if current.candidate.is_some() || self.automatic() { 1.0 } else { 0.5 })
				)
					.width(Shrink)
					.height(Shrink)
					.direction(Direction::Both { vertical: Scrollbar::new(), horizontal: Scrollbar::new() })
					.style(|_, _| Style {
						container: Default::default(),
						vertical_rail: RAIL,
						horizontal_rail: RAIL,
						gap: None,
					})
			)
				.width(Fill)
				.height(Fill)
				.center(Fill)
		)
	}

	/// # View: Image Screen Keyboard Shortcuts.
	///
	/// This returns a simple legend illustrating the available keyboard
	/// shortcuts that can be used in lieu of the button widgets.
	fn view_keyboard_shortcuts(&self) -> Column<Message> {
		let Some(current) = self.current.as_ref() else { return Column::new(); };
		let src_kind = current.input.kind();
		let dst_kind = current.output_kind().unwrap_or(src_kind);
		column!(
			rich_text!(
				emphasize!(span("   [space]")),
				span(" Toggle image view (").color(NiceColors::GREY),
				kind!(src_kind, PURPLE),
				span(" vs ").color(NiceColors::GREY),
				kind!(dst_kind, PINK),
				span(").").color(NiceColors::GREY),
			),
			rich_text!(
				emphasize!(span("       [d]"), RED),
				span(" Reject candidate.").color(NiceColors::GREY),
			),
			rich_text!(
				emphasize!(span("       [k]"), GREEN),
				span(" Accept candidate.").color(NiceColors::GREY),
			),
			rich_text!(
				emphasize!(span("[ctrl]")),
				span("+").color(NiceColors::GREY),
				emphasize!(span("[n]")),
				span(" Toggle night mode.").color(NiceColors::GREY),
			),
		)
			.spacing(5)
	}
}

/// # Other.
impl App {
	/// # Open File Dialogue.
	///
	/// Synchronous file dialogues have a habit of making GNOME think the
	/// program is "stuck", so this spawns one asynchronously so the user can
	/// take however long they want to make a selection.
	///
	/// If and when a selection is made, a separate `Message::AddPaths` task
	/// will be spawned to handle the details.
	fn open_fd(&self, dir: bool) -> Task<Message> {
		// Try to set a sane starting directory for ourselves.
		let mut fd = AsyncFileDialog::new();
		if let Some(p) = self.last_dir.as_ref() { fd = fd.set_directory(p); }
		else if let Ok(p) = std::env::current_dir() { fd = fd.set_directory(p); }

		// Directory version.
		if dir {
			return Task::future(async {
				fd.set_title("Choose Directory")
					.pick_folder()
					.await
					.map(|p| Task::done(
						Message::AddPaths(Dowser::from(p.path()))
					))
			}).and_then(|t| t);
		}

		// File version.
		Task::future(async {
			fd.add_filter("Images", &["jpg", "jpeg", "png"])
				.set_title("Choose Image(s)")
				.pick_files()
				.await
				.map(|paths| Task::done(
					Message::AddPaths(
						Dowser::default().with_paths(
							paths.iter().map(rfd::FileHandle::path)
						)
					)
				))
		}).and_then(|t| t)
	}
}



/// # Activity Table.
///
/// This is essentially an alternative view into the `ImageResults`, one more
/// suitable for display.
///
/// It holds the path, kind, quality, file size, and compression ratio for each
/// source and output, whether saved or not, though owing to the variety, most
/// fields are optional.
struct ActivityTable<'a>(Vec<ActivityTableRow<'a>>);

impl<'a> From<&'a [ImageResults]> for ActivityTable<'a> {
	fn from(src: &'a [ImageResults]) -> Self {
		let mut out = Vec::with_capacity(src.len() * 5);
		for job in src {
			// Push the source.
			out.push(ActivityTableRow {
				src: Cow::Borrowed(&job.src),
				kind: job.src_kind,
				quality: QualityValueFmt::None,
				len: Some(NiceU64::from(job.src_len)),
				ratio: Some(NiceFloat::from(1.0)),

				// Sources never have times; we can use this to signal when
				// an image was skipped.
				time: job.dst.is_empty().then_some(&NiceFloat::ZERO),
			});

			// Push the conversions.
			for (kind, res) in &job.dst {
				if let Some((len, quality)) = res.len.zip(res.quality) {
					out.push(ActivityTableRow {
						src: Cow::Borrowed(&res.src),
						kind: *kind,
						quality: quality.quality_fmt(),
						len: Some(NiceU64::from(len)),
						ratio: job.src_len.get().div_float(len.get()).map(NiceFloat::from),
						time: Some(&res.time),
					});
				}
				else {
					let mut dst = job.src.clone();
					let v = dst.as_mut_os_string();
					v.push(".");
					v.push(kind.extension());
					out.push(ActivityTableRow {
						src: Cow::Owned(dst),
						kind: *kind,
						quality: QualityValueFmt::None,
						len: None,
						ratio: None,
						time: Some(&res.time),
					});
				}
			}
		}

		// Done!
		Self(out)
	}
}

impl ActivityTable<'_> {
	/// # Headers.
	const HEADERS: [&'static str; 6] = [
		"File",
		"Kind",
		"Quality",
		"Size",
		"CR*",
		"Time**",
	];

	/// # Column Widths.
	///
	/// Calculate and return the (approximate) maximum display width for each
	/// column, packed into a more serviceable array format.
	fn widths(&self) -> [usize; 6] {
		self.0.iter()
			.map(ActivityTableRow::widths)
			.fold(Self::HEADERS.map(str::len), |mut acc, v| {
				for (w1, w2) in acc.iter_mut().zip(v) {
					if *w1 < w2 { *w1 = w2; }
				}
				acc
			})
	}
}

/// # Activity Table Row.
///
/// A single row in the table.
struct ActivityTableRow<'a> {
	/// # File Path.
	src: Cow<'a, Path>,

	/// # Image Kind.
	kind: ImageKind,

	/// # Compression Quality.
	quality: QualityValueFmt,

	/// # File Size.
	len: Option<NiceU64>,

	/// # Compression Ratio (old:new).
	ratio: Option<NiceFloat>,

	/// # Computational Time.
	time: Option<&'a NiceFloat>,
}

impl ActivityTableRow<'_> {
	/// # Column Widths.
	///
	/// Calculate and return the (approximate) display width for each field,
	/// packed into a more serviceable array format.
	fn widths(&self) -> [usize; 6] {
		use unicode_width::UnicodeWidthStr;

		[
			self.src.to_string_lossy().width(),
			self.kind.len(),
			self.quality.len(),
			self.len.as_ref().map_or(0, NiceU64::len),
			self.ratio.as_ref().map_or(0, |n| n.precise_str(4).len()),
			self.time.as_ref().map_or(0, |n|
				// Sources never have times; if there's a value here, it'll
				// get printed as "skipped".
				if matches!(self.kind, ImageKind::Jpeg | ImageKind::Png) { 7 }
				else { n.precise_str(3).len() + 1 }
			),
		]
	}
}



/// # Current Image.
///
/// This struct holds the state details for an image that is currently being
/// processed, including the source, settings, last candidate, and encoding
/// iterator.
///
/// Because there is only ever one of these at a time, its existence (or lack
/// thereof) is used to tell which screen to display.
struct CurrentImage {
	/// # Source Path.
	src: PathBuf,

	/// # Decoded Source.
	input: Input,

	/// # Iced-Ready Image Data.
	///
	/// This is largely redundant given that `input` holds the same pixels,
	/// but the caching should help speed up A/B renders.
	img: image::Handle,

	/// # Refract Flags.
	flags: u16,

	/// # Decoded Candidate Image.
	candidate: Option<Candidate>,

	/// # Encoding Count and Iterator.
	iter: Option<(u8, EncodeIter)>,

	/// # Output Kind (Redundant).
	output_kind: Option<ImageKind>,
}

impl CurrentImage {
	/// # New.
	///
	/// This method returns a new instance containing the decoded source
	/// image, if valid.
	///
	/// Note that this does _not_ initialize an encoder or generate a
	/// candidate image. Those tasks can be long-running so are left for later.
	fn new(src: PathBuf, flags: u16) -> Option<Self> {
		let input = std::fs::read(&src).ok()?;
		let input = Input::try_from(input.as_slice()).ok()?.into_rgba();
		let img = image::Handle::from_rgba(
			u32::try_from(input.width()).ok()?,
			u32::try_from(input.height()).ok()?,
			input.pixels_rgba().into_owned(),
		);
		Some(Self {
			src,
			input,
			img,
			flags,
			candidate: None,
			iter: None,
			output_kind: None,
		})
	}

	/// # Is Active?
	///
	/// Returns `true` if an encoder has been set up.
	const fn active(&self) -> bool { self.output_kind.is_some() }

	/// # Provide Feedback.
	fn feedback(&mut self, keep: bool) -> bool {
		if self.candidate.take().is_some() {
			if let Some((_, iter)) = &mut self.iter {
				if keep { iter.keep(); }
				else { iter.discard(); }
				return true;
			}
		}

		false
	}

	/// # Finish Current Encoder.
	///
	/// Remove and return the current encoder data, if any, making room for
	/// the next job.
	///
	/// Post-processing of said data is an exercise left up to the caller.
	fn finish_encoder(&mut self) -> Option<ImageResultWrapper> {
		// Extract a bunch of data.
		let kind = self.output_kind().take();
		let (_, iter) = self.iter.take()?;
		let time = NiceFloat::from(iter.time().as_secs_f32());
		let best = iter.take().ok();
		let kind = kind.or_else(|| best.as_ref().map(Output::kind))?;

		// Copy the source.
		let src = self.src.clone();

		// Come up with a suitable default destination path.
		let mut dst = src.clone();
		let v = dst.as_mut_os_string();
		v.push(".");
		v.push(kind.extension());

		Some(ImageResultWrapper { src, dst, kind, time, best })
	}

	/// # Has Candidate?
	///
	/// Returns `true` if a candidate has been generated.
	const fn has_candidate(&self) -> bool { self.candidate.is_some() }

	/// # Next Candidate (Start).
	///
	/// This method clears the current candidate and advances the guided
	/// encoder (assuming there is one; `None` is returned otherwise).
	///
	/// The actual execution is a bit more convoluted: in order to keep the
	/// time-consuming work _off_ the main thread, we have to temporarily send
	/// the encoder abroad and return it in a `Future<Message>` so it can be
	/// reabsorbed (via `Self::next_candidate_done`).
	///
	/// The workflow isn't ideal, but it all works out.
	fn next_candidate(&mut self) -> Option<Task<Message>> {
		self.candidate = None;
		let borrow = self.iter.take()?;
		Some(Task::future(async {
			let enc = async_std::task::spawn_blocking(||
				EncodeWrapper::from(borrow).advance()
			).await;

			Message::NextStepDone(enc)
		}))
	}

	/// # Next Candidate (Done).
	///
	/// This method reabsorbs the active encoder (that was temporarily sent
	/// to another thread) and updates the candidate image, if any.
	///
	/// Returns `true` if there is now a candidate.
	fn next_candidate_done(&mut self, enc: EncodeWrapper) -> bool {
		let EncodeWrapper { count, iter, output } = enc;
		self.iter.replace((count, iter));
		self.candidate = output;
		self.candidate.is_some()
	}

	/// # Next Encoder.
	///
	/// Pluck the next encoding format from the settings, if any, and
	/// initialize a corresponding encoder.
	///
	/// Returns `true` if successful.
	fn next_encoder(&mut self) -> bool {
		self.candidate = None;
		self.output_kind = None;
		let encoder =
			if FMT_WEBP == self.flags & FMT_WEBP {
				self.flags &= ! FMT_WEBP;
				ImageKind::Webp
			}
			else if FMT_AVIF == self.flags & FMT_AVIF {
				self.flags &= ! FMT_AVIF;
				ImageKind::Avif
			}
			else if FMT_JXL == self.flags & FMT_JXL {
				self.flags &= ! FMT_JXL;
				ImageKind::Jxl
			}
			else { return false; };

		// Convert encoder flags.
		let encoder_flags: u8 =
			if 0 == self.flags & MODE_LOSSY {
				FLAG_NO_LOSSY | FLAG_NO_AVIF_YCBCR
			}
			else {
				let mut flags: u8 = 0;
				if 0 == self.flags & MODE_LOSSLESS { flags |= FLAG_NO_LOSSLESS; }
				if 0 == self.flags & MODE_LOSSY_YCBCR { flags |= FLAG_NO_AVIF_YCBCR; }
				flags
			};

		self.iter = EncodeIter::new(self.input.clone(), encoder, encoder_flags)
			.ok()
			.map(|e| (0, e));

		// It worked if it worked.
		if self.iter.is_some() {
			self.output_kind.replace(encoder);
			true
		}
		else { false }
	}

	/// # Output Kind.
	///
	/// Return the output format that is currently being crunched, if any.
	const fn output_kind(&self) -> Option<ImageKind> { self.output_kind }
}



#[derive(Debug, Clone)]
/// # Encode Wrapper.
///
/// This struct holds a temporarily "borrowed" `EncodeIter` instance, allowing
/// the data and encoding workload to run _off_ the main thread.
///
/// It must be passed back to `CurrentImage` afterwards for post-processing.
pub(super) struct EncodeWrapper {
	/// # Iteration Count.
	count: u8,

	/// # Iterator.
	iter: EncodeIter,

	/// # The Result.
	output: Option<Candidate>
}

impl From<(u8, EncodeIter)> for EncodeWrapper {
	#[inline]
	fn from((count, iter): (u8, EncodeIter)) -> Self {
		Self { count, iter, output: None }
	}
}

impl EncodeWrapper {
	/// # Advance.
	fn advance(mut self) -> Self {
		if let Some(can) = self.iter.advance().and_then(|out| Candidate::try_from(out).ok()) {
			self.count += 1;
			self.output.replace(can.with_count(self.count));
		}
		self
	}
}



/// # Image Encoding Results.
///
/// This struct is used to help group activity logs by source while still
/// allowing for duplication should the user decide to repeat any work.
///
/// It gets initialized each time an image is moved from the queue to `current`,
/// but only if the source can be decoded.
///
/// The conversion details are added as available, with `None` indicating an
/// error or, more typically, a fruitless effort that wasn't worth saving.
struct ImageResults {
	/// # Source Path.
	src: PathBuf,

	/// # Source Kind.
	src_kind: ImageKind,

	/// # Source Size.
	src_len: NonZeroUsize,

	/// # Conversions.
	dst: Vec<(ImageKind, ImageResult)>,
}



/// # (Best) Image Encoding Result.
///
/// This struct holds the details for the best image candidate produced by a
/// given encoding instance, i.e. its location, size, and the codec quality
/// used.
struct ImageResult {
	/// # Path.
	src: PathBuf,

	/// # Size.
	len: Option<NonZeroUsize>,

	/// # Quality.
	quality: Option<Quality>,

	/// # Computational Time (seconds).
	time: NiceFloat,
}



#[derive(Debug, Clone)]
/// # Image Result Wrapper.
///
/// This struct temporarily holds the results of an encoding run, making it
/// easier to split up the various save/log-type finishing tasks.
pub(super) struct ImageResultWrapper {
	/// # Source Path (Sanity Check).
	src: PathBuf,

	/// # Output Path.
	dst: PathBuf,

	/// # Output Kind.
	kind: ImageKind,

	/// # Computational Time (seconds).
	time: NiceFloat,

	/// # Output Image.
	best: Option<Output>,
}

impl ImageResultWrapper {
	/// # Into Result.
	///
	/// Reformat the data for final storage in `ImageResults`, and log the
	/// results to CLI.
	fn into_result(self) -> (ImageKind, ImageResult) {
		if let Some(best) = self.best {
			if let Some(len) = best.size() {
				let quality = best.quality();
				cli_log(&self.dst, Some(quality));
				return (self.kind, ImageResult {
					src: self.dst,
					len: Some(len),
					quality: Some(quality),
					time: self.time,
				});
			}
		}

		cli_log_sad(&self.dst);
		(self.kind, ImageResult {
			src: self.dst,
			len: None,
			quality: None,
			time: self.time,
		})
	}

	/// # Save File.
	///
	/// Permanently save the best candidate, if any, to disk. If this fails,
	/// the candidate will be deleted.
	fn save(&mut self) {
		if let Some(best) = &self.best {
			// If saving fails, pretend there was no best.
			if write_atomic::write_file(&self.dst, best).is_err() {
				let _res = self.best.take();
			}
		}
	}

	/// # Set Output Path.
	///
	/// Pop an async file dialogue so the user can override the output path
	/// or cancel the operation entirely, returning a `Future<Message>` with
	/// the result.
	///
	/// If there is no best candidate image, this returns immediately so we can
	/// get on with it.
	fn open_fd(mut self) -> Task<Message> {
		// Only appropriate if we're saving something.
		if self.best.is_none() { return Task::done(Message::SaveImage(self)); }

		// Get the path.
		Task::future(async {
			let dst = AsyncFileDialog::new()
				.add_filter(self.kind.as_str(), &[self.kind.extension()])
				.set_can_create_directories(true)
				.set_directory(self.dst.parent().unwrap_or_else(|| Path::new(".")))
				.set_file_name(self.dst.file_name().map_or(Cow::Borrowed(""), OsStr::to_string_lossy))
				.set_title(format!("Save the {}!", self.kind))
				.save_file()
				.await;

			// Update the path if they picked one.
			if let Some(dst) = dst {
				let dst = dst.path().to_path_buf();
				if dst.parent().is_some_and(Path::is_dir) && dst.file_name().is_some() {
					self.dst = crate::with_ng_extension(dst, self.kind);
				}
				else { self.best = None; }
			}
			// Or nuke the result.
			else { self.best = None; }

			// Return for saving.
			Message::SaveImage(self)
		})
	}
}



#[derive(Debug, Clone)]
/// # Message.
///
/// This enum is used by `iced` (and occasionally us) to communicate events
/// like button and checkbox clicks so we can react and repaint accordingly.
///
/// They're signals, basically.
pub(super) enum Message {
	/// # Add Image Source Path(s) to the Queue.
	///
	/// This signal processes the results from `OpenFd`. It will trigger
	/// `NextImage` if paths are found and encoding is not already underway.
	AddPaths(Dowser),

	/// # An Error.
	///
	/// See `MessageError` for details.
	Error(MessageError),

	/// # Encoding Feedback.
	///
	/// This signal processes user feedback, rejecting a candidate if `false`,
	/// accepting it if `true`. It will trigger `NextStep` afterwards to set
	/// the next candidate crunching.
	Feedback(bool),

	/// # Next Encoder.
	///
	/// This signal is used to quickly announce a change in encoders (if the
	/// context warrants it). It triggers a `NextStep` when done.
	NextEncoder,

	/// # Next Image.
	///
	/// This is essentially the outermost image-related signal. It moves the
	/// first next queued path to `current` and triggers `NextEncoder`.
	NextImage,

	/// # Next Step.
	///
	/// This signal generates the next candidate image in a separate thread,
	/// triggering a `NextStepDone` once everything is ready.
	///
	/// It is repeated after each round of `Feedback` until no more qualities
	/// remain to be tested.
	NextStep,

	/// # Finish Next Step.
	///
	/// This signal consumes the data produced by `NextStep`. (The two always
	/// go together.)
	///
	/// In most contexts the program will idle after this, waiting for user
	/// feedback, but if we're out of candidates to generate, it'll move onto
	/// `SaveImage`, or `NextEncoder` depending on the state of things.
	NextStepDone(EncodeWrapper),

	/// # Save Image (and Continue).
	///
	/// This signal is used to save the "best" image candidate to disk (if any)
	/// and log the results.
	///
	/// Output paths are either pre-generated or confirmed via file dialogue
	/// beforehand.
	///
	/// When done it triggers `NextEncoder`.
	SaveImage(ImageResultWrapper),

	/// # Open File Dialogue.
	///
	/// This signal pops a file picker if `false` or directory picker if `true`.
	/// Unless canceled, the results will be consumed via an `AddPaths` signal.
	OpenFd(bool),

	/// # Open File.
	///
	/// Poor man's link; ask the DE to open the thing with whatever program
	/// it thinks appropriate.
	OpenFile(PathBuf),

	/// # Open URL.
	///
	/// Poor man's link; ask the DE to open the thing with whatever program
	/// it thinks appropriate.
	OpenUrl(&'static str),

	/// # Toggle Flag.
	///
	/// This signal is used to toggle program settings like Night Mode.
	ToggleFlag(u16),

	/// # Unset Flag.
	///
	/// Like `ToggleFlag`, but only for removal.
	UnsetFlag(u16),
}



#[derive(Debug, Clone, Copy)]
/// # Message Error.
///
/// These enum variants are used to help clarify situations in which nothing,
/// rather than something, happens, such as when a user adds a directory that
/// doesn't actually have any images in it.
pub(super) enum MessageError {
	/// # No Images.
	NoImages,

	/// # Open Failed.
	NoOpen,
}

impl MessageError {
	/// # As Str.
	const fn as_str(self) -> &'static str {
		match self {
			Self::NoImages => "No qualifying images were found.",
			Self::NoOpen => "The link could not be opened.",
		}
	}
}



/// # Widget Cache.
///
/// This struct holds image handles for our embedded and unchanging assets,
/// i.e. the A/B checkerboard backgrounds and program logo, to speed up tree
/// render.
struct WidgetCache {
	/// # Checkerboard Underlay (A).
	checkers_a: image::Handle,

	/// # Checkerboard Underlay (B).
	checkers_b: image::Handle,

	/// # Program Logo.
	logo: image::Handle,
}

impl Default for WidgetCache {
	fn default() -> Self {
		let (checkers_a, checkers_b) = crate::checkers();
		Self {
			checkers_a,
			checkers_b,
			logo: crate::logo(),
		}
	}
}



/// # CLI Log.
///
/// Print a quick timestamped message to STDERR in case anybody's watching.
fn cli_log(src: &Path, quality: Option<Quality>) {
	let Some((dir, name)) = split_path(src) else { return; };
	let now = FmtUtc2k::now_local();
	let mut out = format!(
		"\x1b[2m[\x1b[0;34m{}\x1b[0;2m] {}/\x1b[0m{} \x1b[2m(",
		now.time(),
		dir.to_string_lossy(),
		name.to_string_lossy(),
	);

	if let Some(quality) = quality {
		if ! quality.is_lossless() {
			out.push_str(quality.label());
			out.push(' ');
		}
		out.push_str(&quality.quality_fmt().as_str());
	}
	else { out.push_str("source"); }

	eprintln!("{out})\x1b[0m");
}

/// # Cli Log: Sad Conversion.
///
/// Print a quick timestamped summary of a failed conversion to STDERR.
fn cli_log_sad(src: &Path) {
	let Some((dir, name)) = split_path(src) else { return; };
	let now = FmtUtc2k::now_local();

	eprintln!(
		"\x1b[2m[\x1b[0;34m{}\x1b[0;2m]\x1b[91m {}/\x1b[0;91m{}\x1b[0m",
		now.time(),
		dir.to_string_lossy(),
		name.to_string_lossy(),
	);
}

/// # Cli Log: Error.
///
/// Print a quick timestamped error message to STDERR.
fn cli_log_error(src: MessageError) {
	let now = FmtUtc2k::now_local();
	eprintln!(
		"\x1b[2m[\x1b[0;34m{}\x1b[0;2m]\x1b[0;93m Warning:\x1b[0m {}",
		now.time(),
		src.as_str(),
	);
}

/// # Split Path.
///
/// Split the `parent` and `file_name`, returning `None` if either fail for
/// whatever reason.
fn split_path(src: &Path) -> Option<(&OsStr, &OsStr)> {
	let dir = src.parent()?;
	let name = src.file_name()?;
	Some((dir.as_os_str(), name))
}

/// # Home Subscriptions.
///
/// This callback for `on_key_press` binds listeners for events available on
/// the home screen.
fn subscribe_home(key: Key, modifiers: Modifiers) -> Option<Message> {
	// These require CTRL and not ALT.
	if modifiers.command() && ! modifiers.alt() {
		if let Key::Character(c) = key {
			if c == "n" { return Some(Message::ToggleFlag(OTHER_NIGHT)); }
			if c == "o" { return Some(Message::OpenFd(modifiers.shift())); }
		}
	}

	None
}

/// # A/B Subscriptions.
///
/// This callback for `on_key_press` binds listeners for events available on
/// the A/B screen.
fn subscribe_ab(key: Key, modifiers: Modifiers) -> Option<Message> {
	// Nothing needs ALT.
	if modifiers.alt() { None }
	// CTRL+N toggles Night Mode.
	else if modifiers.command() {
		if let Key::Character(c) = key {
			if c == "n" { return Some(Message::ToggleFlag(OTHER_NIGHT)); }
		}
		None
	}
	else {
		match key {
			// Toggle A/B.
			Key::Named(Named::Space) => Some(Message::ToggleFlag(OTHER_BSIDE)),
			// Feedback.
			Key::Character(c) =>
				if c == "d" { Some(Message::Feedback(false)) }
				else if c == "k" { Some(Message::Feedback(true)) }
				else { None }
			_ => None,
		}
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_flags() {
		// Make sure the flags are actually unique.
		let all = [
			FMT_AVIF, FMT_JXL, FMT_WEBP,
			MODE_LOSSLESS, MODE_LOSSY, MODE_LOSSY_YCBCR,
			OTHER_BSIDE, OTHER_EXIT_AUTO, OTHER_NIGHT, OTHER_SAVE_AUTO,
			SWITCHED_ENCODER,
		];
		let set = all.iter().copied().collect::<BTreeSet<u16>>();
		assert_eq!(all.len(), set.len());
	}
}
