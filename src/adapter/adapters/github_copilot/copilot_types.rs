use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// region:    --- Types

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceCodeRequest {
	pub client_id: String,
	pub scope: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceCodeResponse {
	pub device_code: String,
	pub user_code: String,
	pub verification_uri: String,
	pub expires_in: u64,
	pub interval: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OAuthTokenRequest {
	pub client_id: String,
	pub device_code: String,
	pub grant_type: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OAuthTokenResponse {
	pub access_token: String,
	pub token_type: String,
	pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OAuthPollError {
	pub error: String,
	pub error_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CopilotEndpoints {
	pub api: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CopilotSessionToken {
	pub token: String,
	pub expires_at: i64,
	pub endpoints: CopilotEndpoints,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersistedOAuthToken {
	pub access_token: String,
	pub token_type: String,
	pub scope: String,
	pub created_at: i64,
}

// endregion: --- Types

// region:    --- Constants

pub const COPILOT_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
pub const COPILOT_SCOPE: &str = "read:user";
pub(crate) const COPILOT_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const GITHUB_OAUTH_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const COPILOT_TOKEN_EXCHANGE_URL: &str = "https://api.github.com/copilot_internal/v2/token";
pub const COPILOT_DEFAULT_ENDPOINT: &str = "https://api.githubcopilot.com";
pub const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.26.7";
pub const COPILOT_EDITOR_VERSION: &str = "vscode/1.99.3";
pub const COPILOT_EDITOR_PLUGIN_VERSION: &str = "copilot-chat/0.26.7";
pub const COPILOT_INTEGRATION_ID: &str = "vscode-chat";
pub const TOKEN_REFRESH_BUFFER_SECS: i64 = 300;

// endregion: --- Constants

// region:    --- Constructors / Helpers

impl DeviceCodeRequest {
	pub fn new(client_id: impl Into<String>, scope: impl Into<String>) -> Self {
		Self {
			client_id: client_id.into(),
			scope: scope.into(),
		}
	}
}

impl OAuthTokenRequest {
	pub fn new(client_id: impl Into<String>, device_code: impl Into<String>) -> Self {
		Self {
			client_id: client_id.into(),
			device_code: device_code.into(),
			grant_type: COPILOT_GRANT_TYPE.to_string(),
		}
	}
}

impl PersistedOAuthToken {
	pub fn new(access_token: impl Into<String>, token_type: impl Into<String>, scope: impl Into<String>) -> Self {
		Self {
			access_token: access_token.into(),
			token_type: token_type.into(),
			scope: scope.into(),
			created_at: current_unix_seconds(),
		}
	}
}

fn current_unix_seconds() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_secs() as i64)
		.unwrap_or_default()
}

// endregion: --- Constructors / Helpers

// region:    --- Debug Impls

impl std::fmt::Debug for OAuthTokenResponse {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OAuthTokenResponse")
			.field("access_token", &"REDACTED")
			.field("token_type", &self.token_type)
			.field("scope", &self.scope)
			.finish()
	}
}

impl std::fmt::Debug for CopilotSessionToken {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CopilotSessionToken")
			.field("token", &"REDACTED")
			.field("expires_at", &self.expires_at)
			.field("endpoints", &self.endpoints)
			.finish()
	}
}

impl std::fmt::Debug for PersistedOAuthToken {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PersistedOAuthToken")
			.field("access_token", &"REDACTED")
			.field("token_type", &self.token_type)
			.field("scope", &self.scope)
			.field("created_at", &self.created_at)
			.finish()
	}
}

impl std::fmt::Debug for DeviceCodeResponse {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DeviceCodeResponse")
			.field("device_code", &"[REDACTED]")
			.field("user_code", &self.user_code)
			.field("verification_uri", &self.verification_uri)
			.field("expires_in", &self.expires_in)
			.field("interval", &self.interval)
			.finish()
	}
}

impl std::fmt::Debug for OAuthTokenRequest {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OAuthTokenRequest")
			.field("client_id", &self.client_id)
			.field("device_code", &"[REDACTED]")
			.field("grant_type", &self.grant_type)
			.finish()
	}
}

// endregion: --- Debug Impls

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_device_code_response_deserialize() {
		let json = r#"{"device_code":"DC123","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":899,"interval":5}"#;
		let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
		assert_eq!(resp.user_code, "ABCD-1234");
		assert_eq!(resp.interval, 5);
	}

	#[test]
	fn test_copilot_session_token_deserialize() {
		let json = r#"{"token":"tid=test","expires_at":9999999999,"refresh_in":1500,"endpoints":{"api":"https://api.individual.githubcopilot.com"}}"#;
		let tok: CopilotSessionToken = serde_json::from_str(json).unwrap();
		assert_eq!(tok.token, "tid=test");
		assert_eq!(tok.endpoints.api, "https://api.individual.githubcopilot.com");
	}

	#[test]
	fn test_persisted_token_roundtrip() {
		let token = PersistedOAuthToken {
			access_token: "ghu_test".to_string(),
			token_type: "bearer".to_string(),
			scope: "read:user".to_string(),
			created_at: 1234567890,
		};
		let json = serde_json::to_string(&token).unwrap();
		let loaded: PersistedOAuthToken = serde_json::from_str(&json).unwrap();
		assert_eq!(loaded.access_token, token.access_token);
		assert_eq!(loaded.created_at, token.created_at);
	}
}

// endregion: --- Tests
