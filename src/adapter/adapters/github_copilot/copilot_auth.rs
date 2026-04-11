use crate::adapter::adapters::github_copilot::copilot_types::{
	COPILOT_CLIENT_ID, COPILOT_DEFAULT_ENDPOINT, COPILOT_EDITOR_PLUGIN_VERSION, COPILOT_EDITOR_VERSION,
	COPILOT_INTEGRATION_ID, COPILOT_SCOPE, COPILOT_TOKEN_EXCHANGE_URL, COPILOT_USER_AGENT, CopilotSessionToken,
	DeviceCodeRequest, DeviceCodeResponse, GITHUB_DEVICE_CODE_URL, GITHUB_OAUTH_TOKEN_URL, OAuthPollError,
	OAuthTokenRequest, OAuthTokenResponse, PersistedOAuthToken, TOKEN_REFRESH_BUFFER_SECS,
};
use crate::adapter::adapters::github_copilot::{CopilotAuthCallback, CopilotTokenStore};
use crate::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use crate::{Error, ServiceTarget};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::sleep;

// region:    --- Device Flow

/// URL overrides for testability — allows tests to redirect requests to mock servers.
struct DeviceFlowUrls<'a> {
	device_code_url: &'a str,
	oauth_token_url: &'a str,
}

#[cfg(test)]
#[derive(Clone)]
struct TestDeviceFlowUrls {
	device_code_url: String,
	oauth_token_url: String,
}

impl<'a> Default for DeviceFlowUrls<'a> {
	fn default() -> Self {
		Self {
			device_code_url: GITHUB_DEVICE_CODE_URL,
			oauth_token_url: GITHUB_OAUTH_TOKEN_URL,
		}
	}
}

async fn device_flow_login(
	client: &reqwest::Client,
	callback: &dyn CopilotAuthCallback,
) -> crate::Result<OAuthTokenResponse> {
	device_flow_login_with_urls(client, callback, DeviceFlowUrls::default()).await
}

async fn device_flow_login_with_urls(
	client: &reqwest::Client,
	callback: &dyn CopilotAuthCallback,
	urls: DeviceFlowUrls<'_>,
) -> crate::Result<OAuthTokenResponse> {
	let response = client
		.post(urls.device_code_url)
		.header("User-Agent", COPILOT_USER_AGENT)
		.header("Accept", "application/json")
		.header("Content-Type", "application/json")
		.json(&DeviceCodeRequest::new(COPILOT_CLIENT_ID, COPILOT_SCOPE))
		.send()
		.await
		.map_err(|err| Error::Internal(format!("Device code request failed: {err}")))?;

	let status = response.status();
	let body = response
		.text()
		.await
		.map_err(|err| Error::Internal(format!("Device code response body read failed: {err}")))?;

	if !status.is_success() {
		return Err(Error::Internal(format!(
			"Device code request failed: status={status}, body={}",
			sanitize_body_for_error(&body)
		)));
	}

	let device_code = serde_json::from_str::<DeviceCodeResponse>(&body).map_err(|err| {
		Error::Internal(format!(
			"Device code response parse failed: {err}; body={}",
			sanitize_body_for_error(&body)
		))
	})?;

	callback.on_device_code(&device_code.user_code, &device_code.verification_uri);

	let mut interval = device_code.interval;

	loop {
		sleep(Duration::from_secs(interval)).await;

		let response = client
			.post(urls.oauth_token_url)
			.header("User-Agent", COPILOT_USER_AGENT)
			.header("Accept", "application/json")
			.header("Content-Type", "application/json")
			.json(&OAuthTokenRequest::new(
				COPILOT_CLIENT_ID,
				device_code.device_code.clone(),
			))
			.send()
			.await
			.map_err(|err| Error::Internal(format!("OAuth token poll request failed: {err}")))?;

		let status = response.status();
		let body = response
			.text()
			.await
			.map_err(|err| Error::Internal(format!("OAuth token poll response body read failed: {err}")))?;

		if !status.is_success() {
			return Err(Error::Internal(format!(
				"OAuth token poll failed: status={status}, body={}",
				sanitize_body_for_error(&body)
			)));
		}

		let value: serde_json::Value = serde_json::from_str(&body).map_err(|err| {
			Error::Internal(format!(
				"OAuth token poll response parse failed: {err}; body={}",
				sanitize_body_for_error(&body)
			))
		})?;

		if value.get("access_token").is_some() {
			let token = serde_json::from_value::<OAuthTokenResponse>(value).map_err(|err| {
				Error::Internal(format!(
					"OAuth token success parse failed: {err}; body={}",
					sanitize_body_for_error(&body)
				))
			})?;
			return Ok(token);
		}

		if value.get("error").is_some() {
			let poll_error = serde_json::from_value::<OAuthPollError>(value).map_err(|err| {
				Error::Internal(format!(
					"OAuth token error parse failed: {err}; body={}",
					sanitize_body_for_error(&body)
				))
			})?;

			match poll_error.error.as_str() {
				"authorization_pending" => continue,
				"slow_down" => {
					interval += 5;
					continue;
				}
				"expired_token" => {
					return Err(Error::Internal(format!(
						"OAuth device flow expired_token: {}",
						poll_error
							.error_description
							.unwrap_or_else(|| "device code expired before authorization completed".to_string())
					)));
				}
				"access_denied" => {
					return Err(Error::Internal(format!(
						"OAuth device flow access_denied: {}",
						poll_error
							.error_description
							.unwrap_or_else(|| "user denied access to GitHub Copilot authorization".to_string())
					)));
				}
				other => {
					return Err(Error::Internal(format!(
						"OAuth device flow polling failed: error={other}, description={}",
						poll_error.error_description.unwrap_or_default()
					)));
				}
			}
		}

		return Err(Error::Internal(format!(
			"OAuth token poll response missing expected fields: {}",
			sanitize_body_for_error(&body)
		)));
	}
}

