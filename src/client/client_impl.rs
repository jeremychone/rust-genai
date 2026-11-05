use crate::adapter::{AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::client::ModelSpec;
use crate::embed::{EmbedOptions, EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, ProviderConfig};
use crate::{Client, Error, ModelIden, Result, ServiceTarget};

/// High-level client APIs.
impl Client {
	/// Lists model names for the given adapter.
	///
	/// Notes:
	///
	/// - Non-Ollama adapters use a static list.
	///
	/// - Ollama queries the default host (http://localhost:11434/v1/).
	///
	/// - May evolve to accept a custom endpoint.
	///
	/// - Accepts [`ProviderConfig`] or compatible values to override endpoint and auth.
	///
	/// - For most adapters, names also drive AdapterKind detection (see [`AdapterKind`]).
	///
	/// - Adapters should filter non-chat models until more skills are supported.
	///   Future: `model_names(adapter_kind, Option<&[Skill]>)`.
	pub async fn all_model_names(
		&self,
		adapter_kind: AdapterKind,
		provider_config: impl Into<ProviderConfig>,
	) -> Result<Vec<String>> {
		let ProviderConfig { endpoint, auth } = provider_config.into();

		let (auth, endpoint) = match (auth, endpoint) {
			(Some(auth), Some(endpoint)) => (auth, endpoint),
			(auth, endpoint) => {
				let (default_auth, default_endpoint) = self.config().resolve_adapter_config(adapter_kind).await?;
				(auth.unwrap_or(default_auth), endpoint.unwrap_or(default_endpoint))
			}
		};

		let models = AdapterDispatcher::all_model_names(adapter_kind, endpoint, auth, self.web_client()).await?;
		Ok(models)
	}

	/// Returns the bound [`AdapterKind`] for this Client, if any.
	///
	/// Set via [`ClientBuilder::with_adapter_kind`] at construction
	/// time — `None` for Clients that did not opt into the bound
	/// shape and still use per-call model-name inference. Useful for
	/// callers that want to introspect a Client without tracking the
	/// provider in parallel state.
	///
	/// [`ClientBuilder::with_adapter_kind`]: crate::ClientBuilder::with_adapter_kind
	pub fn adapter_kind(&self) -> Option<AdapterKind> {
		self.config().adapter_kind()
	}

	/// Builds a ModelIden by inferring AdapterKind from the model name.
	pub fn default_model(&self, model_name: &str) -> Result<ModelIden> {
		// -- First get the default ModelInfo
		let adapter_kind = AdapterKind::from_model(model_name)?;
		let model_iden = ModelIden::new(adapter_kind, model_name);
		Ok(model_iden)
	}

	/// Deprecated: use `Client::resolve_service_target`.
	#[deprecated(note = "use `client.resolve_service_target(model_name)`")]
	pub async fn resolve_model_iden(&self, model_name: &str) -> Result<ModelIden> {
		let model = self.default_model(model_name)?;
		let target = self.config().resolve_service_target(model).await?;
		Ok(target.model)
	}

	/// Resolves the service target (endpoint, auth, and model) for the given model.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn resolve_service_target(&self, model: impl Into<ModelSpec>) -> Result<ServiceTarget> {
		self.config().resolve_model_spec(model.into()).await
	}

	/// Sends a chat request and returns the full response.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn exec_chat(
		&self,
		model: impl Into<ModelSpec>,
		chat_req: ChatRequest,
		options: Option<&ChatOptions>,
	) -> Result<ChatResponse> {
		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let target = self.config().resolve_model_spec(model.into()).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		// -- OTel: create the `chat` span (borrows end before `target`/`chat_req` are consumed below).
		#[cfg(feature = "otel")]
		let otel_span = crate::otel::span::chat_request_span(&model, &target.endpoint, &options_set, &chat_req, false);

		// The operation is wrapped so every error path (request shaping, web call, response
		// parsing) flows through the OTel span recorder below.
		let result = async {
			let WebRequestData {
				mut url,
				mut headers,
				payload,
			} = AdapterDispatcher::to_web_request_data(target, ServiceType::Chat, chat_req, options_set.clone())?;

			if let Some(extra_headers) = self.config().chat_options().and_then(|o| o.extra_headers.as_ref()) {
				headers.merge_with(extra_headers);
			}
			if let Some(extra_headers) = options.and_then(|o| o.extra_headers.as_ref()) {
				headers.merge_with(extra_headers);
			}

			if let AuthData::RequestOverride {
				url: override_url,
				headers: override_headers,
			} = auth_data
			{
				url = override_url;
				headers = override_headers;
			};

			let web_res = self
				.web_client()
				.do_post(&url, &headers, &payload)
				.await
				.map_err(|webc_error| Error::WebModelCall {
					model_iden: model.clone(),
					webc_error,
				})?;

			// Note: here we capture/clone the raw body if set in the options_set
			let captured_raw_body = options_set.capture_raw_body().unwrap_or_default().then(|| web_res.body.clone());

			match AdapterDispatcher::to_chat_response(model.clone(), web_res, options_set) {
				Ok(mut chat_res) => {
					chat_res.captured_raw_body = captured_raw_body;
					Ok(chat_res)
				}
				Err(err) => {
					let response_body = captured_raw_body.unwrap_or_else(|| {
						"Raw response not captured. Use the ChatOptions.capturre_raw_body flag to see raw response in this error".into()
					});
					let err = Error::ChatResponseGeneration {
						model_iden: model,
						request_payload: Box::new(payload),
						response_body: Box::new(response_body),
						cause: err.to_string(),
					};
					Err(err)
				}
			}
		}
		.await;

		#[cfg(feature = "otel")]
		let result = crate::otel::span::record_chat_result(&otel_span, result);

		result
	}

	/// Streams a chat response.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn exec_chat_stream(
		&self,
		model: impl Into<ModelSpec>,
		chat_req: ChatRequest,
		options: Option<&ChatOptions>,
	) -> Result<ChatStreamResponse> {
		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let target = self.config().resolve_model_spec(model.into()).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		// -- OTel: create the streaming `chat` span (borrows end before `target`/`chat_req` are consumed).
		#[cfg(feature = "otel")]
		let otel_span = crate::otel::span::chat_request_span(&model, &target.endpoint, &options_set, &chat_req, true);

		// Stream setup is synchronous; wrap it so setup errors are recorded on the span too.
		let result = (move || {
			let WebRequestData {
				mut url,
				mut headers,
				payload,
			} = AdapterDispatcher::to_web_request_data(target, ServiceType::ChatStream, chat_req, options_set.clone())?;

			if let Some(extra_headers) = self.config().chat_options().and_then(|o| o.extra_headers.as_ref()) {
				headers.merge_with(extra_headers);
			}
			if let Some(extra_headers) = options.and_then(|o| o.extra_headers.as_ref()) {
				headers.merge_with(extra_headers);
			}

			// TODO: Need to check this.
			//       This was part of the 429c5cee2241dbef9f33699b9c91202233c22816 commit
			//       But now it is missing in the the exec_chat(..) above, which is probably an issue.
			if let AuthData::RequestOverride {
				url: override_url,
				headers: override_headers,
			} = auth_data
			{
				url = override_url;
				headers = override_headers;
			};

			let reqwest_builder =
				self.web_client()
					.new_req_builder(&url, &headers, &payload)
					.map_err(|webc_error| Error::WebModelCall {
						model_iden: model.clone(),
						webc_error,
					})?;

			let defaults = self.config().chat_options();
			let header_timeout = options
				.and_then(|o| o.stream_header_timeout)
				.or_else(|| defaults.and_then(|o| o.stream_header_timeout));
			let read_timeout = options
				.and_then(|o| o.stream_read_timeout)
				.or_else(|| defaults.and_then(|o| o.stream_read_timeout));
			let reqwest_builder = crate::webc::StreamRequest::new(reqwest_builder, header_timeout, read_timeout);
			let res = AdapterDispatcher::to_chat_stream(model, reqwest_builder, options_set)?;

			Ok(res)
		})();

		match result {
			Ok(res) => {
				// Hand the span to the stream so it stays open for the stream's lifetime.
				#[cfg(feature = "otel")]
				let res = ChatStreamResponse {
					stream: res.stream.with_otel_span(otel_span),
					..res
				};
				Ok(res)
			}
			Err(err) => {
				#[cfg(feature = "otel")]
				crate::otel::span::record_error(&otel_span, &err);
				Err(err)
			}
		}
	}

	/// Creates embeddings for a single input string.
	///
	/// Accepts any type that implements `Into<ModelSpec>` for the model parameter.
	pub async fn embed(
		&self,
		model: impl Into<ModelSpec>,
		input: impl Into<String>,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let embed_req = EmbedRequest::new(input);
		self.exec_embed(model, embed_req, options).await
	}

	/// Creates embeddings for multiple input strings.
	///
	/// Accepts any type that implements `Into<ModelSpec>` for the model parameter.
	pub async fn embed_batch(
		&self,
		model: impl Into<ModelSpec>,
		inputs: Vec<String>,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let embed_req = EmbedRequest::new_batch(inputs);
		self.exec_embed(model, embed_req, options).await
	}

	/// Sends an embedding request and returns the response.
	///
	/// Accepts any type that implements `Into<ModelSpec>`:
	/// - `&str` or `String`: Model name with full inference
	/// - `ModelIden`: Explicit adapter, resolves auth/endpoint
	/// - `ServiceTarget`: Uses directly, bypasses model mapping and auth resolution
	pub async fn exec_embed(
		&self,
		model: impl Into<ModelSpec>,
		embed_req: EmbedRequest,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let options_set = EmbedOptionsSet::new()
			.with_request_options(options)
			.with_client_options(self.config().embed_options());

		let target = self.config().resolve_model_spec(model.into()).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		// -- OTel: create the `embeddings` span (borrow ends before `target` is consumed below).
		#[cfg(feature = "otel")]
		let otel_span = crate::otel::span::embeddings_request_span(&model, &target.endpoint, &options_set);

		let result = async {
			let WebRequestData {
				mut headers,
				payload,
				mut url,
			} = AdapterDispatcher::to_embed_request_data(target, embed_req, options_set.clone())?;

			if let Some(extra_headers) = self.config().embed_options().and_then(|o| o.headers()) {
				headers.merge_with(extra_headers);
			}
			if let Some(extra_headers) = options.and_then(|o| o.headers()) {
				headers.merge_with(extra_headers);
			}

			if let AuthData::RequestOverride {
				url: override_url,
				headers: override_headers,
			} = auth_data
			{
				url = override_url;
				headers = override_headers;
			};

			let web_res = self
				.web_client()
				.do_post(&url, &headers, &payload)
				.await
				.map_err(|webc_error| Error::WebModelCall {
					model_iden: model.clone(),
					webc_error,
				})?;

			let res = AdapterDispatcher::to_embed_response(model, web_res, options_set)?;

			Ok(res)
		}
		.await;

		#[cfg(feature = "otel")]
		let result = crate::otel::span::record_embed_result(&otel_span, result);

		result
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::resolver::Endpoint;
	use crate::{ClientConfig, Headers};
	use std::io::{Read, Write};
	use std::net::{Shutdown, TcpListener};
	use std::sync::{Arc, Mutex};

	fn serve_json(response_body: String, captured_request: Arc<Mutex<String>>) -> u16 {
		let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
		let port = listener.local_addr().expect("addr").port();
		std::thread::spawn(move || {
			let (mut stream, _) = listener.accept().expect("accept");
			let mut buf = [0_u8; 8192];
			let n = stream.read(&mut buf).unwrap_or(0);
			*captured_request.lock().expect("lock") = String::from_utf8_lossy(&buf[..n]).to_string();
			let header = format!(
				"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
				response_body.len()
			);
			stream.write_all(header.as_bytes()).expect("header");
			stream.write_all(response_body.as_bytes()).expect("body");
			let _ = stream.shutdown(Shutdown::Both);
		});
		port
	}

	#[tokio::test]
	async fn test_exec_embed_applies_request_override() {
		let captured = Arc::new(Mutex::new(String::new()));
		let body = serde_json::json!({
			"data": [{"embedding": [0.1, 0.2], "index": 0}],
			"model": "text-embedding-3-small",
			"usage": {"prompt_tokens": 3, "total_tokens": 3}
		})
		.to_string();
		let port = serve_json(body, captured.clone());

		let client = Client::builder().build().expect("client");
		let target = ServiceTarget {
			endpoint: Endpoint::from_static("http://127.0.0.1:1/unused/"),
			auth: AuthData::RequestOverride {
				url: format!("http://127.0.0.1:{port}/custom-embed-path"),
				headers: Headers::from(("x-custom-auth", "override-secret")),
			},
			model: ModelIden::new(AdapterKind::OpenAI, "text-embedding-3-small"),
		};

		let res = client
			.exec_embed(target, EmbedRequest::new("hello"), None)
			.await
			.expect("embed should succeed via RequestOverride");

		assert_eq!(res.embeddings.len(), 1);
		let raw_req = captured.lock().expect("lock").clone();
		assert!(
			raw_req.starts_with("POST /custom-embed-path HTTP/1.1"),
			"unexpected request line: {raw_req}"
		);
		assert!(
			raw_req.to_ascii_lowercase().contains("x-custom-auth: override-secret"),
			"missing override header in: {raw_req}"
		);
		assert!(
			!raw_req.to_ascii_lowercase().contains("authorization:"),
			"override headers should replace adapter default Authorization header: {raw_req}"
		);
	}

	#[tokio::test]
	async fn test_exec_embed_merges_client_and_request_headers() {
		let captured = Arc::new(Mutex::new(String::new()));
		let body = serde_json::json!({
			"data": [{"embedding": [0.1, 0.2], "index": 0}],
			"model": "text-embedding-3-small",
			"usage": {"prompt_tokens": 3, "total_tokens": 3}
		})
		.to_string();
		let port = serve_json(body, captured.clone());

		let client_embed_opts = EmbedOptions::new().with_headers(Headers::from([
			("x-client-only", "from-client"),
			("x-shared", "client"),
		]));
		let config = ClientConfig::default().with_embed_options(client_embed_opts);
		let client = Client::builder().with_config(config).build().expect("client");

		let target = ServiceTarget {
			endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1/")),
			auth: AuthData::from_single("test-key"),
			model: ModelIden::new(AdapterKind::OpenAI, "text-embedding-3-small"),
		};
		let req_opts =
			EmbedOptions::new().with_headers(Headers::from([("x-req-only", "from-req"), ("x-shared", "request")]));

		let _res = client
			.exec_embed(target, EmbedRequest::new("hello"), Some(&req_opts))
			.await
			.expect("embed should succeed");

		let raw_req = captured.lock().expect("lock").to_ascii_lowercase();
		assert!(raw_req.contains("x-client-only: from-client"), "raw_req: {raw_req}");
		assert!(raw_req.contains("x-req-only: from-req"), "raw_req: {raw_req}");
		assert!(raw_req.contains("x-shared: request"), "raw_req: {raw_req}");
	}
}
