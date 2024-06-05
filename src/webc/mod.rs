// region:    --- Modules

mod error;

pub use self::error::{Error, Result};

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Method, RequestBuilder, StatusCode};
use reqwest_eventsource::EventSource;
use serde_json::Value;

// endregion: --- Modules
#[derive(Debug)]
pub struct WebClient {
	reqwest_client: reqwest::Client,
	// TODO: to deprecate
	base_url: Option<String>,
}

// region:    --- Constructors

impl WebClient {
	pub(crate) fn new() -> Self {
		Self {
			reqwest_client: reqwest::Client::new(),
			base_url: None,
		}
	}

	pub(crate) fn base_url(mut self, base_url: impl Into<String>) -> Self {
		self.base_url = Some(base_url.into());
		self
	}
}

// endregion: --- Constructors

// region:    --- Web Method Impl

impl WebClient {
	pub async fn do_post(&self, url: &str, headers: &[(String, String)], content: Value) -> Result<WebResponse> {
		let reqwest_builder = self.req_builder(url, headers, content)?;

		let reqwest_res = reqwest_builder.send().await?;

		let response = WebResponse::from_reqwest_response(reqwest_res).await?;

		Ok(response)
	}

	pub async fn do_post_stream(&self, url: &str, headers: &[(String, String)], content: Value) -> Result<EventSource> {
		let reqwest_builder = self.req_builder(url, headers, content)?;

		let es = EventSource::new(reqwest_builder)?;

		Ok(es)
	}

	pub fn req_builder(&self, url: &str, headers: &[(String, String)], content: Value) -> Result<RequestBuilder> {
		let url = self.compose_url(url);
		let method = Method::POST;

		let mut reqwest_builder = self.reqwest_client.request(method.clone(), url);
		for (k, v) in headers.iter() {
			reqwest_builder = reqwest_builder.header(k, v);
		}
		reqwest_builder = reqwest_builder.json(&content);

		Ok(reqwest_builder)
	}

	fn compose_url(&self, url: &str) -> String {
		match &self.base_url {
			Some(base_url) => format!("{base_url}{url}"),
			None => url.to_string(),
		}
	}
}
// endregion: --- Web Method Impl

// region:    --- PostContent

pub enum PostContent {
	Json(Value),
	Text { body: String, content_type: &'static str },
}
impl From<Value> for PostContent {
	fn from(val: Value) -> Self {
		PostContent::Json(val)
	}
}
impl From<String> for PostContent {
	fn from(val: String) -> Self {
		PostContent::Text {
			content_type: "text/plain",
			body: val,
		}
	}
}
impl From<&String> for PostContent {
	fn from(val: &String) -> Self {
		PostContent::Text {
			content_type: "text/plain",
			body: val.to_string(),
		}
	}
}

impl From<&str> for PostContent {
	fn from(val: &str) -> Self {
		PostContent::Text {
			content_type: "text/plain",
			body: val.to_string(),
		}
	}
}

impl From<(String, &'static str)> for PostContent {
	fn from((body, content_type): (String, &'static str)) -> Self {
		PostContent::Text { body, content_type }
	}
}

impl From<(&str, &'static str)> for PostContent {
	fn from((body, content_type): (&str, &'static str)) -> Self {
		PostContent::Text {
			body: body.to_string(),
			content_type,
		}
	}
}

// endregion: --- Post Body

// endregion: --- PostContent

// region:    --- Response
#[derive(Debug)]
pub struct WebResponse {
	pub status: StatusCode,
	pub body: Value,
}

impl WebResponse {
	/// Note: For now, assume only json response
	pub(crate) async fn from_reqwest_response(mut res: reqwest::Response) -> Result<WebResponse> {
		let status = res.status();

		// Move the headers into a new HeaderMap
		let headers = res.headers_mut().drain().filter_map(|(n, v)| n.map(|n| (n, v)));
		let header_map = HeaderMap::from_iter(headers);

		// Capture the body
		let ct = header_map.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default();
		let body = if ct.starts_with("application/json") {
			res.json::<Value>().await?
		} else {
			return Err(Error::ReqwestResponseNotJson {
				content_type: ct.to_string(),
			});
		};

		Ok(WebResponse { status, body })
	}
}
// endregion: --- Response