// endregion: --- Device Flow

// region:    --- Token Exchange

async fn exchange_for_session_token(client: &reqwest::Client, oauth_token: &str) -> crate::Result<CopilotSessionToken> {
	exchange_for_session_token_with_url(client, oauth_token, COPILOT_TOKEN_EXCHANGE_URL).await
}

async fn exchange_for_session_token_with_url(
	client: &reqwest::Client,
	oauth_token: &str,
	url: &str,
) -> crate::Result<CopilotSessionToken> {
	let response = client
		.get(url)
		.header("Authorization", format!("token {oauth_token}"))
		.header("User-Agent", COPILOT_USER_AGENT)
		.header("Editor-Version", COPILOT_EDITOR_VERSION)
		.header("Editor-Plugin-Version", COPILOT_EDITOR_PLUGIN_VERSION)
		.header("Copilot-Integration-Id", COPILOT_INTEGRATION_ID)
		.send()
		.await
		.map_err(|err| Error::Internal(format!("Token exchange request failed: {err}")))?;

	let status = response.status();
	let body = response
		.text()
		.await
		.map_err(|err| Error::Internal(format!("Token exchange response body read failed: {err}")))?;

	if status != reqwest::StatusCode::OK {
		return Err(Error::Internal(format!(
			"Token exchange failed: status={status}, body={}",
			sanitize_body_for_error(&body)
		)));
	}

	serde_json::from_str(&body).map_err(|err| {
		Error::Internal(format!(
			"Token exchange response parse failed: {err}; body={}",
			sanitize_body_for_error(&body)
		))
	})
}

// endregion: --- Token Exchange

// region:    --- CopilotTokenManager

pub struct CopilotTokenManager {
	oauth_token: Arc<Mutex<Option<String>>>,
	session: Arc<Mutex<Option<CachedSession>>>,
	callback: Arc<dyn CopilotAuthCallback>,
	http_client: reqwest::Client,
	#[cfg(test)]
	exchange_url: Option<String>,
	#[cfg(test)]
	device_flow_urls: Option<TestDeviceFlowUrls>,
	#[cfg(test)]
	token_store: Option<CopilotTokenStore>,
}

struct CachedSession {
	token: String,
	api_url: String,
	expires_at: i64,
}

impl CopilotTokenManager {
	pub fn new(callback: impl CopilotAuthCallback + 'static) -> Self {
		Self {
			oauth_token: Arc::new(Mutex::new(None)),
			session: Arc::new(Mutex::new(None)),
			callback: Arc::new(callback),
			http_client: reqwest::Client::new(),
			#[cfg(test)]
			exchange_url: None,
			#[cfg(test)]
			device_flow_urls: None,
			#[cfg(test)]
			token_store: None,
		}
	}

	pub fn with_oauth_token(callback: impl CopilotAuthCallback + 'static, token: String) -> Self {
		Self {
			oauth_token: Arc::new(Mutex::new(Some(token))),
			session: Arc::new(Mutex::new(None)),
			callback: Arc::new(callback),
			http_client: reqwest::Client::new(),
			#[cfg(test)]
			exchange_url: None,
			#[cfg(test)]
			device_flow_urls: None,
			#[cfg(test)]
			token_store: None,
		}
	}

	#[cfg(test)]
	fn with_exchange_url(mut self, url: String) -> Self {
		self.exchange_url = Some(url);
		self
	}

	#[cfg(test)]
	fn with_store(mut self, store: CopilotTokenStore) -> Self {
		self.token_store = Some(store);
		self
	}

	#[cfg(test)]
	fn with_device_flow_urls(mut self, device_code_url: String, oauth_token_url: String) -> Self {
		self.device_flow_urls = Some(TestDeviceFlowUrls {
			device_code_url,
			oauth_token_url,
		});
		self
	}

