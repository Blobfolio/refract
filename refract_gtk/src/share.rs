/*!
# `Refract GTK` - Sharing
*/

use atomic::Atomic;
use crate::{
	Candidate,
	Window,
};
use crossbeam_channel::{
	Receiver,
	Sender,
};
use refract_core::{
	ImageKind,
	Output,
	RefractError,
};
use std::{
	cell::RefCell,
	convert::TryFrom,
	path::PathBuf,
	sync::{
		Arc,
		atomic::Ordering::SeqCst,
	},
	time::Duration,
};



/// # Feedback Check Delay.
///
/// When a sister thread sends data back to the main thread, it sometimes needs
/// to wait for a response. In such cases, the sister thread will check for
/// updates at this frequency.
///
/// A `Condvar` would make more sense were it not for their "spurious wakeups".
/// Haha.
const FEEDBACK_PAUSE: Duration = Duration::from_millis(60);



/// # Payload Type.
pub(super) type SharePayload = Result<Share, RefractError>;



thread_local!(
	#[allow(clippy::type_complexity)] // This is the only definition.
	/// # Global.
	///
	/// This gives us a way to reach the main thread from a sister thread.
	static GLOBAL: RefCell<Option<(Arc<Window>, Receiver<SharePayload>, Arc<Atomic<ShareFeedback>>)>> = RefCell::new(None);
);



#[derive(Debug)]
/// # Shared Data.
///
/// This is data passed from a sister thread back to the main thread via shared
/// channels. This is entirely encoding-related.
pub(super) enum Share {
	/// # Path.
	Path(PathBuf),

	/// # New Source.
	Source(Candidate),

	/// # Encoder.
	Encoder(ImageKind),

	/// # New Candidate.
	Candidate(Candidate),

	/// # Final "Best" Output.
	Best(PathBuf, Output),

	/// # Totally Done.
	DoneEncoding,
}

impl TryFrom<&Output> for Share {
	type Error = RefractError;

	#[inline]
	fn try_from(src: &Output) -> Result<Self, Self::Error> {
		let inner = Candidate::try_from(src)?;
		Ok(Self::Candidate(inner))
	}
}

impl Share {
	/// # Initialize.
	pub(super) fn init(window: Arc<Window>)
	-> (Sender<SharePayload>, Arc<Atomic<ShareFeedback>>) {
		let (tx, rx) = crossbeam_channel::bounded(8);
		let fb = Arc::new(Atomic::new(ShareFeedback::Ok));
		GLOBAL.with(|global| {
			*global.borrow_mut() = Some((window, rx, Arc::clone(&fb)));
		});

		(tx, fb)
	}

	/// # Sync Share.
	///
	/// This pushes a payload to the main thread, then optionally waits for and
	/// returns the response.
	///
	/// When not waiting for a response, [`ShareFeedback::Ok`] is returned
	/// immediately.
	pub(super) fn sync(
		tx: &Sender<SharePayload>,
		fb: &Arc<Atomic<ShareFeedback>>,
		share: SharePayload,
		verify: bool,
	) -> ShareFeedback {
		fb.store(ShareFeedback::WantsFeedback, SeqCst);
		tx.send(share).unwrap();
		glib::source::idle_add(|| {
			get_share();
			glib::source::Continue(false)
		});

		if verify {
			loop {
				let res = fb.load(SeqCst);
				if res == ShareFeedback::WantsFeedback {
					std::thread::sleep(FEEDBACK_PAUSE);
				}
				else {
					return res;
				}
			}
		}
		else { ShareFeedback::Ok }
	}
}



#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Feedback.
///
/// This enum is used by the main thread when responding to a [`SharePayload`]
/// sent from a sister thread.
///
/// This is primarily used for candidate feedback — where the user has to
/// decide to keep or kill the image — but it may also indicate an error, in
/// which case the sister thread will try to close itself down.
pub(super) enum ShareFeedback {
	/// # Payload Accepted. Continue...
	Ok,

	/// # Payload Rejected. Abort...
	Err,

	/// # Discard Candidate.
	Discard,

	/// # Keep Candidate.
	Keep,

	/// # Waiting on Feedback.
	///
	/// This status is always set when sending a new [`SharePayload`], but it
	/// will also be returned by the main thread when, well, it is waiting for
	/// feedback.
	///
	/// The sister thread treats this as a blocking value and will not continue
	/// its work until it changes to something else.
	WantsFeedback,
}



/// # Receive Data on the Main Thread.
///
/// This method uses `thread_local` data to receive and parse data sent from a
/// sister thread on the main thread (so e.g. UI actions may be taken).
///
/// ## Panics
///
/// This will panic if the global data is missing from the thread. This
/// shouldn't actually happen, though.
fn get_share() {
	GLOBAL.with(|global| {
		let ptr = global.borrow();
		let (ui, rx, feedback) = ptr.as_ref()
			.expect("An unregistered thread was encountered.");

		if let Ok(res) = rx.recv() {
			feedback.store(ui.process_share(res).unwrap_or(ShareFeedback::Err), SeqCst);
		}

		ui.paint();
	});
}
