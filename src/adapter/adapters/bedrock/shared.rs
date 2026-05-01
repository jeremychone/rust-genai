//! Shared helpers used by both the SigV4 and API-key Bedrock adapters.

use crate::adapter::{AdapterKind, ServiceType};
use crate::resolver::Endpoint;
use crate::{Error, ModelIden, Result};
use reqwest::RequestBuilder;

/// The hostname prefix for the Bedrock runtime endpoint. Regions are interpolated between this
/// prefix and `.amazonaws.com`.
pub(super) const BEDROCK_RUNTIME_HOST_PREFIX: &str = "bedrock-runtime";

/// Curated snapshot of model IDs. Dynamic listing requires the `bedrock` (control plane) API,
/// not `bedrock-runtime`, so we return a hard-coded list here.
pub(super) fn curated_model_names() -> Vec<String> {
	vec![
		"anthropic.claude-sonnet-4-5-20250929-v1:0".to_string(),
		"anthropic.claude-opus-4-1-20250805-v1:0".to_string(),
		"anthropic.claude-haiku-4-5-20251001-v1:0".to_string(),
		"amazon.nova-pro-v1:0".to_string(),
		"amazon.nova-lite-v1:0".to_string(),
		"amazon.nova-micro-v1:0".to_string(),
		"meta.llama3-1-70b-instruct-v1:0".to_string(),
		"mistral.mistral-large-2407-v1:0".to_string(),
	]
}

/// Build the Converse / ConverseStream URL for a given model + service type.
pub(super) fn build_service_url(
	model: &ModelIden,
	service_type: ServiceType,
	endpoint: Endpoint,
	adapter_kind: AdapterKind,
) -> Result<String> {
	let base_url = endpoint.base_url();
	let (_, model_name) = model.model_name.namespace_and_name();
	// Model IDs contain ':' (e.g., anthropic.claude-sonnet-4-5-20250929-v1:0) and must be
	// URL-encoded inside the path segment.
	let encoded = urlencode_path_segment(model_name);

	let url = match service_type {
		ServiceType::Chat => format!("{base_url}model/{encoded}/converse"),
		ServiceType::ChatStream => format!("{base_url}model/{encoded}/converse-stream"),
		ServiceType::Embed => {
			return Err(Error::AdapterNotSupported {
				adapter_kind,
				feature: "embeddings via Converse (use /invoke instead, not yet supported)".to_string(),
			});
		}
	};
	Ok(url)
}

fn urlencode_path_segment(s: &str) -> String {
	let mut out = String::with_capacity(s.len());
	for b in s.bytes() {
		match b {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
			_ => {
				out.push('%');
				out.push_str(&format!("{:02X}", b));
			}
		}
	}
	out
}

/// Turn a `reqwest::RequestBuilder` into a byte stream for the event-stream frame parser.
/// On HTTP error, yields a single error item so the parser surfaces it through the normal
/// error path.
pub(super) fn async_stream_bytes(
	reqwest_builder: RequestBuilder,
) -> impl futures::Stream<Item = std::result::Result<bytes::Bytes, crate::error::BoxError>> + Send {
	use futures::StreamExt;
	async_stream_once(reqwest_builder).flat_map(|result| match result {
		Ok(stream) => stream.boxed(),
		Err(err) => futures::stream::once(async move { Err(err) }).boxed(),
	})
}

fn async_stream_once(
	reqwest_builder: RequestBuilder,
) -> impl futures::Stream<
	Item = std::result::Result<
		futures::stream::BoxStream<'static, std::result::Result<bytes::Bytes, crate::error::BoxError>>,
		crate::error::BoxError,
	>,
> + Send {
	use futures::StreamExt;
	futures::stream::once(async move {
		let resp = reqwest_builder
			.send()
			.await
			.map_err(|e| Box::new(e) as crate::error::BoxError)?;
		let status = resp.status();
		if !status.is_success() {
			let body = resp.text().await.unwrap_or_default();
			let err = crate::Error::HttpError {
				status,
				canonical_reason: status.canonical_reason().unwrap_or("Unknown").to_string(),
				body,
			};
			return Err(Box::new(err) as crate::error::BoxError);
		}
		let bytes: futures::stream::BoxStream<'static, std::result::Result<bytes::Bytes, crate::error::BoxError>> =
			resp.bytes_stream()
				.map(|r| r.map_err(|e| Box::new(e) as crate::error::BoxError))
				.boxed();
		Ok(bytes)
	})
}
