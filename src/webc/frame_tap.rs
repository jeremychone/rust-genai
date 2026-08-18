//! Internal plumbing that feeds a user-provided [`ChatFrameSink`] from the transports.
//!
//! Provider specific streamers use this implementation to send frames to the sink

use crate::chat::{ChatFrameSink, FrameCtx, RawFrameRef, StreamEnd};
use std::sync::Arc;
use std::time::Instant;

/// Feeds decoded frames to a sink.
///
/// Always use [`FrameTap::on_frame`] instead of directly calling the sink to maintain
/// a counter
#[derive(Debug, Clone)]
pub(crate) struct FrameTap {
	sink: Arc<dyn ChatFrameSink>,
	ctx: FrameCtx,
	next_index: u32,
	start: Instant,
}

impl FrameTap {
	pub(crate) fn new(sink: Arc<dyn ChatFrameSink>, ctx: FrameCtx) -> Self {
		Self {
			sink,
			ctx,
			next_index: 0,
			start: Instant::now(),
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

	pub(crate) fn on_stream_end(&self, end: &StreamEnd) {
		self.sink.on_end(&self.ctx, end);
	}

	pub(crate) fn on_error(&self, err: &crate::Error) {
		self.sink.on_error(&self.ctx, err);
	}
}
