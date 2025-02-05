/*!
# Refract: App
*/

use argyle::Argument;
use crate::{
	button_style,
	Candidate,
	CHECKERS,
	DARK_PALETTE,
	DARK_THEME,
	FONT_BOLD,
	LIGHT_PALETTE,
	LIGHT_THEME,
	NiceColors,
	tooltip_style,
};
use dactyl::{
	NicePercent,
	NiceU64,
};
use dowser::Dowser;
use iced::{
	alignment::{
		Horizontal,
		Vertical,
	},
	Background,
	ContentFit,
	Element,
	Fill,
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
		Scrollable,
		span,
		Stack,
		svg,
		text,
		text::Rich,
		tooltip,
	},
};
use refract_core::{
	EncodeIter,
	FLAG_NO_AVIF_YCBCR,
	FLAG_NO_LOSSLESS,
	FLAG_NO_LOSSY,
	ImageKind,
	Input,
	Quality,
	RefractError,
};
use rfd::FileDialog;
use std::{
	borrow::Cow,
	collections::BTreeSet,
	ffi::OsStr,
	num::NonZeroUsize,
	path::{
		Path,
		PathBuf,
	},
};



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

/// # Night Mode.
const OTHER_NIGHT: u16 =      0b0000_1000_0000;

/// # Save w/o Prompt.
const OTHER_SAVE_AUTO: u16 =  0b0001_0000_0000;

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



/// # Application.
pub(super) struct App {
	/// # Flags.
	flags: u16,

	/// # Paths (Queue).
	paths: BTreeSet<PathBuf>,

	/// # Current Job.
	current: Option<CurrentImage>,

	/// # Last Directory.
	last_dir: Option<PathBuf>,

	/// # Results.
	done: Vec<ImageResults>,

	/// # Error.
	error: Option<MessageError>,
}

impl App {
	/// # New.
	///
	/// Parse the CLI arguments (if any) and return a new instance, unless
	/// `--help` or `--version` were requested instead.
	pub(super) fn new() -> Result<Self, RefractError> {
		let mut paths = Dowser::default();
		let mut flags = DEFAULT_FLAGS;

		// Load CLI arguments, if any.
		let args = argyle::args()
			.with_keywords(include!(concat!(env!("OUT_DIR"), "/argyle.rs")));
		for arg in args {
			match arg {
				Argument::Key("-h" | "--help") => return Err(RefractError::PrintHelp),
				Argument::Key("--no-avif") => { flags &= ! FMT_AVIF; },
				Argument::Key("--no-jxl") => { flags &= ! FMT_JXL; },
				Argument::Key("--no-webp") => { flags &= ! FMT_WEBP; },
				Argument::Key("--no-lossless") => { flags &= ! MODE_LOSSLESS; },
				Argument::Key("--no-lossy") => { flags &= ! MODE_LOSSY; },
				Argument::Key("--no-ycbcr") => { flags &= ! MODE_LOSSY_YCBCR; },
				Argument::Key("--save-auto") => { flags |= OTHER_SAVE_AUTO; },
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
		};

		// Digest the paths, if any.
		out.add_paths(paths);

		// Done!
		Ok(out)
	}
}

/// # Getters.
impl App {
	/// # Has Flag.
	const fn has_flag(&self, flag: u16) -> bool { flag == self.flags & flag }

	/// # State.
	pub(super) fn state(&self) -> State {
		self.current.as_ref().map_or(State::Normal, CurrentImage::state)
	}

	/// # Theme.
	pub(super) fn theme(&self) -> Theme {
		if self.has_flag(OTHER_NIGHT) { DARK_THEME.clone() }
		else { LIGHT_THEME.clone() }
	}
}

/// # Setters.
impl App {
	/// # Digest Paths.
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

		// Add it and the rest.
		self.paths.insert(first);
		self.paths.extend(paths);
	}

	/// # Toggle Flag.
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
	pub(super) fn start(&self) -> Task<Message> {
		if self.paths.is_empty() { Task::none() }
		else { Task::done(Message::NextImage) }
	}

