use crate::Headers;
use crate::resolver::oauth_credentials::OAuthCredentials;
use crate::resolver::{Error, Result};
use std::collections::HashMap;
/// `AuthData` specifies either how or the key itself for an authentication resolver call.
#[derive(Clone)]
pub enum AuthData {
	/// Specify the environment name to get the key value from.
	FromEnv(String),

	/// The key value itself.
	Key(String),

	/// Override headers and request url for unorthodox authentication schemes
	RequestOverride { url: String, headers: Headers },

	/// The key names/values when a credential has multiple pieces of credential information.
	/// This will be adapter-specific.
	/// NOTE: Not used yet.
	MultiKeys(HashMap<String, String>),

	/// OAuth credentials for Claude Code CLI authentication.
	/// Uses `Authorization: Bearer {token}` instead of `x-api-key`.
	OAuth(OAuthCredentials),
}

/// Constructors
impl AuthData {
	/// Create a new `AuthData` from an environment variable name.
	pub fn from_env(env_name: impl Into<String>) -> Self {
		AuthData::FromEnv(env_name.into())
	}

	/// Create a new `AuthData` from a single value.
	pub fn from_single(value: impl Into<String>) -> Self {
		AuthData::Key(value.into())
	}

	/// Create a new `AuthData` from multiple values.
	pub fn from_multi(data: HashMap<String, String>) -> Self {
		AuthData::MultiKeys(data)
	}

	/// Create a new `AuthData` from OAuth credentials.
	pub fn from_oauth(creds: OAuthCredentials) -> Self {
		AuthData::OAuth(creds)
	}

	/// Create a new `AuthData` from an OAuth token string.
	///
	/// This is a convenience method that creates `OAuthCredentials` with just the token.
	pub fn from_oauth_token(token: impl Into<String>) -> Self {
		AuthData::OAuth(OAuthCredentials::new(token))
	}
}

/// Getters
impl AuthData {
	/// Get the single value from the `AuthData`.
	pub fn single_key_value(&self) -> Result<String> {
		match self {
			// Overrides don't use an api key
			AuthData::RequestOverride { .. } => Ok(String::new()),
			AuthData::FromEnv(env_name) => {
				// Get value from the environment name.
				let value = std::env::var(env_name).map_err(|_| Error::ApiKeyEnvNotFound {
					env_name: env_name.to_string(),
				})?;
				Ok(value)
			}
			AuthData::Key(value) => Ok(value.to_string()),
			AuthData::OAuth(creds) => Ok(creds.access_token.clone()),
			_ => Err(Error::ResolverAuthDataNotSingleValue),
		}
	}

	/// Check if this is OAuth authentication.
	pub fn is_oauth(&self) -> bool {
		matches!(self, AuthData::OAuth(_))
	}

	/// Get the OAuth credentials if this is an OAuth authentication.
	pub fn oauth_credentials(&self) -> Option<&OAuthCredentials> {
		match self {
			AuthData::OAuth(creds) => Some(creds),
			_ => None,
		}
	}
}

// region:    --- AuthData Std Impls

// Implement Debug to redact sensitive information.
impl std::fmt::Debug for AuthData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			// NOTE: Here we also redact for `FromEnv` in case the developer confuses this with a key.
			AuthData::FromEnv(_env_name) => write!(f, "AuthData::FromEnv(REDACTED)"),
			AuthData::Key(_) => write!(f, "AuthData::Single(REDACTED)"),
			AuthData::MultiKeys(_) => write!(f, "AuthData::Multi(REDACTED)"),
			AuthData::RequestOverride { .. } => {
				write!(f, "AuthData::RequestOverride {{ url: REDACTED, headers: REDACTED }}")
			}
			AuthData::OAuth(_) => write!(f, "AuthData::OAuth(REDACTED)"),
		}
	}
}

// endregion: --- AuthData Std Impls
