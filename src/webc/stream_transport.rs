//! Deadlines around HTTP response headers and each raw body read.
//! Parsing/heartbeats must never change the meaning of transport progress.
use crate::Headers;
use crate::error::BoxError;
use bytes::Bytes;
use futures::{StreamExt, stream::BoxStream};
use std::sync::{Arc, OnceLock};
use std::{fmt, time::Duration};

/// Shared slot filled with the HTTP response headers once a streaming response arrives.
/// The transport writes it, and the `ChatStream` reads it to fill `StreamEnd.captured_response_headers`.
pub(crate) type ResponseHeadersSlot = Arc<OnceLock<Headers>>;

/// The transport phase that exceeded its configured streaming timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamTimeout {
	/// The request did not receive response headers in time.
	Headers,
	/// No raw response body chunk arrived in time.
	Read,
}
impl fmt::Display for StreamTimeout {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Headers => "model response headers timed out",
			Self::Read => "model response body read timed out",
		})
	}
}
impl std::error::Error for StreamTimeout {}

pub(crate) struct StreamRequest {
	request: reqwest::RequestBuilder,
	header_timeout: Option<Duration>,
	read_timeout: Option<Duration>,
	response_headers: Option<ResponseHeadersSlot>,
}
impl StreamRequest {
	pub(crate) fn new(
		request: reqwest::RequestBuilder,
		header_timeout: Option<Duration>,
		read_timeout: Option<Duration>,
	) -> Self {
		Self {
			request,
			header_timeout,
			read_timeout,
			response_headers: None,
		}
	}
	/// Sets the slot that receives the response headers (only set when capture is enabled).
	pub(crate) fn with_response_headers_slot(mut self, slot: Option<ResponseHeadersSlot>) -> Self {
		self.response_headers = slot;
		self
	}
	pub(crate) async fn send(self) -> Result<StreamResponse, BoxError> {
		let response = match self.header_timeout {
			Some(timeout) => tokio::time::timeout(timeout, self.request.send())
				.await
				.map_err(|_| Box::new(StreamTimeout::Headers) as BoxError)?,
			None => self.request.send().await,
		}
		.map_err(|error| Box::new(error) as BoxError)?;
		if let Some(slot) = self.response_headers {
			let _ = slot.set(Headers::from_header_map(response.headers()));
		}
		Ok(StreamResponse {
			response,
			read_timeout: self.read_timeout,
		})
	}
}

pub(crate) struct StreamResponse {
	response: reqwest::Response,
	read_timeout: Option<Duration>,
}
impl StreamResponse {
	pub(crate) fn headers(&self) -> &reqwest::header::HeaderMap {
		self.response.headers()
	}
	pub(crate) fn status(&self) -> reqwest::StatusCode {
		self.response.status()
	}
	pub(crate) fn bytes_stream(self) -> BoxStream<'static, Result<Bytes, BoxError>> {
		let timeout = self.read_timeout;
		let stream = self.response.bytes_stream().boxed();
		futures::stream::unfold(Some(stream), move |state| async move {
			let mut stream = state?;
			let next = match timeout {
				Some(timeout) => match tokio::time::timeout(timeout, stream.next()).await {
					Ok(next) => next,
					Err(_) => return Some((Err(Box::new(StreamTimeout::Read) as BoxError), None)),
				},
				None => stream.next().await,
			};
			match next {
				Some(Ok(bytes)) => Some((Ok(bytes), Some(stream))),
				Some(Err(e)) => Some((Err(Box::new(e) as BoxError), None)),
				None => None,
			}
		})
		.boxed()
	}
	pub(crate) async fn text(self) -> Result<String, BoxError> {
		if self.read_timeout.is_none() {
			return self.response.text().await.map_err(|error| Box::new(error) as BoxError);
		}
		let mut stream = self.bytes_stream();
		let mut bytes = Vec::new();
		while let Some(chunk) = stream.next().await {
			bytes.extend_from_slice(&chunk?);
		}
		Ok(String::from_utf8_lossy(&bytes).into_owned())
	}
}
