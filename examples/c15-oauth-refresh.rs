//! OAuth token refresh example.
//!
//! This example demonstrates automatic token refresh with `OAuthRefreshManager`.
//!
//! The manager integrates with `Client` via `into_auth_resolver()`:
//! - Automatically refreshes tokens before they expire
//! - Calls `on_refresh` callback to save new tokens
//! - Calls `on_auth_required` when re-authentication is needed
//!
//! Run with:
//! ```
//! cargo run --example c15-oauth-refresh
//! ```

use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{OAuthConfig, OAuthCredentials, OAuthRefreshManager};
use genai::Client;
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
		println!("[auto-save] New tokens saved to {}", TOKEN_FILE);
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== OAuth Auto-Refresh Example ===\n");

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
			println!("Token EXPIRED {} minutes ago - will auto-refresh on first request", (-remaining) / 60);
		}
	}

	// Create credentials
	let mut creds = OAuthCredentials::new(&saved.access_token).with_oauth_config(OAuthConfig::minimal());
	if let Some(rt) = &saved.refresh_token {
		creds = creds.with_refresh_token(rt);
	}
	if let Some(exp) = saved.expires_at {
		creds = creds.with_expires_at(exp as u64);
	}

	// Create refresh manager with callbacks
	let refresh_manager = OAuthRefreshManager::new(creds)
		.with_on_refresh(|access, refresh, expires_in| {
			println!("\n=== TOKEN AUTO-REFRESHED ===");
			println!("New access token: {}...", &access[..30.min(access.len())]);
			println!("Expires in: {} hours", expires_in / 3600);
			save_token(access, refresh, expires_in);
		})
		.with_on_auth_required(|| {
			println!("\n=== RE-AUTHENTICATION REQUIRED ===");
			println!("Refresh token expired. Run: cargo run --example c13-oauth-full");
		});

	// Build client with auto-refreshing auth resolver
	// This is the key: into_auth_resolver() returns a resolver that
	// automatically refreshes tokens before each request if needed
	let client = Client::builder()
		.with_auth_resolver(refresh_manager.into_auth_resolver())
		.build();

	// Now just use the client normally - refresh happens automatically!
	println!("\n=== Test 1: Simple Chat ===");
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'Auto-refresh works!' and nothing else.")]);

	match client.exec_chat(MODEL, chat_req, None).await {
		Ok(response) => {
			if let Some(text) = response.first_text() {
				println!("Response: {}", text);
				println!("[PASS] API call successful");
			}
		}
		Err(e) => {
			println!("[FAIL] API call failed: {}", e);
		}
	}

	// Second request - uses the same (possibly refreshed) token
	println!("\n=== Test 2: Second Request ===");
	let chat_req2 = ChatRequest::new(vec![ChatMessage::user("Say 'Still working!' and nothing else.")]);

	match client.exec_chat(MODEL, chat_req2, None).await {
		Ok(response) => {
			if let Some(text) = response.first_text() {
				println!("Response: {}", text);
				println!("[PASS] Second request successful");
			}
		}
		Err(e) => {
			println!("[FAIL] Second request failed: {}", e);
		}
	}

	println!("\n=== Done ===");
	println!("\nNote: If token was expired, it was automatically refreshed");
	println!("and new tokens were saved to {}", TOKEN_FILE);
	Ok(())
}
