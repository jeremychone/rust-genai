//! OAuth token refresh functionality.
//!
//! This module handles automatic token refresh for OAuth credentials.
//! Based on CLIProxyAPI implementation.

use crate::adapter::AdapterKind;
use crate::resolver::{AuthData, AuthResolver, OAuthCredentials};
use crate::ModelIden;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// OAuth token refresh configuration and constants.
pub mod config {
	/// Token refresh endpoint.
	pub const TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";

	/// OAuth client ID (Claude Code CLI).
	pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

	/// Buffer time before expiration to trigger proactive refresh (5 minutes).
	pub const REFRESH_BUFFER_SECS: u64 = 5 * 60;
}

/// Response from the token refresh endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct TokenRefreshResponse {
	pub access_token: String,
	pub refresh_token: String,
	#[serde(default)]
	pub expires_in: i64,
	pub token_type: Option<String>,
}

/// Callback type for token refresh notifications.
/// Called with (new_access_token, new_refresh_token, expires_in_seconds).
pub type OnRefreshCallback = Arc<dyn Fn(&str, &str, i64) + Send + Sync>;

/// Callback type for re-authentication required notifications.
/// Called when refresh token is invalid and full OAuth flow is needed.
pub type OnAuthRequiredCallback = Arc<dyn Fn() + Send + Sync>;

/// Error indicating that re-authentication is required.
#[derive(Debug)]
pub struct AuthRequiredError {
	pub message: String,
}

impl std::fmt::Display for AuthRequiredError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Re-authentication required: {}", self.message)
	}
}

impl std::error::Error for AuthRequiredError {}

/// Refresh OAuth tokens using the refresh token.
///
/// This function calls Anthropic's token endpoint to exchange
/// a refresh token for new access and refresh tokens.
///
/// # Arguments
/// * `refresh_token` - The current refresh token
///
/// # Returns
/// * `Ok(TokenRefreshResponse)` - New tokens on success
/// * `Err(String)` - Error message on failure
pub async fn refresh_tokens(refresh_token: &str) -> Result<TokenRefreshResponse, String> {
	let client = reqwest::Client::new();

	let request_body = serde_json::json!({
		"client_id": config::CLIENT_ID,
		"grant_type": "refresh_token",
		"refresh_token": refresh_token,
	});

	let response = client
		.post(config::TOKEN_URL)
		.header("Content-Type", "application/json")
		.header("Accept", "application/json")
		.json(&request_body)
		.send()
		.await
		.map_err(|e| format!("Token refresh request failed: {}", e))?;

	let status = response.status();
	let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

	if !status.is_success() {
		return Err(format!("Token refresh failed with status {}: {}", status, body));
	}

	serde_json::from_str(&body).map_err(|e| format!("Failed to parse token response: {} - body: {}", e, body))
}

/// Refresh tokens with retry logic.
///
/// Attempts to refresh tokens up to `max_retries` times with exponential backoff.
pub async fn refresh_tokens_with_retry(refresh_token: &str, max_retries: u32) -> Result<TokenRefreshResponse, String> {
	let mut last_error = String::new();

	for attempt in 0..max_retries {
		if attempt > 0 {
			// Exponential backoff: 1s, 2s, 4s...
			let delay = std::time::Duration::from_secs(1 << (attempt - 1));
			tokio::time::sleep(delay).await;
		}

		match refresh_tokens(refresh_token).await {
			Ok(response) => return Ok(response),
			Err(e) => {
				last_error = e;
				// Continue to retry
			}
		}
	}

	Err(format!(
		"Token refresh failed after {} attempts: {}",
		max_retries, last_error
	))
}

/// Manager for handling OAuth token refresh with callbacks.
#[derive(Clone)]
pub struct OAuthRefreshManager {
	credentials: Arc<tokio::sync::RwLock<OAuthCredentials>>,
	on_refresh: Option<OnRefreshCallback>,
	on_auth_required: Option<OnAuthRequiredCallback>,
}

impl OAuthRefreshManager {
	/// Create a new refresh manager with credentials.
	pub fn new(credentials: OAuthCredentials) -> Self {
		Self {
			credentials: Arc::new(tokio::sync::RwLock::new(credentials)),
			on_refresh: None,
			on_auth_required: None,
		}
	}

	/// Set the callback to be called when tokens are refreshed.
	pub fn with_on_refresh<F>(mut self, callback: F) -> Self
	where
		F: Fn(&str, &str, i64) + Send + Sync + 'static,
	{
		self.on_refresh = Some(Arc::new(callback));
		self
	}

	/// Set the callback to be called when re-authentication is required.
	///
	/// This is called when the refresh token is invalid (expired or already used)
	/// and the user needs to go through the full OAuth flow again.
	pub fn with_on_auth_required<F>(mut self, callback: F) -> Self
	where
		F: Fn() + Send + Sync + 'static,
	{
		self.on_auth_required = Some(Arc::new(callback));
		self
	}

	/// Get the current access token.
	pub async fn access_token(&self) -> String {
		self.credentials.read().await.access_token.clone()
	}

