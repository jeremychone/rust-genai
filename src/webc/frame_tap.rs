//! Internal plumbing that feeds a user-provided [`ChatFrameSink`] from the transports.
//!
//! Provider specific streamers use this implementation to send frames to the sink.

use crate::chat::{ChatFrameSink, FrameCtx, RawFrameRef, StreamEnd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Feeds decoded frames to a sink.
///
/// Always use [`FrameTap::on_frame`] instead of calling the sink directly, so the frame
/// index stays contiguous.
///
/// The terminal hooks are latched: across all clones of a tap, at most one of
/// [`FrameTap::on_stream_end`] / [`FrameTap::on_error`] ever reaches the sink. The transports
/// keep polling after an error (they do not set their `done` flag on every error path), so
/// without the latch a stream that reports an error and then completes would deliver several
/// terminal callbacks for the same stream.
#[derive(Debug, Clone)]
pub(crate) struct FrameTap {
	sink: Arc<dyn ChatFrameSink>,
	ctx: FrameCtx,
	next_index: u32,
	start: Instant,
	/// Shared across clones: set by whichever terminal hook fires first.
	terminated: Arc<AtomicBool>,
}

impl FrameTap {
	pub(crate) fn new(sink: Arc<dyn ChatFrameSink>, ctx: FrameCtx) -> Self {
		Self {
			sink,
			ctx,
			next_index: 0,
			start: Instant::now(),
			terminated: Arc::new(AtomicBool::new(false)),
		}
	}

	/// Emits one frame, assigning the next index and the elapsed time.
	pub(crate) fn on_frame(&mut self, event: Option<&str>, data: &str) {
		let index = self.next_index;
		self.next_index += 1;

		let frame = RawFrameRef {
			index,
			event,
			data,
			elapsed_us: self.start.elapsed().as_micros() as u64,
		};

		self.sink.on_frame(&self.ctx, frame);
	}

	/// Claims the terminal slot. `true` for the first caller only.
	fn claim_terminal(&self) -> bool {
		self.terminated
			.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
			.is_ok()
	}

	/// Fires `on_end`, unless a terminal hook already fired for this stream.
	pub(crate) fn on_stream_end(&self, end: &StreamEnd) {
		if self.claim_terminal() {
			self.sink.on_end(&self.ctx, end);
		}
	}

	/// Fires `on_error`, unless a terminal hook already fired for this stream.
	pub(crate) fn on_error(&self, err: &crate::Error) {
		if self.claim_terminal() {
			self.sink.on_error(&self.ctx, err);
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::ModelIden;
	use crate::adapter::AdapterKind;
	use std::sync::atomic::AtomicUsize;

	#[derive(Debug, Default)]
	struct TerminalSink {
		ends: AtomicUsize,
		errors: AtomicUsize,
	}

	impl ChatFrameSink for TerminalSink {
		fn on_frame(&self, _ctx: &FrameCtx, _frame: RawFrameRef<'_>) {}

		fn on_end(&self, _ctx: &FrameCtx, _end: &StreamEnd) {
			self.ends.fetch_add(1, Ordering::Relaxed);
		}

		fn on_error(&self, _ctx: &FrameCtx, _err: &crate::Error) {
			self.errors.fetch_add(1, Ordering::Relaxed);
		}
	}

	fn new_tap(sink: Arc<TerminalSink>) -> FrameTap {
		let ctx = FrameCtx::new(ModelIden::new(AdapterKind::OpenAI, "gpt-test"));
		FrameTap::new(sink, ctx)
	}

	fn some_error() -> crate::Error {
		crate::Error::ChatResponse {
			model_iden: ModelIden::new(AdapterKind::OpenAI, "gpt-test"),
			body: serde_json::json!({"message": "boom"}),
		}
	}

	#[test]
	fn test_frame_tap_terminal_latch_error_wins_over_later_end() -> Result<()> {
		// -- Setup & Fixtures
		let sink = Arc::new(TerminalSink::default());
		let tap = new_tap(sink.clone());

		// -- Exec
		// The transports keep polling after an error, so a stream can report several errors
		// and still reach its `[DONE]` frame.
		tap.on_error(&some_error());
		tap.on_error(&some_error());
		tap.on_stream_end(&StreamEnd::default());

		// -- Check
		assert_eq!(sink.errors.load(Ordering::Relaxed), 1, "on_error should fire once");
		assert_eq!(
			sink.ends.load(Ordering::Relaxed),
			0,
			"on_end should not fire after on_error"
		);

		Ok(())
	}

	#[test]
	fn test_frame_tap_terminal_latch_end_wins_over_later_error() -> Result<()> {
		// -- Setup & Fixtures
		let sink = Arc::new(TerminalSink::default());
		let tap = new_tap(sink.clone());

		// -- Exec
		tap.on_stream_end(&StreamEnd::default());
		tap.on_stream_end(&StreamEnd::default());
		tap.on_error(&some_error());

		// -- Check
		assert_eq!(sink.ends.load(Ordering::Relaxed), 1, "on_end should fire once");
		assert_eq!(
			sink.errors.load(Ordering::Relaxed),
			0,
			"on_error should not fire after on_end"
		);

		Ok(())
	}

	#[test]
	fn test_frame_tap_terminal_latch_shared_across_clones() -> Result<()> {
		// -- Setup & Fixtures
		// `ChatStream` fires the terminal hooks through a clone of the streamer's tap.
		let sink = Arc::new(TerminalSink::default());
		let tap = new_tap(sink.clone());
		let cloned_tap = tap.clone();

		// -- Exec
		tap.on_error(&some_error());
		cloned_tap.on_stream_end(&StreamEnd::default());

		// -- Check
		assert_eq!(sink.errors.load(Ordering::Relaxed), 1);
		assert_eq!(sink.ends.load(Ordering::Relaxed), 0, "the latch is shared with clones");

		Ok(())
	}

	#[test]
	fn test_frame_tap_frame_indices_stay_contiguous() -> Result<()> {
		// -- Setup & Fixtures
		let sink = Arc::new(crate::chat::CollectorSink::new());
		let ctx = FrameCtx::new(ModelIden::new(AdapterKind::OpenAI, "gpt-test"));
		let mut tap = FrameTap::new(sink.clone(), ctx);

		// -- Exec
		tap.on_frame(Some("a"), "{}");
		tap.on_frame(None, "[DONE]");

		// -- Check
		assert_eq!(sink.frames().iter().map(|f| f.index).collect::<Vec<_>>(), vec![0, 1]);

		Ok(())
	}
}

// endregion: --- Tests
