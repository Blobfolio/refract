/*!
# `Refract GTK` - Sharing
*/

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
	sync::Arc,
};



/// # Payload Type.
pub(super) type SharePayload = Result<Share, RefractError>;

/// # Main Thread Receiver.
type MainRx = Receiver<SharePayload>;

/// # Main Thread Sender.
pub(super) type MainTx = Sender<ShareFeedback>;

/// # Sister Thread Receiver.
pub(super) type SisterRx = Receiver<ShareFeedback>;

/// # Sister Thread Sender.
pub(super) type SisterTx = Sender<SharePayload>;



thread_local!(
	/// # Global.
	///
	/// This gives us a way to reach the main thread from a sister thread.
	static GLOBAL: RefCell<Option<(Arc<Window>, MainRx, MainTx)>> = RefCell::new(None);
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
	-> (SisterTx, MainTx, SisterRx) {
		let (tx, rx) = crossbeam_channel::bounded(8);
		let (tx2, rx2) = crossbeam_channel::bounded(8);
		GLOBAL.with(|global| {
			*global.borrow_mut() = Some((window, rx, tx2.clone()));
		});

		(tx, tx2, rx2)
	}

	/// # Sync Share.
	///
	/// This pushes a payload to the main thread, then optionally waits for and
	/// returns the response.
	///
	/// When not waiting for a response, [`ShareFeedback::Continue`] is returned
	/// immediately.
	pub(super) fn sync(tx: &SisterTx, rx: &SisterRx, share: SharePayload)
	-> ShareFeedback {
		tx.send(share).unwrap();
		glib::source::idle_add(|| {
			get_share();
			glib::source::Continue(false)
		});

		loop {
			let res = rx.recv().unwrap_or(ShareFeedback::Abort);
			if res != ShareFeedback::Wait { return res; }
		}
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
	Continue,
	Abort,
	Discard,
	Keep,
	Wait,
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
		let (ui, rx, tx) = ptr.as_ref()
			.expect("Missing main thread state.");

		tx.send(
			if let Ok(res) = rx.recv() {
				ui.process_share(res).unwrap_or(ShareFeedback::Abort)
			}
			else { ShareFeedback::Abort }
		).unwrap();

		ui.paint();
	});
}