	/// # Update.
	pub(super) fn update(&mut self, message: Message) -> Task<Message> {
		// Clear the last error, if any.
		let _res = self.error.take();

		match message {
			// Add File(s) or Directory.
			Message::AddPaths(dir) => {
				// Try to set a sane starting directory for ourselves.
				let mut fd = FileDialog::new();
				if let Some(p) = self.last_dir.as_ref() { fd = fd.set_directory(p); }
				else if let Ok(p) = std::env::current_dir() {
					fd = fd.set_directory(p);
				}

				// Pop a dialog for the user and wait for their selection.
				let paths =
					if dir {
						fd.set_title("Open Directory")
							.pick_folder()
							.map(Dowser::from)
					}
					else {
						fd.add_filter("Images", &["jpg", "jpeg", "png"])
							.set_title("Open Image(s)")
							.pick_files()
							.map(Dowser::from)
					};

				// Proceed if anything came back.
				if let Some(paths) = paths {
					self.add_paths(paths);

					// Nothing?
					if self.paths.is_empty() {
						return Task::done(Message::Error(MessageError::NoImages));
					}

					// Otherwise let's get going!
					return Task::done(Message::NextImage);
				}
			},

			// An error.
			Message::Error(err) => { self.error.replace(err); },

			// Provide Feedback.
			Message::Feedback(feedback) => {
				self.flags &= ! OTHER_BSIDE;
				if let Some(current) = &mut self.current {
					// Back around again!
					if current.feedback(feedback) {
						return Task::done(Message::NextStep);
					}
				}
			},

			// Load next image.
			Message::NextImage => {
				self.flags &= ! OTHER_BSIDE;
				self.current = None;
				while let Some(src) = self.paths.pop_first() {
					if let Some(mut current) = CurrentImage::new(src, self.flags) {
						// Add an entry for it.
						self.done.push(ImageResults {
							src: current.src.clone(),
							src_kind: current.input.kind(),
							src_len: NonZeroUsize::new(current.input.size()).unwrap(),
							dst: Vec::new(),
						});

						if current.next_encoder() {
							self.current = Some(current);
							return Task::done(Message::NextStep);
						}
					}
				}
			},

			// Encode next image.
			Message::NextStep => {
				self.flags &= ! OTHER_BSIDE;
				let confirm = ! self.has_flag(OTHER_SAVE_AUTO);
				if let Some(current) = &mut self.current {
					// Advance iterator and wait for feedback.
					if current.advance() {
						self.flags |= OTHER_BSIDE;
						return Task::none();
					}

					// Save it!
					if let Some((_, iter)) = current.finish() {
						if let Some(last) = self.done.last_mut() {
							last.push(iter, confirm);
						}
					}

					// Advance the encoder.
					if current.next_encoder() {
						return Task::done(Message::NextStep);
					}
				}

				self.current = None;
				if ! self.paths.is_empty() { return Task::done(Message::NextImage); }
			},

			// Open a file.
			Message::OpenFile(src) => {
				if open::that_detached(src).is_err() {
					return Task::done(Message::Error(MessageError::NoOpen));
				}
			},

			// Open a URL.
			Message::OpenUrl(url) => {
				if open::that_detached(url).is_err() {
					return Task::done(Message::Error(MessageError::NoOpen));
				}
			},

			// Toggle a flag.
			Message::ToggleFlag(flag) => { self.toggle_flag(flag); },
		}

		Task::none()
	}

	/// # View.
	pub(super) fn view(&self) -> Element<'_, Message> {
		match self.state() {
			State::Normal => self.view_normal(),
			State::WaitEncode | State::WaitFeedback => self.view_working(),
		}
	}

	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # Subscription.
	pub(super) fn subscription(&self) -> Subscription<Message> {
		use iced::{
			event::Status,
			Event,
			keyboard::{
				Event::KeyPressed,
				key::Named,
				Key,
			},
		};

		iced::event::listen_with(|event, status, _id| {
			if matches!(status, Status::Ignored) {
				match event {
					// Toggle image A/B.
					Event::Keyboard(KeyPressed {
						key: Key::Named(Named::Space),
						..
					}) => Some(Message::ToggleFlag(OTHER_BSIDE)),

					// Keep or discard a candidate image.
					Event::Keyboard(KeyPressed {
						key: Key::Character(c),
						..
					}) =>
						if c == "d" { Some(Message::Feedback(false)) }
						else if c == "k" { Some(Message::Feedback(true)) }
						else { None },
					_ => None,
				}
			}
			else { None }
		})
	}
}

