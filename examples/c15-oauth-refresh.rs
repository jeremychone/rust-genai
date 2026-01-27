//! OAuth token refresh example.
//!
//! This example demonstrates:
//! 1. Loading saved OAuth tokens
//! 2. Setting up automatic refresh with callback
//! 3. The callback saves new tokens to file
//!
//! Run with:
//! ```
//! cargo run --example c15-oauth-refresh
//! ```

use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, AuthResolver, OAuthConfig, OAuthCredentials, OAuthRefreshManager};
use genai::{Client, ModelIden};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MODEL: &str = "claude-sonnet-4-20250514";
const TOKEN_FILE: &str = ".oauth_token.json";

#[derive(Debug, Serialize, Deserialize)]
struct SavedToken {
	access_token: String,
	refresh_token: Option<String>,
	expires_at: Option<i64>,
}

fn get_token_path() -> PathBuf {
	PathBuf::from(TOKEN_FILE)
}

fn load_saved_token() -> Option<SavedToken> {
	let path = get_token_path();
	if path.exists() {
		let content = std::fs::read_to_string(&path).ok()?;
		serde_json::from_str(&content).ok()
	} else {
		None
	}
}

fn save_token(access_token: &str, refresh_token: &str, expires_in: i64) {
	let expires_at = if expires_in > 0 {
		Some(
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_secs() as i64
				+ expires_in,
		)
	} else {
		None
	};

	let token = SavedToken {
		access_token: access_token.to_string(),
		refresh_token: Some(refresh_token.to_string()),
		expires_at,
	};

	if let Ok(content) = serde_json::to_string_pretty(&token) {
		let _ = std::fs::write(get_token_path(), content);
		println!("New tokens saved to {}", TOKEN_FILE);
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== OAuth Refresh Example ===\n");

	// Load saved token
	let saved = load_saved_token().ok_or("No saved token found. Run c13-oauth-full first.")?;

	println!("Loaded token from {}", TOKEN_FILE);

	// Show token status
	if let Some(expires_at) = saved.expires_at {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs() as i64;
		let remaining = expires_at - now;
		if remaining > 0 {
			let hours = remaining / 3600;
			let minutes = (remaining % 3600) / 60;
			println!("Token expires in: {}h {}m", hours, minutes);
		} else {
			println!("Token EXPIRED {} minutes ago - will refresh", (-remaining) / 60);
		}
	}

	// Create credentials with custom OAuth config
	// Available configs: OAuthConfig::minimal(), full_cloaking(), cli_proxy_api_compat(), none()
	let oauth_config = OAuthConfig::minimal(); // Only inject system prompt (required)

	let mut creds = OAuthCredentials::new(&saved.access_token).with_oauth_config(oauth_config);
	if let Some(rt) = &saved.refresh_token {
		creds = creds.with_refresh_token(rt);
	}
	if let Some(exp) = saved.expires_at {
		creds = creds.with_expires_at(exp as u64);
	}

	println!("OAuth config: {:?}", creds.oauth_config);

	// Create refresh manager with callbacks
	let refresh_manager = OAuthRefreshManager::new(creds)
		.with_on_refresh(|access, refresh, expires_in| {
			println!("\n=== TOKEN REFRESHED ===");
			println!("New access token: {}...", &access[..30.min(access.len())]);
			println!("New refresh token: {}...", &refresh[..30.min(refresh.len())]);
			println!("Expires in: {} seconds ({} hours)", expires_in, expires_in / 3600);

			// Save to file
			save_token(access, refresh, expires_in);
		})
		.with_on_auth_required(|| {
			// Called when refresh token is invalid and re-auth is needed
			println!("\n=== RE-AUTHENTICATION REQUIRED ===");
			println!("Your refresh token has expired or been revoked.");
			println!("Please run: cargo run --example c13-oauth-full");
			println!("to perform a new OAuth login.\n");
		});

	// Check if refresh is needed
	if refresh_manager.needs_refresh().await {
		println!("\nToken needs refresh, refreshing...");
		refresh_manager.force_refresh().await?;
	} else {
		println!("\nToken is still valid, no refresh needed");
	}

	// Get current credentials for the client
	let current_creds = refresh_manager.credentials().await;

	// Build client with OAuth
	let auth_resolver = AuthResolver::from_resolver_fn(move |model_iden: ModelIden| {
		if model_iden.adapter_kind == genai::adapter::AdapterKind::Anthropic {
			Ok(Some(AuthData::from_oauth(current_creds.clone())))
		} else {
			Ok(None)
		}
	});

	let client = Client::builder().with_auth_resolver(auth_resolver).build();

	// Test API call
	println!("\n=== Testing API Call ===");

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'Refresh works!' and nothing else.")]);

	match client.exec_chat(MODEL, chat_req, None).await {
		Ok(response) => {
			if let Some(text) = response.first_text() {
				println!("Response: {}", text);
				println!("[PASS] API call successful");
			}
		}
		Err(e) => {
			println!("[FAIL] API call failed: {}", e);

			// If failed, try to refresh and retry
			println!("\nAttempting token refresh...");
			if let Err(refresh_err) = refresh_manager.force_refresh().await {
				println!("Refresh failed: {}", refresh_err);
			} else {
				println!("Token refreshed, try running the example again");
			}
		}
	}

	// Demo: Force refresh
	println!("\n=== Force Refresh Demo ===");
	println!("Forcing token refresh even though current token is valid...");
	refresh_manager.force_refresh().await?;

	// Test with new token
	println!("\n=== Testing with NEW token ===");
	let new_creds = refresh_manager.credentials().await;

	let auth_resolver2 = AuthResolver::from_resolver_fn(move |model_iden: ModelIden| {
		if model_iden.adapter_kind == genai::adapter::AdapterKind::Anthropic {
			Ok(Some(AuthData::from_oauth(new_creds.clone())))
		} else {
			Ok(None)
		}
	});

	let client2 = Client::builder().with_auth_resolver(auth_resolver2).build();
	let chat_req2 = ChatRequest::new(vec![ChatMessage::user("Say 'New token works!' and nothing else.")]);

	match client2.exec_chat(MODEL, chat_req2, None).await {
		Ok(response) => {
			if let Some(text) = response.first_text() {
				println!("Response: {}", text);
				println!("[PASS] New token works!");
			}
		}
		Err(e) => {
			println!("[FAIL] API call with new token failed: {}", e);
		}
	}

	println!("\n=== Done ===");
	Ok(())
}