	/// Holds the `session` mutex for the full duration of the call.
	///
	/// This intentionally provides single-flight behaviour for GitHub Copilot auth:
	/// concurrent callers serialize here so only one session refresh or device-flow
	/// login runs at a time. While a device flow is active, other callers block on
	/// the mutex until the in-flight attempt completes.
	pub async fn get_session(&self) -> crate::Result<(String, String)> {
		let mut cached = self.session.lock().await;

		if let Some(session) = &*cached
			&& is_session_valid(session)
		{
			return Ok((session.token.clone(), session.api_url.clone()));
		}

		let store = self.token_store()?;
		let oauth_token = self.ensure_oauth_token(&store).await?;

		#[cfg(test)]
		let session_result = if let Some(url) = &self.exchange_url {
			exchange_for_session_token_with_url(&self.http_client, &oauth_token, url).await
		} else {
			exchange_for_session_token(&self.http_client, &oauth_token).await
		};
		#[cfg(not(test))]
		let session_result = exchange_for_session_token(&self.http_client, &oauth_token).await;

		let session_token = match session_result {
			Ok(session_token) => session_token,
			Err(err) if is_auth_error(&err) => {
				self.clear_oauth_token(&store).await?;
				let fresh_token = self.ensure_oauth_token(&store).await?;

				#[cfg(test)]
				{
					if let Some(url) = &self.exchange_url {
						exchange_for_session_token_with_url(&self.http_client, &fresh_token, url).await?
					} else {
						exchange_for_session_token(&self.http_client, &fresh_token).await?
					}
				}
				#[cfg(not(test))]
				{
					exchange_for_session_token(&self.http_client, &fresh_token).await?
				}
			}
			Err(err) => return Err(err),
		};

		let api_url = if session_token.endpoints.api.is_empty() {
			COPILOT_DEFAULT_ENDPOINT.to_string()
		} else {
			session_token.endpoints.api.clone()
		};

		let cached_session = CachedSession {
			token: session_token.token.clone(),
			api_url: api_url.clone(),
			expires_at: session_token.expires_at,
		};

		*cached = Some(cached_session);

		Ok((session_token.token, api_url))
	}

	#[cfg(test)]
	fn into_cached_session(session: CopilotSessionToken) -> CachedSession {
		let api_url = if session.endpoints.api.is_empty() {
			COPILOT_DEFAULT_ENDPOINT.to_string()
		} else {
			session.endpoints.api
		};

		CachedSession {
			token: session.token,
			api_url,
			expires_at: session.expires_at,
		}
	}

	async fn ensure_oauth_token(&self, store: &CopilotTokenStore) -> crate::Result<String> {
		let mut oauth_token = self.oauth_token.lock().await;

		if let Some(token) = &*oauth_token {
			return Ok(token.clone());
		}

		if let Some(persisted) = store.load()? {
			let token = persisted.access_token;
			*oauth_token = Some(token.clone());
			return Ok(token);
		}

		#[cfg(test)]
		let oauth_response = if let Some(urls) = &self.device_flow_urls {
			device_flow_login_with_urls(
				&self.http_client,
				&*self.callback,
				DeviceFlowUrls {
					device_code_url: &urls.device_code_url,
					oauth_token_url: &urls.oauth_token_url,
				},
			)
			.await?
		} else {
			device_flow_login(&self.http_client, &*self.callback).await?
		};
		#[cfg(not(test))]
		let oauth_response = device_flow_login(&self.http_client, &*self.callback).await?;
		let token = oauth_response.access_token.clone();
		let persisted = PersistedOAuthToken::new(
			oauth_response.access_token,
			oauth_response.token_type,
			oauth_response.scope,
		);
		store.save(&persisted)?;

		*oauth_token = Some(token.clone());

		Ok(token)
	}

	fn token_store(&self) -> crate::Result<CopilotTokenStore> {
		#[cfg(test)]
		{
			return match &self.token_store {
				Some(store) => Ok(store.clone()),
				None => CopilotTokenStore::new(),
			};
		}

		#[cfg(not(test))]
		{
			CopilotTokenStore::new()
		}
	}

	async fn clear_oauth_token(&self, store: &CopilotTokenStore) -> crate::Result<()> {
		let mut oauth_token = self.oauth_token.lock().await;
		*oauth_token = None;
		store.clear()?;
		Ok(())
	}

	pub fn into_service_target_resolver(self) -> ServiceTargetResolver {
		let manager = Arc::new(self);
		let resolver = move |service_target: ServiceTarget| {
			let manager = Arc::clone(&manager);
			Box::pin(async move {
				let (session_token, api_url) = manager
					.get_session()
					.await
					.map_err(|err| crate::resolver::Error::Custom(err.to_string()))?;

				Ok(ServiceTarget {
					auth: AuthData::from_single(session_token),
					endpoint: Endpoint::from_owned(api_url),
					model: service_target.model,
				})
			}) as Pin<Box<dyn Future<Output = crate::resolver::Result<ServiceTarget>> + Send>>
		};

		ServiceTargetResolver::from_resolver_async_fn(resolver)
	}
}

// endregion: --- CopilotTokenManager