	/// Check if the token needs refresh (expired or about to expire).
	pub async fn needs_refresh(&self) -> bool {
		self.credentials.read().await.is_expired()
	}

	/// Refresh tokens if needed (proactive refresh).
	///
	/// Returns `true` if tokens were refreshed, `false` if no refresh was needed.
	pub async fn refresh_if_needed(&self) -> Result<bool, String> {
		if !self.needs_refresh().await {
			return Ok(false);
		}

		self.force_refresh().await?;
		Ok(true)
	}

	/// Force a token refresh regardless of expiration status.
	///
	/// Returns `Err` with error message if refresh fails.
	/// If the error indicates the refresh token is invalid, the `on_auth_required`
	/// callback will be called (if set).
	pub async fn force_refresh(&self) -> Result<(), String> {
		let refresh_token = {
			let creds = self.credentials.read().await;
			creds
				.refresh_token
				.clone()
				.ok_or_else(|| "No refresh token available".to_string())?
		};

		let response = match refresh_tokens(&refresh_token).await {
			Ok(resp) => resp,
			Err(e) => {
				// Check if this is an auth error (invalid refresh token)
				if self.is_auth_required_error(&e) {
					if let Some(callback) = &self.on_auth_required {
						callback();
					}
				}
				return Err(e);
			}
		};

		// Calculate new expiration
		let expires_at = if response.expires_in > 0 {
			Some(
				std::time::SystemTime::now()
					.duration_since(std::time::UNIX_EPOCH)
					.unwrap()
					.as_secs()
					+ response.expires_in as u64,
			)
		} else {
			None
		};

		// Update credentials
		{
			let mut creds = self.credentials.write().await;
			creds.access_token = response.access_token.clone();
			creds.refresh_token = Some(response.refresh_token.clone());
			creds.expires_at = expires_at;
		}

		// Call callback if set
		if let Some(callback) = &self.on_refresh {
			callback(&response.access_token, &response.refresh_token, response.expires_in);
		}

		Ok(())
	}

	/// Get a clone of the current credentials.
	pub async fn credentials(&self) -> OAuthCredentials {
		self.credentials.read().await.clone()
	}

	/// Check if an error indicates that re-authentication is required.
	///
	/// This detects errors like:
	/// - "invalid_grant" - refresh token expired or already used
	/// - 401 status - unauthorized
	/// - 403 status - forbidden
	fn is_auth_required_error(&self, error: &str) -> bool {
		let error_lower = error.to_lowercase();
		error_lower.contains("invalid_grant")
			|| error_lower.contains("invalid_token")
			|| error_lower.contains("expired")
			|| error_lower.contains("revoked")
			|| error_lower.contains("status 401")
			|| error_lower.contains("status 403")
			|| error_lower.contains("unauthorized")
	}

	/// Convert this manager into an `AuthResolver` that automatically refreshes tokens.
	///
	/// The returned resolver will:
	/// 1. Check if the token needs refresh before each request
	/// 2. Refresh automatically if needed (calling `on_refresh` callback)
	/// 3. Return the current (possibly refreshed) credentials
	///
	/// This is the recommended way to use OAuth with automatic refresh.
	///
	/// # Example
	/// ```ignore
	/// let refresh_manager = OAuthRefreshManager::new(creds)
	///     .with_on_refresh(|access, refresh, expires_in| {
	///         save_tokens_to_file(access, refresh, expires_in);
	///     });
	///
	/// let client = Client::builder()
	///     .with_auth_resolver(refresh_manager.into_auth_resolver())
	///     .build();
	///
	/// // Now all requests will automatically refresh tokens when needed
	/// client.exec_chat(MODEL, chat_req, None).await?;
	/// ```
	pub fn into_auth_resolver(self) -> AuthResolver {
		let manager = Arc::new(self);

		AuthResolver::from_resolver_async_fn(move |model_iden: ModelIden| {
			let manager = manager.clone();

			Box::pin(async move {
				// Only handle Anthropic models
				if model_iden.adapter_kind != AdapterKind::Anthropic {
					return Ok(None);
				}

				// Refresh if needed (this is async)
				if let Err(e) = manager.refresh_if_needed().await {
					// Log error but continue with current token
					// The API call might still work if the token isn't actually expired
					tracing::warn!("OAuth token refresh failed: {}", e);
				}

				// Return current credentials
				let creds = manager.credentials().await;
				Ok(Some(AuthData::from_oauth(creds)))
			}) as Pin<Box<dyn Future<Output = crate::resolver::Result<Option<AuthData>>> + Send>>
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_refresh_manager_creation() {
		let creds = OAuthCredentials::new("test-access-token").with_refresh_token("test-refresh-token");

		let manager = OAuthRefreshManager::new(creds);
		assert!(manager.on_refresh.is_none());
	}

	#[test]
	fn test_refresh_manager_with_callback() {
		let creds = OAuthCredentials::new("test-access-token").with_refresh_token("test-refresh-token");

		let callback_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let callback_called_clone = callback_called.clone();

		let manager = OAuthRefreshManager::new(creds).with_on_refresh(move |_, _, _| {
			callback_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
		});

		assert!(manager.on_refresh.is_some());
	}
}