/// # View: Normal.
impl App {
	/// # View.
	///
	/// This screen is shown when nothing else is going on.
	fn view_normal(&self) -> Element<'_, Message> {
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
			.into()
	}

	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # About.
	fn view_about(&self) -> Column<Message> {
		column!(
			rich_text!(
				span("Refract ").color(NiceColors::PINK),
				span(concat!("v", env!("CARGO_PKG_VERSION"))).color(NiceColors::PURPLE),
			)
				.font(FONT_BOLD),

			rich_text!(
				span(env!("CARGO_PKG_REPOSITORY"))
					.color(NiceColors::GREEN)
					.link(Message::OpenUrl(env!("CARGO_PKG_REPOSITORY")))
			)
				.font(FONT_BOLD),
		)
			.align_x(Horizontal::Right)
			.spacing(5)
			.width(Shrink)
	}

	/// # Error.
	fn view_error(&self) -> Option<Container<Message>> {
		use iced::widget::container::Style;

		self.error.map(|err|
			container(row!(
				rich_text!(
					span("Warning: ").font(FONT_BOLD),
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

	/// # Format Checkboxes.
	fn view_formats(&self) -> Column<Message> {
		column!(
			text("Formats").color(NiceColors::PINK).font(FONT_BOLD),
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

	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # View Instructions.
	fn view_instructions(&self) -> Container<Message> {
		container(
			column!(
				row!(
					button(text("Open Image(s)").size(18).font(FONT_BOLD))
						.style(|_, status| button_style(status, NiceColors::PURPLE))
						.padding(BTN_PADDING)
						.on_press(Message::AddPaths(false)),

					text("or").size(18),

					button(text("Directory").size(18).font(FONT_BOLD))
						.style(|_, status| button_style(status, NiceColors::PINK))
						.padding(BTN_PADDING)
						.on_press(Message::AddPaths(true)),
				)
					.align_y(Vertical::Center)
					.spacing(10)
					.width(Shrink),

				rich_text!(
					span("Choose one or more "),
					span("JPEG").font(FONT_BOLD),
					span(" or "),
					span("PNG").font(FONT_BOLD),
					span(" images."),
				),
			)
				.align_x(Horizontal::Center)
				.spacing(10)
		)
			.center_x(Fill)
			.width(Fill)
	}

	#[expect(clippy::too_many_lines, reason = "Yeah, it's a bit much.")]
	/// # View Log.
	fn view_log(&self) -> Scrollable<Message> {
		type TableRow<'a> = (Cow<'a, Path>, ImageKind, Option<String>, Option<NiceU64>, Option<NicePercent>);

		let mut lines = Column::new();

		// Build up a pretty results table.
		if ! self.done.is_empty() {
			let fg =
				if self.has_flag(OTHER_NIGHT) { DARK_PALETTE.text }
				else { LIGHT_PALETTE.text };

			let mut table: Vec<Result<TableRow, &Path>> = Vec::new();
			for job in &self.done {
				// Nothing?
				if job.dst.is_empty() {
					table.push(Err(job.src.as_path()));
				}
				else {
					// Push the source.
					table.push(Ok((
						Cow::Borrowed(&job.src),
						job.src_kind,
						None,
						Some(NiceU64::from(job.src_len)),
						Some(NicePercent::MAX),
					)));

					// Push the conversions.
					for (kind, res) in &job.dst {
						if let Some(res) = res {
							table.push(Ok((
								Cow::Borrowed(&res.src),
								*kind,
								Some(res.quality.quality().to_string()),
								Some(NiceU64::from(res.len)),
								NicePercent::try_from((res.len.get(), job.src_len.get())).ok(),
							)));
						}
						else {
							let mut dst = job.src.clone();
							let v = dst.as_mut_os_string();
							v.push(".");
							v.push(kind.extension());
							table.push(Ok((
								Cow::Owned(dst),
								*kind,
								None,
								None,
								None,
							)));
						}
					}
				}
			}

			let headers = [
				"File",
				"Kind",
				"Quality",
				"Size",
				"Ratio",
			];

			// Find the max column lengths.
			let mut widths = headers.map(str::len);
			for row in table.iter().filter_map(|r| r.as_ref().ok()) {
				let tmp = [
					row.0.to_string_lossy().len(),
					row.1.len(),
					row.2.as_ref().map_or(0, String::len),
					row.3.as_ref().map_or(0, NiceU64::len),
					row.4.as_ref().map_or(0, NicePercent::len),
				];
				for (w1, w2) in widths.iter_mut().zip(tmp) {
					if *w1 < w2 { *w1 = w2; }
				}
			}

			let total_width = widths.iter().copied().sum::<usize>() + 4 * 3;
			let divider = "-".repeat(total_width);

			// Finally, add all the lines!
			lines = lines.push(rich_text!(
				span(format!("{:<w$}", headers[0], w=widths[0])).color(NiceColors::PURPLE).font(FONT_BOLD),
				span(" | ").color(NiceColors::PINK),
				span(format!("{:<w$}", headers[1], w=widths[1])).color(NiceColors::PURPLE).font(FONT_BOLD),
				span(" | ").color(NiceColors::PINK),
				span(format!("{:>w$}", headers[2], w=widths[2])).color(NiceColors::PURPLE).font(FONT_BOLD),
				span(" | ").color(NiceColors::PINK),
				span(format!("{:>w$}", headers[3], w=widths[3])).color(NiceColors::PURPLE).font(FONT_BOLD),
				span(" | ").color(NiceColors::PINK),
				span(format!("{:>w$}", headers[4], w=widths[4])).color(NiceColors::PURPLE).font(FONT_BOLD),
			));

			for row in table {
				match row {
					Err(path) => {
						let Some(dir) = path.parent() else { continue; };
						let Some(file) = path.file_name() else { continue; };
						lines = lines.push(text(divider.clone()).color(NiceColors::GREY));
						lines = lines.push(rich_text!(
							span(format!("{}/", dir.to_string_lossy())).color(NiceColors::GREY),
							span(file.to_string_lossy()).color(NiceColors::RED),
							span(": Nothing doing.").color(NiceColors::GREY),
						));
					},
					Ok((path, kind, quality, len, per)) => {
						let Some(dir) = path.parent().map(Path::as_os_str) else { continue; };
						let Some(file) = path.file_name() else { continue; };
						let is_src = matches!(kind, ImageKind::Png | ImageKind::Jpeg);
						let color =
							if is_src { fg }
							else if len.is_some() { NiceColors::GREEN }
							else { NiceColors::RED };

						let link =
							if len.is_some() && path.is_file() { Some(Message::OpenFile(path.to_path_buf())) }
							else { None };

						if is_src {
							lines = lines.push(text(divider.clone()).color(NiceColors::PINK));
						}

						lines = lines.push(rich_text!(
							span(format!("{}/", dir.to_string_lossy())).color(NiceColors::GREY),
							span(file.to_string_lossy().into_owned()).color(color).link_maybe(link),
							span(format!("{} | ", " ".repeat(widths[0].saturating_sub(dir.len() + 1 + file.len())))).color(NiceColors::PINK),
							span(format!("{:<w$}", kind.as_str(), w=widths[1])),
							span(" | ").color(NiceColors::PINK),
							span(format!("{:>w$}", quality.unwrap_or_else(String::new), w=widths[2])),
							span(" | ").color(NiceColors::PINK),
							span(format!("{:>w$}", len.as_ref().map_or("", NiceU64::as_str), w=widths[3])),
							span(" | ").color(NiceColors::PINK),
							span(format!("{:>w$}", per.as_ref().map_or("", NicePercent::as_str), w=widths[4])),
						));
					},
				}
			}
		}

		scrollable(container(lines).width(Fill).padding(10))
			.height(Fill)
			.anchor_bottom()
	}

	/// # View Checkboxes.
	fn view_modes(&self) -> Column<Message> {
		column!(
			text("Compression").color(NiceColors::PINK).font(FONT_BOLD),
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

	/// # View Checkboxes.
	fn view_other(&self) -> Column<Message> {
		column!(
			text("Other").color(NiceColors::PINK).font(FONT_BOLD),
			tooltip(
				checkbox("Auto-Save", self.has_flag(OTHER_SAVE_AUTO))
					.on_toggle(|_| Message::ToggleFlag(OTHER_SAVE_AUTO))
					.size(CHK_SIZE),
				container(
					text("Always use the (automatically) derived output paths when saving images instead of popping a file dialogue.")
						.size(12)
				)
					.padding(20)
					.max_width(300_u16)
					.style(|_| tooltip_style(! self.has_flag(OTHER_NIGHT))),
				tooltip::Position::Bottom,
			),
			checkbox("Night Mode", self.has_flag(OTHER_NIGHT))
				.on_toggle(|_| Message::ToggleFlag(OTHER_NIGHT))
				.size(CHK_SIZE),
		)
			.spacing(5)
	}

	/// # View Settings.
	fn view_settings(&self) -> Container<Message> {
		container(
			row!(
				self.view_formats(),
				self.view_modes(),
				self.view_other(),
				self.view_instructions(),
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
}

/// # View: Images.
impl App {
	/// # View Images.
	///
	/// This screen is shown when there are images happening.
	fn view_working(&self) -> Element<'_, Message> {
		container(
			column!(
				self.view_image_summary(),
				self.view_image(),
				container(
					row!(
						self.view_image_legend(),
						self.view_progress(),
						self.view_image_actions(),
					)
						.align_y(Vertical::Center)
						.padding(20)
						.spacing(20)
				)
					.width(Fill)
			)
		)
			.width(Fill)
			.into()
	}

	#[expect(clippy::option_if_let_else, reason = "Absolutely not!")]
	/// # Image Summary Legend.
	fn view_image_legend(&self) -> Column<Message> {
		let Some(current) = self.current.as_ref() else { return Column::new(); };

		if let Some(dst_kind) = current.candidate.as_ref().map(|c| c.kind) {

			column!(
				rich_text!(
					span("[space]").font(FONT_BOLD),
					span(" Toggle image view (").color(NiceColors::GREY),
					span(current.input.kind().to_string()).color(NiceColors::PURPLE).font(FONT_BOLD),
					span(" vs ").color(NiceColors::GREY),
					span(dst_kind.to_string()).color(NiceColors::PINK).font(FONT_BOLD),
					span(").").color(NiceColors::GREY),
				),
				rich_text!(
					span("    [d]").color(NiceColors::RED).font(FONT_BOLD),
					span(" Reject candidate.").color(NiceColors::GREY),
				),
				rich_text!(
					span("    [k]").color(NiceColors::GREEN).font(FONT_BOLD),
					span(" Accept candidate.").color(NiceColors::GREY),
				),
			)
				.spacing(5)
		}
		else {
			column!(
				rich_text!(
					span("The next "),
					current.iter.as_ref().map_or_else(
						|| span("image"),
						|(_, i)| span(i.output_kind().to_string()).color(NiceColors::PINK).font(FONT_BOLD)
					),
					span(" is cooking…"),
				),
				text("Hang tight!").size(18).font(FONT_BOLD),
			)
		}
	}

	/// # Image Progress.
	fn view_progress(&self) -> Column<Message> {
		let Some(current) = self.current.as_ref() else { return Column::new(); };

		let new_kind = current.iter.as_ref().map_or(ImageKind::Png, |(_, i)| i.output_kind());
		let mut formats = Vec::new();
		for (flag, kind) in [
			(FMT_WEBP, ImageKind::Webp),
			(FMT_AVIF, ImageKind::Avif),
			(FMT_JXL, ImageKind::Jxl),
		] {
			if self.has_flag(flag) {
				if ! formats.is_empty() {
					formats.push(span(" + ").color(NiceColors::GREY));
				}
				if kind == new_kind {
					formats.push(span(kind.to_string()).color(NiceColors::PINK).font(FONT_BOLD));
				}
				else {
					formats.push(span(kind.to_string()).color(NiceColors::GREY).font(FONT_BOLD));
				}
			}
		}
		formats.insert(0, span(" > ").color(NiceColors::GREY));
		formats.insert(0, span(current.input.kind().to_string()).color(NiceColors::PURPLE).font(FONT_BOLD));

		column!(
			text(current.src.to_string_lossy()).color(NiceColors::GREY),

			Rich::with_spans(formats),

			checkbox("Night Mode", self.has_flag(OTHER_NIGHT))
				.on_toggle(|_| Message::ToggleFlag(OTHER_NIGHT))
				.size(CHK_SIZE),
		)
			.spacing(5)
			.align_x(Horizontal::Center)
			.width(Fill)
	}

	/// # Image Summary.
	fn view_image_summary(&self) -> Container<Message> {
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
			if ! self.has_flag(MODE_LOSSY) && self.has_flag(OTHER_SAVE_AUTO) {
				row = row.push(text(
					"Lossless conversion is automatic. Just sit back and wait!"
				).font(FONT_BOLD));
			}
			else {
				row = row.push(text("Reticulating splines…").font(FONT_BOLD));
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

			row = row.push(text(kind.to_string()).font(FONT_BOLD));

			if count != 0 {
				row = row.push(rich_text!(
					span("Take: "),
					span(format!("#{count}")).font(FONT_BOLD),
				));
			}

			if let Some(quality) = quality {
				row = row.push(rich_text!(
					span(format!("{}: ", quality.label_title())),
					span(quality.quality().to_string()).font(FONT_BOLD),
				));
			}
			else {
				row = row.push(rich_text!(
					span("Quality: "),
					span("Original").font(FONT_BOLD),
				));
			}
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

	/// # Image Summary Actions.
	fn view_image_actions(&self) -> Row<Message> {
		let active = self.current.as_ref().is_some_and(|c| c.candidate.is_some());

		// Keep and discard buttons.
		let btn_no = button(text("Reject").size(18).font(FONT_BOLD))
			.style(|_, status| button_style(status, NiceColors::RED))
			.padding(BTN_PADDING)
			.on_press_maybe(active.then_some(Message::Feedback(false)));
		let btn_yes = button(text("Accept").size(18).font(FONT_BOLD))
			.style(|_, status| button_style(status, NiceColors::GREEN))
			.padding(BTN_PADDING)
			.on_press_maybe(active.then_some(Message::Feedback(true)));

		row!(btn_no, btn_yes)
			.width(Shrink)
			.spacing(10)
	}

	/// # Image Stack.
	fn view_image(&self) -> Stack<Message> {
		let mut layers = Stack::new();
		layers = layers.push(self.view_image_layer0());
		layers = layers.push_maybe(self.view_image_layer1());
		layers
			.width(Fill)
			.height(Fill)
	}

	#[expect(clippy::unused_self, reason = "Required by API.")]
	/// # Pixel Background Layer.
	fn view_image_layer0(&self) -> Container<Message> {
		use iced::widget::svg::Handle;
		container(
			svg(Handle::from_memory(CHECKERS))
				.opacity(0.2)
				.content_fit(ContentFit::None)
				.width(Fill)
				.height(Fill)
		)
			.clip(true)
	}

	#[expect(clippy::cast_possible_truncation, reason = "Meh.")]
	/// # Image Layer.
	fn view_image_layer1(&self) -> Option<Container<Message>> {
		use iced::widget::{
			image::Handle,
			scrollable::{
				Direction,
				Scrollbar,
			},
		};

		let current = self.current.as_ref()?;
		let mut handle = None;

		// Show the new one?
		if self.has_flag(OTHER_BSIDE) {
			if let Some(can) = current.candidate.as_ref() {
				handle.replace(Handle::from_rgba(
					can.width.get(),
					can.height.get(),
					can.buf.to_vec(),
				));
			}
		}

		// If we aren't showing the new one, show the old one.
		let handle = handle.unwrap_or_else(|| Handle::from_rgba(
			current.input.width() as u32,
			current.input.height() as u32,
			current.input.to_vec(),
		));

		Some(
			container(
				scrollable(
						image(handle)
							.content_fit(ContentFit::None)
							.width(Shrink)
							.height(Shrink)
							.opacity(if current.candidate.is_some() { 1.0 } else { 0.5 })
				)
					.width(Shrink)
					.height(Shrink)
					.direction(Direction::Both { vertical: Scrollbar::new(), horizontal: Scrollbar::new() })
			)
				.width(Fill)
				.height(Fill)
				.center(Fill)
		)
	}
}



/// # Current Image.
struct CurrentImage {
	/// # Source Path.
	src: PathBuf,

	/// # Decoded Source.
	input: Input,

	/// # Refract Flags.
	flags: u16,

	/// # Candidate Image.
	candidate: Option<Candidate>,

	/// # Encoding Count and Iterator.
	iter: Option<(u8, EncodeIter)>,
}

impl CurrentImage {
	/// # New.
	fn new(src: PathBuf, flags: u16) -> Option<Self> {
		let input = std::fs::read(&src).ok()?;
		let input = Input::try_from(input.as_slice()).ok()?;
		Some(Self {
			src,
			input,
			flags,
			candidate: None,
			iter: None,
		})
	}

	/// # Finish (Current Encoder).
	fn finish(&mut self) -> Option<(u8, EncodeIter)> { self.iter.take() }

	/// # State.
	const fn state(&self) -> State {
		if self.iter.is_some() {
			if self.candidate.is_some() { State::WaitFeedback }
			else { State::WaitEncode }
		}
		else { State::Normal }
	}

	/// # Advance.
	///
	/// Advance the guide, returning `true` if a new candidate was generated.
	fn advance(&mut self) -> bool {
		self.candidate = None;
		if let Some((count, iter)) = &mut self.iter {
			if let Some(candidate) = iter.advance().and_then(|out| Candidate::try_from(out).ok()) {
				*count += 1;
				self.candidate = Some(candidate.with_count(*count));
				return true;
			}
		}

		false
	}

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

	/// # Next Encoder.
	///
	/// Clear any previous results and move onto the next encoder, returning
	/// `true` if there is one.
	fn next_encoder(&mut self) -> bool {
		let _res = self.candidate.take();
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
		self.iter.is_some()
	}
}



/// # Image Encoding Results.
struct ImageResults {
	/// # Source Path.
	src: PathBuf,

	/// # Source Kind.
	src_kind: ImageKind,

	/// # Source Size.
	src_len: NonZeroUsize,

	/// # Conversions.
	dst: Vec<(ImageKind, Option<ImageResult>)>,
}

impl ImageResults {
	/// # Push and Save Result.
	fn push(&mut self, iter: EncodeIter, confirm: bool) {
		let kind = iter.output_kind();

		if let Some((len, best)) = iter.output_size().zip(iter.take().ok()) {
			// Come up with a suitable default destination path.
			let mut dst = self.src.clone();
			let v = dst.as_mut_os_string();
			v.push(".");
			v.push(kind.extension());

			// If confirmation is required, suggest the default but let the
			// user decide where it should go.
			if confirm {
				if let Some(p) = FileDialog::new()
					.add_filter(kind.as_str(), &[kind.extension()])
					.set_can_create_directories(true)
					.set_directory(dst.parent().unwrap_or_else(|| Path::new(".")))
					.set_file_name(dst.file_name().map_or(Cow::Borrowed(""), OsStr::to_string_lossy))
					.set_title(format!("Save the {kind}"))
					.save_file()
				{
					dst = crate::with_ng_extension(p, kind);
				}
				// Abort on CANCEL or whatever.
				else {
					self.dst.push((kind, None));
					return;
				}
			}

			// Save it and record the results!
			if write_atomic::write_file(&dst, &best).is_ok() {
				let quality = best.quality();

				self.dst.push((kind, Some(ImageResult {
					src: dst,
					len,
					quality,
				})));
				return;
			}
		}

		self.dst.push((kind, None));
	}
}

/// # (Best) Image Encoding Result.
struct ImageResult {
	/// # Path.
	src: PathBuf,

	/// # Size.
	len: NonZeroUsize,

	/// # Quality.
	quality: Quality,
}



#[derive(Debug, Clone)]
/// # Message.
pub(super) enum Message {
	/// # File Open Dialog.
	AddPaths(bool),

	/// # An Error.
	Error(MessageError),

	/// # Encoding Feedback.
	Feedback(bool),

	/// # Next Image.
	NextImage,

	/// # Next Step.
	NextStep,

	/// # Open File.
	OpenFile(PathBuf),

	/// # Open URL.
	OpenUrl(&'static str),

	/// # Toggle Flag.
	ToggleFlag(u16),
}



#[derive(Debug, Clone, Copy)]
/// # Message Error.
///
/// This is used for non-critical errors that nonetheless deserve a mention,
/// like adding a directory without any qualifying images.
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



#[derive(Default, Debug, Clone, Copy)]
/// # State.
pub(super) enum State {
	#[default]
	/// # Normal.
	Normal,

	/// # Encoding.
	WaitEncode,

	/// # Waiting on Feedback.
	WaitFeedback,
}