// region:    --- Helpers

/// Redact known sensitive fields from a JSON response body before including it
/// in error messages. Replaces values of `access_token`, `token`, and `device_code`
/// with `"[REDACTED]"`. Non-JSON bodies are replaced entirely.
fn sanitize_body_for_error(body: &str) -> String {
	const SENSITIVE_KEYS: &[&str] = &["access_token", "token", "device_code"];

	let Ok(mut value) = serde_json::from_str::<serde_json::Value>(body) else {
		return "[non-JSON body omitted]".to_string();
	};

	if let Some(obj) = value.as_object_mut() {
		for key in SENSITIVE_KEYS {
			if obj.contains_key(*key) {
				obj.insert((*key).to_string(), serde_json::Value::String("[REDACTED]".to_string()));
			}
		}
	}

	value.to_string()
}

fn is_session_valid(session: &CachedSession) -> bool {
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_secs() as i64)
		.unwrap_or(0);

	session.expires_at - TOKEN_REFRESH_BUFFER_SECS > now
}

fn is_auth_error(err: &crate::Error) -> bool {
	match err {
		crate::Error::HttpError { status, .. } => status.as_u16() == 401,
		crate::Error::Internal(msg) => msg.contains("401") || msg.contains("Unauthorized"),
		_ => false,
	}
}

