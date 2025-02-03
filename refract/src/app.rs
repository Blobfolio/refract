/*!
# Refract: App
*/

use argyle::Argument;
use crate::Candidate;
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
	Border,
	Color,
	ContentFit,
	Element,
	Fill,
	font::{
		Font,
		Weight,
	},
	Padding,
	Shrink,
	Shadow,
	Subscription,
	Task,
	theme::Palette,
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
		text_input,
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
	ffi::OsStr,
	num::NonZeroUsize,
	path::{
		Path,
		PathBuf,
	},
};



/// # Format: AVIF.
pub(super) const FMT_AVIF: u8 =         0b0000_0001;

/// # Format: JPEG XL.
pub(super) const FMT_JXL: u8 =          0b0000_0010;

/// # Format: WebP.
pub(super) const FMT_WEBP: u8 =         0b0000_0100;

/// # Mode: Lossless.
pub(super) const MODE_LOSSLESS: u8 =    0b0000_1000;

/// # Mode: Lossy.
pub(super) const MODE_LOSSY: u8 =       0b0001_0000;

/// # Mode: Lossy + YCBCR.
///
/// This only applies for AVIF conversions.
pub(super) const MODE_LOSSY_YCBCR: u8 = 0b0010_0000;

/// # Show B (Candidate) Image.
pub(super) const OTHER_BSIDE: u8 =      0b0100_0000;

/// # Night Mode.
pub(super) const OTHER_NIGHT: u8 =      0b1000_0000;

/// # All Formats.
pub(super) const FMT_FLAGS: u8 =
	FMT_AVIF | FMT_JXL | FMT_WEBP;

/// # All Modes.
pub(super) const MODE_FLAGS: u8 =
	MODE_LOSSLESS | MODE_LOSSY;

/// # Default Flags.
pub(super) const DEFAULT_FLAGS: u8 =
	FMT_FLAGS | MODE_FLAGS | MODE_LOSSY_YCBCR;

/// # Hot Pink (#ff3596).
const COLOR_PINK: Color = Color::from_rgb(1.0, 0.208, 0.588);

/// # Purple (#9b59b6).
const COLOR_PURPLE: Color = Color::from_rgb(0.608, 0.349, 0.714);

/// # Green (#2ecc71).
const COLOR_GREEN: Color = Color::from_rgb(0.18, 0.8, 0.443);

/// # Red (#e74c3c).
const COLOR_RED: Color = Color::from_rgb(0.906, 0.298, 0.235);

/// # Check Size.
const CHK_SIZE: u16 = 12_u16;

/// # Bold Font.
const BOLD: Font = Font {
	weight: Weight::Bold,
	..Font::MONOSPACE
};

/// # Button Padding.
const BTN_PADDING: Padding = Padding {
	top: 10.0,
	right: 20.0,
	bottom: 8.0,
	left: 20.0,
};



/// # Settings Header.
macro_rules! settings_header {
	($lbl:literal) => (
		text($lbl)
			.color(COLOR_PINK)
			.font(BOLD)
	);
}



/// # Application.
pub(super) struct App {
	/// # Flags.
	flags: u8,

	/// # Paths (Queue).
	paths: Vec<PathBuf>,

	/// # Current Job.
	current: Option<CurrentImage>,

	/// # Results.
	done: Vec<ImageResults>,
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

		// Digest the paths.
		let paths = paths.into_vec_filtered(crate::is_jpeg_png);

		Ok(Self {
			flags,
			paths,
			current: None,
			done: Vec::new(),
		})
	}
}

/// # Getters.
impl App {
	/// # Has Flag.
	const fn has_flag(&self, flag: u8) -> bool { flag == self.flags & flag }

	/// # Background.
	///
	/// Returns A or B depending on what's going on.
	const fn bg(&self) -> &'static [u8] {
		if self.has_flag(OTHER_NIGHT) { crate::BG_DARK }
		else { crate::BG_LIGHT }
	}

	/// # State.
	pub(super) fn state(&self) -> State {
		self.current.as_ref().map_or(State::Normal, CurrentImage::state)
	}

	/// # Half Text.
	const fn half_text_color(&self) -> Color {
		let palette =
			if self.has_flag(OTHER_NIGHT) { Palette::DARK }
			else { Palette::LIGHT };

		let mut color = palette.text;
		color.a = 0.5;
		color
	}

	/// # Theme.
	pub(super) const fn theme(&self) -> Theme {
		if self.has_flag(OTHER_NIGHT) { Theme::Dark }
		else { Theme::Light }
	}
}

