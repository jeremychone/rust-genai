use crate::webc::{Error, Result};
use reqwest::header::HeaderMap;
use reqwest::{Method, RequestBuilder, StatusCode};
use serde_json::Value;

/// A simple reqwest client wrapper for this library.
#[derive(Debug)]
pub struct WebClient {
	reqwest_client: reqwest::Client,
}

// Implements Default
impl Default for WebClient {
	fn default() -> Self {
		WebClient {
			reqwest_client: reqwest::Client::new(),
		}
	}
}

// region:    --- Constructors

impl WebClient {
	pub fn from_reqwest_client(reqwest_client: reqwest::Client) -> Self {
		WebClient { reqwest_client }
	}
}

// endregion: --- Constructors

// region:    --- Web Method Implementation

impl WebClient {
	pub async fn do_get(&self, url: &str, headers: &[(String, String)]) -> Result<WebResponse> {
		let mut reqwest_builder = self.reqwest_client.request(Method::GET, url);

		for (k, v) in headers.iter() {
			reqwest_builder = reqwest_builder.header(k, v);
		}
		let reqwest_res = reqwest_builder.send().await?;

		let response = WebResponse::from_reqwest_response(reqwest_res).await?;

		Ok(response)
	}

	pub async fn do_post(&self, url: &str, headers: &[(String, String)], content: Value) -> Result<WebResponse> {
		let reqwest_builder = self.new_req_builder(url, headers, content)?;

		let reqwest_res = reqwest_builder.send().await?;

		let response = WebResponse::from_reqwest_response(reqwest_res).await?;

		Ok(response)
	}

	pub fn new_req_builder(&self, url: &str, headers: &[(String, String)], content: Value) -> Result<RequestBuilder> {
		let method = Method::POST;

		let mut reqwest_builder = self.reqwest_client.request(method, url);
		for (k, v) in headers.iter() {
			reqwest_builder = reqwest_builder.header(k, v);
		}
		reqwest_builder = reqwest_builder.json(&content);

		Ok(reqwest_builder)
	}
}
// endregion: --- Web Method Implementation

// region:    --- WebResponse

// NOTE: This is not a non-streaming web response (assumed to be JSON for this library).
//       Streaming is handled with event-source or custom streams (for example, for Cohere).

#[derive(Debug)]
pub struct WebResponse {
	#[allow(unused)]
	pub status: StatusCode,
	pub body: Value,
}

impl WebResponse {
	/// Note 1: For now, assume only a JSON response.
	/// Note 2: Currently, the WebResponse holds a Value (parsed from the entire body), and then the caller
	///         can cherry-pick/deserialize further. In the future, we might consider returning `body: String`
	///         to enable more optimized parsing, allowing for selective parsing constrained by the structure.
	pub(crate) async fn from_reqwest_response(mut res: reqwest::Response) -> Result<WebResponse> {
		let status = res.status();

		if !status.is_success() {
			let body = res.text().await?;
			return Err(Error::ResponseFailedStatus { status, body });
		}

		// Move the headers into a new HeaderMap
		let headers = res.headers_mut().drain().filter_map(|(n, v)| n.map(|n| (n, v)));
		let header_map = HeaderMap::from_iter(headers);

		// Capture the body
		let ct = header_map.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default();
		let body = if ct.starts_with("application/json") {
			res.json::<Value>().await?
		} else {
			return Err(Error::ResponseFailedNotJson {
				content_type: ct.to_string(),
			});
		};

		Ok(WebResponse { status, body })
	}
}

// endregion: --- WebResponse