// endregion: --- Helpers

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::adapters::github_copilot::copilot_types::CopilotEndpoints;

	#[tokio::test]
	async fn test_manager_new_does_not_panic() {
		let callback = |_: &str, _: &str| {};
		let _ = CopilotTokenManager::new(callback);
	}

	#[tokio::test]
	async fn test_manager_with_oauth_token_does_not_panic() {
		let callback = |_: &str, _: &str| {};
		let _ = CopilotTokenManager::with_oauth_token(callback, "test_token".into());
	}

	#[test]
	fn test_into_service_target_resolver_is_sync() {
		let callback = |_: &str, _: &str| {};
		let manager = CopilotTokenManager::new(callback);
		let _resolver = manager.into_service_target_resolver();
	}

	#[test]
	fn test_session_validity_uses_refresh_buffer() {
		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("time should be after unix epoch")
			.as_secs() as i64;

		let valid = CachedSession {
			token: "session".into(),
			api_url: COPILOT_DEFAULT_ENDPOINT.into(),
			expires_at: now + TOKEN_REFRESH_BUFFER_SECS + 10,
		};
		let stale = CachedSession {
			token: "session".into(),
			api_url: COPILOT_DEFAULT_ENDPOINT.into(),
			expires_at: now + TOKEN_REFRESH_BUFFER_SECS - 1,
		};

		assert!(is_session_valid(&valid));
		assert!(!is_session_valid(&stale));
	}

	#[test]
	fn test_into_cached_session_falls_back_to_default_endpoint() {
		let cached = CopilotTokenManager::into_cached_session(CopilotSessionToken {
			token: "session".into(),
			expires_at: 123,
			endpoints: CopilotEndpoints { api: String::new() },
		});

		assert_eq!(cached.api_url, COPILOT_DEFAULT_ENDPOINT);
	}

	mod mock_server {
		use http_body_util::Full;
		use hyper::body::Bytes;
		use hyper::server::conn::http1;
		use hyper::service::service_fn;
		use hyper::{Request, Response, StatusCode};
		use hyper_util::rt::TokioIo;
		use std::convert::Infallible;
		use std::sync::{Arc, Mutex};
		use tokio::net::TcpListener;

		#[derive(Clone)]
		pub struct MockRoute {
			pub method: String,
			pub path: String,
			pub status: u16,
			pub body: String,
		}

		pub struct CapturedRequest {
			pub method: String,
			pub path: String,
			pub headers: Vec<(String, String)>,
		}

		pub struct MockServer {
			pub port: u16,
			pub captured: Arc<Mutex<Vec<CapturedRequest>>>,
			shutdown_tx: tokio::sync::oneshot::Sender<()>,
		}

		impl MockServer {
			pub fn base_url(&self) -> String {
				format!("http://127.0.0.1:{}", self.port)
			}

			pub fn url(&self, path: &str) -> String {
				format!("{}{}", self.base_url(), path)
			}

			pub fn captured_requests(&self) -> Vec<CapturedRequest> {
				let guard = self.captured.lock().unwrap();
				guard
					.iter()
					.map(|r| CapturedRequest {
						method: r.method.clone(),
						path: r.path.clone(),
						headers: r.headers.clone(),
					})
					.collect()
			}

			pub fn shutdown(self) {
				let _ = self.shutdown_tx.send(());
			}
		}

		pub async fn start(routes: Vec<MockRoute>) -> MockServer {
			let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
			let port = listener.local_addr().unwrap().port();
			let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
			let routes = Arc::new(Mutex::new(routes));
			let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

			let captured_clone = Arc::clone(&captured);
			let routes_clone = Arc::clone(&routes);

			tokio::spawn(async move {
				loop {
					tokio::select! {
						accept_result = listener.accept() => {
							let (stream, _) = match accept_result {
								Ok(v) => v,
								Err(_) => continue,
							};
							let io = TokioIo::new(stream);
							let captured = Arc::clone(&captured_clone);
							let routes = Arc::clone(&routes_clone);

							tokio::spawn(async move {
								let svc = service_fn(move |req: Request<hyper::body::Incoming>| {
									let captured = Arc::clone(&captured);
									let routes = Arc::clone(&routes);
									async move {
										let method = req.method().to_string();
										let path = req.uri().path().to_string();
										let headers: Vec<(String, String)> = req
											.headers()
											.iter()
											.map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
											.collect();

										captured.lock().unwrap().push(CapturedRequest {
											method: method.clone(),
											path: path.clone(),
											headers,
									});

									let mut routes_guard = routes.lock().unwrap();
									let idx = routes_guard.iter().position(|r| {
											r.method == method && r.path == path
										});

										if let Some(idx) = idx {
											let route = routes_guard.remove(idx);
											let status = StatusCode::from_u16(route.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
											Ok::<_, Infallible>(
												Response::builder()
													.status(status)
													.header("content-type", "application/json")
													.body(Full::new(Bytes::from(route.body)))
													.unwrap(),
											)
										} else {
											Ok(Response::builder()
												.status(StatusCode::NOT_FOUND)
												.body(Full::new(Bytes::from("{\"error\":\"no matching mock route\"}")))
												.unwrap())
										}
									}
								});

								let _ = http1::Builder::new().serve_connection(io, svc).await;
							});
						}
						_ = &mut shutdown_rx => {
							break;
						}
					}
				}
			});

			MockServer {
				port,
				captured,
				shutdown_tx,
			}
		}
	}

	// -- Device Flow Tests

	#[tokio::test]
	async fn test_device_flow_happy_path() {
		let server = mock_server::start(vec![
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/device/code".into(),
				status: 200,
				body: r#"{"device_code":"DC123","user_code":"ABCD-1234","verification_uri":"https://test.github.com/device","interval":0,"expires_in":300}"#.into(),
			},
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/oauth/access_token".into(),
				status: 200,
				body: r#"{"access_token":"ghu_test123","token_type":"bearer","scope":"read:user"}"#.into(),
			},
		])
		.await;

		let captured_code = Arc::new(std::sync::Mutex::new(String::new()));
		let captured_uri = Arc::new(std::sync::Mutex::new(String::new()));
		let code_clone = Arc::clone(&captured_code);
		let uri_clone = Arc::clone(&captured_uri);

		let callback = move |user_code: &str, verification_uri: &str| {
			*code_clone.lock().unwrap() = user_code.to_string();
			*uri_clone.lock().unwrap() = verification_uri.to_string();
		};

		let client = reqwest::Client::new();
		let urls = DeviceFlowUrls {
			device_code_url: &server.url("/login/device/code"),
			oauth_token_url: &server.url("/login/oauth/access_token"),
		};

		let result = device_flow_login_with_urls(&client, &callback, urls).await;
		let token = result.expect("device flow should succeed");

		assert_eq!(token.access_token, "ghu_test123");
		assert_eq!(token.token_type, "bearer");
		assert_eq!(token.scope, "read:user");
		assert_eq!(captured_code.lock().unwrap().as_str(), "ABCD-1234");
		assert_eq!(captured_uri.lock().unwrap().as_str(), "https://test.github.com/device");

		server.shutdown();
	}

	#[tokio::test]
	async fn test_device_flow_slow_down_then_success() {
		let server = mock_server::start(vec![
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/device/code".into(),
				status: 200,
				body: r#"{"device_code":"DC456","user_code":"SLOW-DOWN","verification_uri":"https://test.github.com/device","interval":0,"expires_in":300}"#.into(),
			},
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/oauth/access_token".into(),
				status: 200,
				body: r#"{"error":"slow_down"}"#.into(),
			},
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/oauth/access_token".into(),
				status: 200,
				body: r#"{"access_token":"ghu_after_slowdown","token_type":"bearer","scope":"read:user"}"#.into(),
			},
		])
		.await;

		let callback = |_: &str, _: &str| {};
		let client = reqwest::Client::new();
		let urls = DeviceFlowUrls {
			device_code_url: &server.url("/login/device/code"),
			oauth_token_url: &server.url("/login/oauth/access_token"),
		};

		let result = device_flow_login_with_urls(&client, &callback, urls).await;
		let token = result.expect("device flow should succeed after slow_down");

		assert_eq!(token.access_token, "ghu_after_slowdown");

		server.shutdown();
	}

	#[tokio::test]
	async fn test_device_flow_expired_token() {
		let server = mock_server::start(vec![
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/device/code".into(),
				status: 200,
				body: r#"{"device_code":"DC_EXP","user_code":"EXP-CODE","verification_uri":"https://test.github.com/device","interval":0,"expires_in":300}"#.into(),
			},
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/oauth/access_token".into(),
				status: 200,
				body: r#"{"error":"expired_token","error_description":"The device code has expired."}"#.into(),
			},
		])
		.await;

		let callback = |_: &str, _: &str| {};
		let client = reqwest::Client::new();
		let urls = DeviceFlowUrls {
			device_code_url: &server.url("/login/device/code"),
			oauth_token_url: &server.url("/login/oauth/access_token"),
		};

		let result = device_flow_login_with_urls(&client, &callback, urls).await;
		let err = result.expect_err("expired_token should produce an error");

		let err_msg = err.to_string();
		assert!(
			err_msg.contains("expired_token"),
			"error should mention expired_token, got: {err_msg}"
		);
		assert!(
			err_msg.contains("device code has expired"),
			"error should contain description, got: {err_msg}"
		);

		server.shutdown();
	}

	#[tokio::test]
	async fn test_device_flow_access_denied() {
		let server = mock_server::start(vec![
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/device/code".into(),
				status: 200,
				body: r#"{"device_code":"DC_DENY","user_code":"DENY-CODE","verification_uri":"https://test.github.com/device","interval":0,"expires_in":300}"#.into(),
			},
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/oauth/access_token".into(),
				status: 200,
				body: r#"{"error":"access_denied","error_description":"User denied."}"#.into(),
			},
		])
		.await;

		let callback = |_: &str, _: &str| {};
		let client = reqwest::Client::new();
		let urls = DeviceFlowUrls {
			device_code_url: &server.url("/login/device/code"),
			oauth_token_url: &server.url("/login/oauth/access_token"),
		};

		let result = device_flow_login_with_urls(&client, &callback, urls).await;
		let err = result.expect_err("access_denied should produce an error");

		let err_msg = err.to_string();
		assert!(
			err_msg.contains("access_denied"),
			"error should mention access_denied, got: {err_msg}"
		);

		server.shutdown();
	}

	// -- Token Exchange Tests

	#[tokio::test]
	async fn test_token_exchange_happy_path() {
		let server = mock_server::start(vec![mock_server::MockRoute {
			method: "GET".into(),
			path: "/copilot_internal/v2/token".into(),
			status: 200,
			body: r#"{"token":"tid=test123","expires_at":9999999999,"endpoints":{"api":"https://api.test.githubcopilot.com"}}"#.into(),
		}])
		.await;

		let client = reqwest::Client::new();
		let url = server.url("/copilot_internal/v2/token");

		let result = exchange_for_session_token_with_url(&client, "ghu_test", &url).await;
		let session = result.expect("token exchange should succeed");

		assert_eq!(session.token, "tid=test123");
		assert_eq!(session.expires_at, 9999999999);
		assert_eq!(session.endpoints.api, "https://api.test.githubcopilot.com");

		let requests = server.captured_requests();
		assert_eq!(requests.len(), 1);
		let auth_header = requests[0]
			.headers
			.iter()
			.find(|(k, _)| k == "authorization")
			.expect("authorization header should be present");
		assert_eq!(auth_header.1, "token ghu_test", "must use 'token' scheme, not 'Bearer'");

		server.shutdown();
	}

	#[tokio::test]
	async fn test_token_exchange_401_failure() {
		let server = mock_server::start(vec![mock_server::MockRoute {
			method: "GET".into(),
			path: "/copilot_internal/v2/token".into(),
			status: 401,
			body: r#"{"message":"Bad credentials"}"#.into(),
		}])
		.await;

		let client = reqwest::Client::new();
		let url = server.url("/copilot_internal/v2/token");

		let result = exchange_for_session_token_with_url(&client, "ghu_invalid", &url).await;
		let err = result.expect_err("401 should produce an error");

		let err_msg = err.to_string();
		assert!(
			err_msg.contains("401"),
			"error should contain status 401, got: {err_msg}"
		);

		server.shutdown();
	}

	// -- Token Manager Caching Tests

	fn session_token_body(token: &str, expires_at: i64, api_url: &str) -> String {
		format!(r#"{{"token":"{token}","expires_at":{expires_at},"endpoints":{{"api":"{api_url}"}}}}"#)
	}

	fn now_secs() -> i64 {
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("time should be after unix epoch")
			.as_secs() as i64
	}

	#[tokio::test]
	async fn test_token_manager_caches_session_token() {
		let expires_at = now_secs() + TOKEN_REFRESH_BUFFER_SECS + 3000;
		let server = mock_server::start(vec![mock_server::MockRoute {
			method: "GET".into(),
			path: "/copilot_internal/v2/token".into(),
			status: 200,
			body: session_token_body("cached_session_tok", expires_at, "https://api.test.githubcopilot.com"),
		}])
		.await;

		let callback = |_: &str, _: &str| {};
		let manager = CopilotTokenManager::with_oauth_token(callback, "ghu_cached_test".into())
			.with_exchange_url(server.url("/copilot_internal/v2/token"));

		// When: call get_session twice
		let (token1, url1) = manager.get_session().await.expect("first get_session should succeed");
		let (token2, url2) = manager.get_session().await.expect("second get_session should succeed");

		// Then: same token returned, server hit only once
		assert_eq!(token1, "cached_session_tok");
		assert_eq!(token1, token2, "second call should return the same cached token");
		assert_eq!(url1, url2);
		let requests = server.captured_requests();
		assert_eq!(
			requests.len(),
			1,
			"exchange endpoint should be called only once; second call must use cache"
		);

		server.shutdown();
	}

	#[tokio::test]
	async fn test_token_manager_refreshes_expired_session() {
		let now = now_secs();
		let stale_expires = now + TOKEN_REFRESH_BUFFER_SECS - 10;
		let fresh_expires = now + TOKEN_REFRESH_BUFFER_SECS + 3000;

		let server = mock_server::start(vec![
			mock_server::MockRoute {
				method: "GET".into(),
				path: "/copilot_internal/v2/token".into(),
				status: 200,
				body: session_token_body("stale_session_tok", stale_expires, "https://api.test.githubcopilot.com"),
			},
			mock_server::MockRoute {
				method: "GET".into(),
				path: "/copilot_internal/v2/token".into(),
				status: 200,
				body: session_token_body("fresh_session_tok", fresh_expires, "https://api.test.githubcopilot.com"),
			},
		])
		.await;

		let callback = |_: &str, _: &str| {};
		let manager = CopilotTokenManager::with_oauth_token(callback, "ghu_refresh_test".into())
			.with_exchange_url(server.url("/copilot_internal/v2/token"));

		// When: first call gets stale token, second call should refresh
		let (token1, _) = manager.get_session().await.expect("first get_session should succeed");
		assert_eq!(token1, "stale_session_tok");

		let (token2, _) = manager.get_session().await.expect("second get_session should succeed");
		assert_eq!(token2, "fresh_session_tok", "expired session should trigger a refresh");

		// Then: exchange endpoint called twice
		let requests = server.captured_requests();
		assert_eq!(
			requests.len(),
			2,
			"exchange endpoint should be called twice when session is expired"
		);

		server.shutdown();
	}

	#[tokio::test]
	async fn test_token_manager_loads_oauth_from_disk() {
		let now = now_secs();
		let expires_at = now + TOKEN_REFRESH_BUFFER_SECS + 3000;

		// Given: a persisted OAuth token on disk
		let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("time").as_nanos();
		let temp_path = std::env::temp_dir().join(format!("genai_test_oauth_disk_{nonce}_{}.json", std::process::id()));
		let store = CopilotTokenStore::with_path(temp_path.clone());
		let persisted = PersistedOAuthToken::new("ghu_disk_test", "bearer", "read:user");
		store.save(&persisted).expect("save should succeed");

		let server = mock_server::start(vec![mock_server::MockRoute {
			method: "GET".into(),
			path: "/copilot_internal/v2/token".into(),
			status: 200,
			body: session_token_body("disk_session_tok", expires_at, "https://api.test.githubcopilot.com"),
		}])
		.await;

		let callback_called = Arc::new(std::sync::Mutex::new(false));
		let callback_clone = Arc::clone(&callback_called);
		let callback = move |_: &str, _: &str| {
			*callback_clone.lock().unwrap() = true;
		};

		// When: manager has no in-memory oauth_token but store has one
		let manager = CopilotTokenManager::new(callback)
			.with_exchange_url(server.url("/copilot_internal/v2/token"))
			.with_store(store.clone());

		let (token, _) = manager.get_session().await.expect("get_session should succeed");

		// Then: session obtained without device flow
		assert_eq!(token, "disk_session_tok");
		assert!(
			!*callback_called.lock().unwrap(),
			"device flow callback should not be called when token is loaded from disk"
		);
		let requests = server.captured_requests();
		assert_eq!(requests.len(), 1);
		let auth_header = requests[0]
			.headers
			.iter()
			.find(|(k, _)| k == "authorization")
			.expect("authorization header should be present");
		assert_eq!(
			auth_header.1, "token ghu_disk_test",
			"exchange should use the disk-loaded OAuth token"
		);

		store.clear().ok();
		server.shutdown();
	}

	#[tokio::test]
	async fn test_token_manager_retries_after_invalid_persisted_oauth_token() {
		let now = now_secs();
		let expires_at = now + TOKEN_REFRESH_BUFFER_SECS + 3000;
		let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("time").as_nanos();
		let temp_path = std::env::temp_dir().join(format!(
			"genai_test_invalid_oauth_retry_{nonce}_{}.json",
			std::process::id()
		));
		let store = CopilotTokenStore::with_path(temp_path.clone());
		store
			.save(&PersistedOAuthToken::new("ghu_revoked", "bearer", "read:user"))
			.expect("save should succeed");

		let server = mock_server::start(vec![
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/device/code".into(),
				status: 200,
				body: r#"{"device_code":"DC_RETRY","user_code":"RETRY-CODE","verification_uri":"https://test.github.com/device","interval":0,"expires_in":300}"#.into(),
			},
			mock_server::MockRoute {
				method: "POST".into(),
				path: "/login/oauth/access_token".into(),
				status: 200,
				body: r#"{"access_token":"ghu_fresh_after_retry","token_type":"bearer","scope":"read:user"}"#.into(),
			},
			mock_server::MockRoute {
				method: "GET".into(),
				path: "/copilot_internal/v2/token".into(),
				status: 401,
				body: r#"{"message":"Bad credentials"}"#.into(),
			},
			mock_server::MockRoute {
				method: "GET".into(),
				path: "/copilot_internal/v2/token".into(),
				status: 200,
				body: session_token_body("fresh_session_tok", expires_at, "https://api.test.githubcopilot.com"),
			},
		])
		.await;

		let callback_called = Arc::new(std::sync::Mutex::new(false));
		let callback_clone = Arc::clone(&callback_called);
		let callback = move |_: &str, _: &str| {
			*callback_clone.lock().unwrap() = true;
		};

		let manager = CopilotTokenManager::new(callback)
			.with_device_flow_urls(
				server.url("/login/device/code"),
				server.url("/login/oauth/access_token"),
			)
			.with_exchange_url(server.url("/copilot_internal/v2/token"))
			.with_store(store.clone());

		let (token, _) = manager.get_session().await.expect("get_session should recover after 401");

		assert_eq!(token, "fresh_session_tok");
		let persisted = store
			.load()
			.expect("load should succeed")
			.expect("fresh OAuth token should be re-persisted after recovery");
		assert_eq!(persisted.access_token, "ghu_fresh_after_retry");
		assert!(
			*callback_called.lock().unwrap(),
			"device flow callback should run after invalid persisted token is cleared"
		);

		let requests = server.captured_requests();
		let exchange_requests: Vec<_> = requests
			.iter()
			.filter(|request| request.method == "GET" && request.path == "/copilot_internal/v2/token")
			.collect();
		assert_eq!(
			exchange_requests.len(),
			2,
			"exchange should retry exactly once after 401"
		);
		assert_eq!(
			exchange_requests[0]
				.headers
				.iter()
				.find(|(k, _)| k == "authorization")
				.expect("first authorization header should be present")
				.1,
			"token ghu_revoked"
		);
		assert_eq!(
			exchange_requests[1]
				.headers
				.iter()
				.find(|(k, _)| k == "authorization")
				.expect("second authorization header should be present")
				.1,
			"token ghu_fresh_after_retry"
		);

		server.shutdown();
	}

	#[tokio::test]
	async fn test_token_manager_concurrent_requests_share_single_exchange() {
		let expires_at = now_secs() + TOKEN_REFRESH_BUFFER_SECS + 3000;
		let server = mock_server::start(vec![mock_server::MockRoute {
			method: "GET".into(),
			path: "/copilot_internal/v2/token".into(),
			status: 200,
			body: session_token_body("shared_session_tok", expires_at, "https://api.test.githubcopilot.com"),
		}])
		.await;

		let manager = Arc::new(
			CopilotTokenManager::with_oauth_token(|_: &str, _: &str| {}, "ghu_shared".into())
				.with_exchange_url(server.url("/copilot_internal/v2/token")),
		);

		let (first, second) = tokio::join!(manager.get_session(), manager.get_session());
		let first = first.expect("first get_session should succeed");
		let second = second.expect("second get_session should succeed");

		assert_eq!(first.0, "shared_session_tok");
		assert_eq!(second.0, "shared_session_tok");
		assert_eq!(first.1, second.1);
		assert_eq!(
			server.captured_requests().len(),
			1,
			"single-flight session exchange should only hit the backend once"
		);

		server.shutdown();
	}

	#[test]
	fn test_token_store_save_load_clear_roundtrip() {
		let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("time").as_nanos();
		let temp_path = std::env::temp_dir().join(format!(
			"genai_test_store_roundtrip_{nonce}_{}.json",
			std::process::id()
		));
		let store = CopilotTokenStore::with_path(temp_path);

		let token = PersistedOAuthToken {
			access_token: "test_roundtrip_token".to_string(),
			token_type: "bearer".to_string(),
			scope: "read:user".to_string(),
			created_at: 1234567890,
		};

		store.save(&token).expect("save should succeed");

		let loaded = store.load().expect("load should succeed").expect("token should exist");
		assert_eq!(loaded.access_token, "test_roundtrip_token");
		assert_eq!(loaded.token_type, "bearer");
		assert_eq!(loaded.scope, "read:user");
		assert_eq!(loaded.created_at, 1234567890);

		store.clear().expect("clear should succeed");
		let after_clear = store.load().expect("load after clear should succeed");
		assert!(after_clear.is_none(), "token should be None after clear");
	}
}

// endregion: --- Tests