/// # Setters.
impl App {
	/// # Toggle Flag.
	fn toggle_flag(&mut self, flag: u8) {
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
		match message {
			// Load next image.
			Message::NextImage => {
				self.flags &= ! OTHER_BSIDE;
				self.current = None;
				while let Some(src) = self.paths.pop() {
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
				if let Some(current) = &mut self.current {
					// Advance iterator and wait for feedback.
					if current.advance() {
						self.flags |= OTHER_BSIDE;
						return Task::none();
					}

					// Save it!
					if let Some((_, iter)) = current.finish() {
						if let Some(last) = self.done.last_mut() {
							last.push(iter);
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

			// Toggle a flag.
			Message::ToggleFlag(flag) => { self.toggle_flag(flag); },

			// Add File(s) or Directory.
			Message::AddPaths(dir) => {
				let paths =
					if dir {
						FileDialog::new()
							.set_title("Open Directory")
							.pick_folder()
							.map(Dowser::from)
					}
					else {
						FileDialog::new()
							.add_filter("Images", &["jpg", "jpeg", "png"])
							.set_title("Open Image(s)")
							.pick_files()
							.map(Dowser::from)
					};
				if let Some(paths) = paths {
					self.paths.extend(paths.filter(|p: &PathBuf| crate::is_jpeg_png(p)));
					if ! self.paths.is_empty() { return Task::done(Message::NextImage); }
				}
			},
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

	#[expect(clippy::unused_self, reason = "Not our API.")]
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
				.spacing(10)
		)
			.padding(10)
			.width(Fill)
			.into()
	}

	/// # About.
	fn view_about(&self) -> Column<Message> {
		column!(
			rich_text!(
				span("Refract ").color(COLOR_PINK),
				span(concat!("v", env!("CARGO_PKG_VERSION"))).color(COLOR_PURPLE),
				span(format!(" ({})", utc2k::FmtUtc2k::now().date())).color(self.half_text_color()),
			)
				.font(BOLD),

			// To make the text selectable, we need to use an input field,
			// reskinned to not look like an input field. Haha.
			text_input(env!("CARGO_PKG_REPOSITORY"), env!("CARGO_PKG_REPOSITORY"))
				.style(|_, _| iced::widget::text_input::Style {
					background: Background::Color(Color::TRANSPARENT),
					border: Border {
						color: Color::TRANSPARENT,
						width: 0.0,
						radius: 0.0.into(),
					},
					icon: COLOR_PURPLE,
					placeholder: Color::TRANSPARENT,
					value: COLOR_GREEN,
					selection: COLOR_GREEN.scale_alpha(0.2),
				})
				.font(BOLD)
				.padding(0),
		)
			.width(Shrink)
	}

	/// # Format Checkboxes.
	fn view_formats(&self) -> Column<Message> {
		column!(
			settings_header!("Formats"),
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
	}

	#[expect(clippy::unused_self, reason = "Lifetime gets weird without it.")]
	/// # View Instructions.
	fn view_instructions(&self) -> Container<Message> {
		container(
			column!(
				row!(
					button(text("Open Image(s)").size(18).font(BOLD))
						.style(|_, status| button_style(status, COLOR_PURPLE))
						.padding(BTN_PADDING)
						.on_press(Message::AddPaths(false)),

					text("or").size(18),

					button(text("Directory").size(18).font(BOLD))
						.style(|_, status| button_style(status, COLOR_PINK))
						.padding(BTN_PADDING)
						.on_press(Message::AddPaths(true)),
				)
					.align_y(Vertical::Center)
					.spacing(10)
					.width(Shrink),

				rich_text!(
					span("Choose one or more "),
					span("JPEG").font(BOLD),
					span(" or "),
					span("PNG").font(BOLD),
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
			let grey = self.half_text_color();
			let fg =
				if self.has_flag(OTHER_NIGHT) { Palette::DARK.text }
				else { Palette::LIGHT.text };

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
				span(format!("{:<w$}", headers[0], w=widths[0])).color(COLOR_PURPLE).font(BOLD),
				span(" | ").color(COLOR_PINK),
				span(format!("{:<w$}", headers[1], w=widths[1])).color(COLOR_PURPLE).font(BOLD),
				span(" | ").color(COLOR_PINK),
				span(format!("{:>w$}", headers[2], w=widths[2])).color(COLOR_PURPLE).font(BOLD),
				span(" | ").color(COLOR_PINK),
				span(format!("{:>w$}", headers[3], w=widths[3])).color(COLOR_PURPLE).font(BOLD),
				span(" | ").color(COLOR_PINK),
				span(format!("{:>w$}", headers[4], w=widths[4])).color(COLOR_PURPLE).font(BOLD),
			));

			for row in table {
				match row {
					Err(path) => {
						let Some(dir) = path.parent() else { continue; };
						let Some(file) = path.file_name() else { continue; };
						lines = lines.push(text(divider.clone()).color(grey));
						lines = lines.push(rich_text!(
							span(format!("{}/", dir.to_string_lossy())).color(grey),
							span(file.to_string_lossy()).color(COLOR_RED),
							span(": Nothing doing.").color(grey),
						));
					},
					Ok((path, kind, quality, len, per)) => {
						let Some(dir) = path.parent().map(Path::as_os_str) else { continue; };
						let Some(file) = path.file_name() else { continue; };
						let is_src = matches!(kind, ImageKind::Png | ImageKind::Jpeg);
						let color =
							if is_src { fg }
							else if len.is_some() { COLOR_GREEN }
							else { COLOR_RED };

						if is_src {
							lines = lines.push(text(divider.clone()).color(COLOR_PINK));
						}

						lines = lines.push(rich_text!(
							span(format!("{}/", dir.to_string_lossy())).color(grey),
							span(file.to_string_lossy().into_owned()).color(color),
							span(format!("{} | ", " ".repeat(widths[0].saturating_sub(dir.len() + 1 + file.len())))).color(COLOR_PINK),
							span(format!("{:<w$}", kind.as_str(), w=widths[1])),
							span(" | ").color(COLOR_PINK),
							span(format!("{:>w$}", quality.unwrap_or_else(String::new), w=widths[2])),
							span(" | ").color(COLOR_PINK),
							span(format!("{:>w$}", len.as_ref().map_or("", NiceU64::as_str), w=widths[3])),
							span(" | ").color(COLOR_PINK),
							span(format!("{:>w$}", per.as_ref().map_or("", NicePercent::as_str), w=widths[4])),
						));
					},
				}
			}
		}

		scrollable(container(lines).padding(10))
			.height(Fill)
			.anchor_bottom()
	}

	/// # View Checkboxes.
	fn view_modes(&self) -> Column<Message> {
		column!(
			settings_header!("Compression"),
			checkbox("Lossless", self.has_flag(MODE_LOSSLESS))
				.on_toggle(|_| Message::ToggleFlag(MODE_LOSSLESS))
				.size(CHK_SIZE),
			checkbox("Lossy", self.has_flag(MODE_LOSSY))
				.on_toggle(|_| Message::ToggleFlag(MODE_LOSSY))
				.size(CHK_SIZE),
			checkbox("Lossy YCBCR", self.has_flag(MODE_LOSSY_YCBCR))
				.on_toggle_maybe(self.has_flag(FMT_AVIF | MODE_LOSSY).then_some(|_| Message::ToggleFlag(MODE_LOSSY_YCBCR)))
				.size(CHK_SIZE),
		)
	}

	/// # View Checkboxes.
	fn view_other(&self) -> Column<Message> {
		column!(
			settings_header!("Other"),
			checkbox("Night Mode", self.has_flag(OTHER_NIGHT))
				.on_toggle(|_| Message::ToggleFlag(OTHER_NIGHT))
				.size(CHK_SIZE),
		)
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
					.style(|_| {
						let mut style = bordered_box(&self.theme());
						let _res = style.background.take();
						style
					})
					.width(Fill)
			)
				.spacing(10)
		)
			.padding(10)
			.width(Fill)
			.into()
	}

	#[expect(clippy::option_if_let_else, reason = "Absolutely not!")]
	/// # Image Summary Legend.
	fn view_image_legend(&self) -> Column<Message> {
		let Some(current) = self.current.as_ref() else { return Column::new(); };

		let grey = self.half_text_color();
		if let Some(dst_kind) = current.candidate.as_ref().map(|c| c.kind) {

			column!(
				rich_text!(
					span("[space]").font(BOLD),
					span(" Toggle image view (").color(grey),
					span(current.input.kind().to_string()).color(COLOR_PURPLE).font(BOLD),
					span(" vs ").color(grey),
					span(dst_kind.to_string()).color(COLOR_PINK).font(BOLD),
					span(").").color(grey),
				),
				rich_text!(
					span("    [d]").color(COLOR_RED).font(BOLD),
					span(" Reject candidate.").color(grey),
				),
				rich_text!(
					span("    [k]").color(COLOR_GREEN).font(BOLD),
					span(" Accept candidate.").color(grey),
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
						|(_, i)| span(i.output_kind().to_string()).color(COLOR_PINK).font(BOLD)
					),
					span(" is cooking…"),
				),
				text("Hang tight!").size(18).font(BOLD),
			)
		}
	}

	/// # Image Progress.
	fn view_progress(&self) -> Column<Message> {
		let Some(current) = self.current.as_ref() else { return Column::new(); };
		let grey = self.half_text_color();

		let new_kind = current.iter.as_ref().map_or(ImageKind::Png, |(_, i)| i.output_kind());
		let mut formats = Vec::new();
		for (flag, kind) in [
			(FMT_WEBP, ImageKind::Webp),
			(FMT_AVIF, ImageKind::Avif),
			(FMT_JXL, ImageKind::Jxl),
		] {
			if self.has_flag(flag) {
				if ! formats.is_empty() {
					formats.push(span(" + ").color(grey));
				}
				if kind == new_kind {
					formats.push(span(kind.to_string()).color(COLOR_PINK).font(BOLD));
				}
				else {
					formats.push(span(kind.to_string()).color(grey).font(BOLD));
				}
			}
		}
		formats.insert(0, span(" > ").color(grey));
		formats.insert(0, span(current.input.kind().to_string()).color(COLOR_PURPLE).font(BOLD));

		column!(
			text(current.src.to_string_lossy()).color(grey),

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
		let mut quality = None;
		let mut kind = current.input.kind();
		let mut count = 0;
		let mut color = COLOR_PURPLE;

		// Pull the candidate info if we're looking at that.
		if self.has_flag(OTHER_BSIDE) {
			if let Some(can) = current.candidate.as_ref() {
				kind = can.kind;
				count = can.count;
				color = COLOR_PINK;
				quality.replace(can.quality);
			}
		}

		row = row.push(text(kind.to_string()).font(BOLD));

		if count != 0 {
			row = row.push(rich_text!(
				span("Take: "),
				span(format!("#{count}")).font(BOLD),
			));
		}

		if let Some(quality) = quality {
			row = row.push(rich_text!(
				span(format!("{}: ", quality.label_title())),
				span(quality.quality().to_string()).font(BOLD),
			));
		}
		else {
			row = row.push(rich_text!(
				span("Quality: "),
				span("Original").font(BOLD),
			));
		}

		container(row)
			.padding(Padding {
				top: 10.0,
				right: 10.0,
				bottom: 8.0,
				left: 10.0,
			})
			.center(Fill)
			.height(Shrink)
			.style(move |_| Style {
				text_color: Some(Color::WHITE),
				background: Some(Background::Color(color)),
				..Style::default()
			})
	}

	/// # Image Summary Actions.
	fn view_image_actions(&self) -> Row<Message> {
		let active = self.current.as_ref().is_some_and(|c| c.candidate.is_some());

		// Keep and discard buttons.
		let btn_no = button(text("Reject").size(18).font(BOLD))
			.style(|_, status| button_style(status, COLOR_RED))
			.padding(BTN_PADDING)
			.on_press_maybe(active.then_some(Message::Feedback(false)));
		let btn_yes = button(text("Accept").size(18).font(BOLD))
			.style(|_, status| button_style(status, COLOR_GREEN))
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

	/// # Pixel Background Layer.
	fn view_image_layer0(&self) -> Container<Message> {
		use iced::widget::svg::Handle;
		container(
			svg(Handle::from_memory(self.bg()))
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
				.padding(10)
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
	input: Input<'static>,

	/// # Refract Flags.
	flags: u8,

	/// # Candidate Image.
	candidate: Option<Candidate>,

	/// # Encoding Count and Iterator.
	iter: Option<(u8, EncodeIter)>,
}

impl CurrentImage {
	/// # New.
	fn new(src: PathBuf, flags: u8) -> Option<Self> {
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
	fn push(&mut self, iter: EncodeIter) {
		let kind = iter.output_kind();

		if let Some((len, best)) = iter.output_size().zip(iter.take().ok()) {
			// Come up with a suitable destination path.
			let mut dst = self.src.clone();
			let v = dst.as_mut_os_string();
			v.push(".");
			v.push(kind.extension());

			// But make the user verify it.
			if let Some(dst) = FileDialog::new()
				.add_filter(kind.as_str(), &[kind.extension()])
				.set_can_create_directories(true)
				.set_directory(dst.parent().unwrap_or_else(|| Path::new(".")))
				.set_file_name(dst.file_name().map_or(Cow::Borrowed(""), OsStr::to_string_lossy))
				.set_title(format!("Save the {kind}"))
				.save_file()
			{
				let dst = crate::with_ng_extension(dst, kind);
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



#[derive(Debug, Clone, Copy)]
/// # Message.
pub(super) enum Message {
	/// # Encoding Feedback.
	Feedback(bool),

	/// # File Open Dialog.
	AddPaths(bool),

	/// # Next Image.
	NextImage,

	/// # Next Step.
	NextStep,

	/// # Toggle Flag.
	ToggleFlag(u8),
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



/// # Button Style.
fn button_style(status: iced::widget::button::Status, base: Color) -> iced::widget::button::Style {
	use iced::widget::button::{Status, Style};
	Style {
		background: Some(Background::Color(match status {
			Status::Active => base,
			Status::Hovered | Status::Pressed => base.scale_alpha(0.9),
			Status::Disabled => base.scale_alpha(0.5),
		})),
		text_color: Color::WHITE,
		border: Border {
			color: Color::TRANSPARENT,
			width: 0.0,
			radius: 8.0.into(),
		},
		shadow: Shadow::default(),
	}
}
