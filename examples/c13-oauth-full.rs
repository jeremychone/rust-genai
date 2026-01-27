//! Full OAuth authentication flow with comprehensive testing.
//!
//! This example demonstrates:
//! 1. Complete OAuth PKCE flow (authorization URL, local callback server, token exchange)
//! 2. Testing API access with OAuth tokens
//! 3. Tool use with automatic `proxy_` prefix handling
//! 4. All OAuth transformations (system prompt, user_id, obfuscation)
//!
//! Run with:
//! ```
//! cargo run --example c13-oauth-full
//! ```

use axum::{
	extract::Query,
	response::{Html, IntoResponse},
	routing::get,
	Router,
};
use genai::chat::{ChatMessage, ChatRequest, Tool, ToolResponse};
use genai::resolver::{AuthData, AuthResolver, OAuthCredentials};
use genai::{Client, ModelIden};
use rand::Rng;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::oneshot;

const AUTH_URL: &str = "https://claude.ai/oauth/authorize";
const TOKEN_URL: &str = "https://console.anthropic.com/v1/oauth/token";
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const REDIRECT_URI: &str = "http://localhost:54545/callback";
const SCOPES: &str = "org:create_api_key user:profile user:inference";

const MODEL: &str = "claude-sonnet-4-20250514";

#[derive(Debug, Deserialize)]
struct CallbackParams {
	code: String,
	state: Option<String>,
}

#[derive(Debug, Serialize)]
struct TokenRequest {
	code: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	state: Option<String>,
	grant_type: String,
	client_id: String,
	redirect_uri: String,
	code_verifier: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
	access_token: String,
	refresh_token: String,
	#[serde(default)]
	expires_in: i64,
	organization: Option<Organization>,
	account: Option<Account>,
}

#[derive(Debug, Deserialize)]
struct Organization {
	uuid: String,
	name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Account {
	uuid: String,
	email_address: String,
}

struct AppState {
	expected_state: String,
	code_verifier: String,
	tx: Option<oneshot::Sender<CallbackParams>>,
}

struct PkceCodes {
	verifier: String,
	challenge: String,
}

fn generate_pkce() -> PkceCodes {
	let mut rng = rand::rng();
	let random_bytes: Vec<u8> = (0..96).map(|_| rng.random::<u8>()).collect();

	let verifier = base64_url_encode(&random_bytes);

	let mut hasher = Sha256::new();
	hasher.update(verifier.as_bytes());
	let hash = hasher.finalize();
	let challenge = base64_url_encode(&hash);

	PkceCodes { verifier, challenge }
}

fn base64_url_encode(data: &[u8]) -> String {
	use base64::engine::general_purpose::URL_SAFE_NO_PAD;
	use base64::Engine;
	URL_SAFE_NO_PAD.encode(data)
}

fn generate_state() -> String {
	let mut rng = rand::rng();
	let random_bytes: Vec<u8> = (0..16).map(|_| rng.random::<u8>()).collect();
	hex::encode(random_bytes)
}

fn parse_code_and_state(code: &str) -> (String, Option<String>) {
	if let Some(pos) = code.find('#') {
		let (c, s) = code.split_at(pos);
		(c.to_string(), Some(s[1..].to_string()))
	} else {
		(code.to_string(), None)
	}
}

const TOKEN_FILE: &str = ".oauth_token.json";

#[derive(Debug, Serialize, Deserialize)]
struct SavedToken {
	access_token: String,
	refresh_token: Option<String>,
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

fn save_token(access_token: &str, refresh_token: Option<&str>) {
	let token = SavedToken {
		access_token: access_token.to_string(),
		refresh_token: refresh_token.map(|s| s.to_string()),
	};
	if let Ok(content) = serde_json::to_string_pretty(&token) {
		let _ = std::fs::write(get_token_path(), content);
		println!("Token saved to {}", TOKEN_FILE);
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	println!("=== genai OAuth Full Test ===\n");

	// Check if token is provided via environment variable
	if let Ok(token) = std::env::var("ANTHROPIC_OAUTH_TOKEN") {
		println!("Using OAuth token from ANTHROPIC_OAUTH_TOKEN environment variable\n");
		run_tests(&token, None).await?;
		return Ok(());
	}

	// Check for saved token file
	if let Some(saved) = load_saved_token() {
		println!("Using saved token from {}\n", TOKEN_FILE);
		run_tests(&saved.access_token, saved.refresh_token.as_deref()).await?;
		return Ok(());
	}

	// Otherwise, perform full OAuth flow
	println!("No saved token found, starting OAuth flow...\n");

	let token_response = perform_oauth_flow().await?;

	// Save token for future use
	save_token(&token_response.access_token, Some(&token_response.refresh_token));

	run_tests(&token_response.access_token, Some(&token_response.refresh_token)).await?;

	println!("\n=== All Tests Complete ===");
	Ok(())
}

async fn perform_oauth_flow() -> anyhow::Result<TokenResponse> {
	// Generate PKCE codes
	let pkce = generate_pkce();
	let state = generate_state();

	println!("Generated PKCE challenge and state token");
	println!("Code verifier length: {}", pkce.verifier.len());

	// Build authorization URL
	let params = [
		("client_id", CLIENT_ID),
		("code", "true"),
		("code_challenge", &pkce.challenge),
		("code_challenge_method", "S256"),
		("redirect_uri", REDIRECT_URI),
		("response_type", "code"),
		("scope", SCOPES),
		("state", &state),
	];

	let query_string = params
		.iter()
		.map(|(k, v)| {
			let encoded = urlencoding::encode(v).replace("%20", "+");
			format!("{}={}", k, encoded)
		})
		.collect::<Vec<_>>()
		.join("&");

	let auth_url = format!("{}?{}", AUTH_URL, query_string);

	println!("\nAuth URL:\n{}\n", auth_url);

	// Create channel for callback
	let (tx, rx) = oneshot::channel::<CallbackParams>();

	let app_state = Arc::new(tokio::sync::Mutex::new(AppState {
		expected_state: state.clone(),
		code_verifier: pkce.verifier.clone(),
		tx: Some(tx),
	}));

	// Start local server
	let app = Router::new().route("/callback", get(callback_handler)).with_state(app_state.clone());

	let listener = tokio::net::TcpListener::bind("127.0.0.1:54545").await?;
	println!("Local server started on http://localhost:54545");

	// Open browser
	println!("\nOpening browser for authentication...");
	if let Err(e) = webbrowser::open(&auth_url) {
		println!("Failed to open browser automatically: {}", e);
		println!("\nPlease open this URL manually:");
		println!("{}\n", auth_url);
	}

	// Spawn server
	let server = tokio::spawn(async move {
		axum::serve(listener, app).await.unwrap();
	});

	// Wait for callback
	println!("Waiting for OAuth callback...\n");
	let callback = rx.await?;

	println!("Received callback!");
	println!("Code: {}...", &callback.code[..20.min(callback.code.len())]);

	// Get verifier
	let state_guard = app_state.lock().await;
	let verifier = state_guard.code_verifier.clone();
	let expected_state = state_guard.expected_state.clone();
	drop(state_guard);

	let (actual_code, state_from_code) = parse_code_and_state(&callback.code);
	let final_state = callback.state.or(state_from_code).unwrap_or(expected_state);

	println!("\nExchanging code for tokens...");

	// Exchange code for tokens
	let token_request = TokenRequest {
		code: actual_code,
		state: Some(final_state),
		grant_type: "authorization_code".to_string(),
		client_id: CLIENT_ID.to_string(),
		redirect_uri: REDIRECT_URI.to_string(),
		code_verifier: verifier,
	};

	let client = reqwest::Client::new();
	let response = client
		.post(TOKEN_URL)
		.header("Content-Type", "application/json")
		.header("Accept", "application/json")
		.json(&token_request)
		.send()
		.await?;

	let status = response.status();
	let body = response.text().await?;

	if !status.is_success() {
		anyhow::bail!("Token exchange failed: {} - {}", status, body);
	}

	let token_response: TokenResponse = serde_json::from_str(&body)?;

	// Print results
	println!("\n=== OAuth Success ===");
	if let Some(org) = &token_response.organization {
		println!("Organization: {} ({})", org.name, org.uuid);
	}
	if let Some(account) = &token_response.account {
		println!("Account: {}", account.email_address);
	}
	println!("Expires in: {} seconds", token_response.expires_in);
	println!(
		"Access Token: {}...",
		&token_response.access_token[..30.min(token_response.access_token.len())]
	);

	// Shutdown server
	server.abort();

	Ok(token_response)
}

async fn run_tests(access_token: &str, refresh_token: Option<&str>) -> anyhow::Result<()> {
	// Verify token format
	if !OAuthCredentials::is_oauth_token(access_token) {
		println!("WARNING: Token doesn't match OAuth token format (sk-ant-oat-*)");
		println!("Proceeding anyway...\n");
	} else {
		println!("Token format verified: OAuth token (sk-ant-oat-*)\n");
	}

	// Test 0: Raw reqwest test (bypass genai entirely)
	test_raw_reqwest(access_token).await?;

	// Create OAuth credentials
	let mut creds = OAuthCredentials::new(access_token);
	if let Some(rt) = refresh_token {
		creds = creds.with_refresh_token(rt);
		println!("Refresh token attached to credentials");
	}

	// Build client with OAuth
	let auth_resolver = AuthResolver::from_resolver_fn(move |model_iden: ModelIden| {
		if model_iden.adapter_kind == genai::adapter::AdapterKind::Anthropic {
			Ok(Some(AuthData::from_oauth(creds.clone())))
		} else {
			Ok(None)
		}
	});

	let client = Client::builder().with_auth_resolver(auth_resolver).build();

	// Test 1: Simple chat
	test_simple_chat(&client).await?;

	// Test 2: Tool use
	test_tool_use(&client).await?;

	// Test 3: System prompt handling
	test_system_prompt(&client).await?;

	Ok(())
}

async fn test_simple_chat(client: &Client) -> anyhow::Result<()> {
	println!("\n=== Test 1: Simple Chat ===");
	println!("Testing basic OAuth chat without tools...\n");

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'OAuth works!' and nothing else.")]);

	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

	if let Some(text) = chat_res.first_text() {
		println!("Response: {}", text);
		if text.to_lowercase().contains("oauth works") {
			println!("[PASS] Simple chat working");
		} else {
			println!("[WARN] Unexpected response content");
		}
	} else {
		println!("[FAIL] No text content in response");
	}

	Ok(())
}

async fn test_tool_use(client: &Client) -> anyhow::Result<()> {
	use serde_json::json;

	println!("\n=== Test 2: Tool Use ===");
	println!("Testing tool use with OAuth (prefix_tool_names disabled by default)...\n");

	// Define a simple tool
	let calculator_tool = Tool::new("calculate")
		.with_description("Perform a mathematical calculation")
		.with_schema(json!({
			"type": "object",
			"properties": {
				"expression": {
					"type": "string",
					"description": "The mathematical expression to evaluate"
				}
			},
			"required": ["expression"]
		}));

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 42 * 17? Use the calculate tool.")])
		.with_tools(vec![calculator_tool]);

	println!("Sending request with tool 'calculate'...");
	println!("(Note: prefix_tool_names is disabled in default config)");

	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;

	let tool_calls = chat_res.into_tool_calls();

	if tool_calls.is_empty() {
		println!("[WARN] No tool calls received (model responded directly)");
		return Ok(());
	}

	println!("\nTool calls received:");
	for tc in &tool_calls {
		println!("  Function: {}", tc.fn_name);
		println!("  Arguments: {}", tc.fn_arguments);

		if tc.fn_name == "calculate" {
			println!("[PASS] Tool call received correctly");
		} else {
			println!("[WARN] Unexpected tool name: {}", tc.fn_name);
		}
	}

	// Continue with tool result
	let tool_response = ToolResponse::new(tool_calls[0].call_id.clone(), "714".to_string());

	let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

	println!("\nSending tool result back...");
	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

	if let Some(text) = chat_res.first_text() {
		println!("Final response: {}", text);
		println!("[PASS] Tool conversation completed successfully");
	}

	Ok(())
}

async fn test_system_prompt(client: &Client) -> anyhow::Result<()> {
	println!("\n=== Test 3: System Prompt Handling ===");
	println!("Testing that custom system prompts work with OAuth...\n");

	// OAuth automatically prepends "You are Claude Code..." to system prompts
	// Verify that our custom system prompt is also included
	let chat_req = ChatRequest::new(vec![ChatMessage::user(
		"What special instruction were you given? \
		Just answer with the key phrase from your system prompt.",
	)])
	.with_system("SPECIAL_TEST_PHRASE: The answer is 'banana republic'.");

	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

	if let Some(text) = chat_res.first_text() {
		println!("Response: {}", text);

		// The response should include reference to our custom system prompt
		// (even though Claude Code prompt was prepended)
		if text.to_lowercase().contains("banana") {
			println!("[PASS] Custom system prompt preserved alongside OAuth system prompt");
		} else {
			println!("[WARN] Custom system prompt may not have been preserved");
		}
	}

	Ok(())
}

/// Test with raw reqwest - bypasses genai entirely to see if the issue is in genai or Anthropic
async fn test_raw_reqwest(access_token: &str) -> anyhow::Result<()> {
	println!("\n=== Test 0: Raw reqwest (bypass genai) ===");
	println!("Testing direct API call with minimal setup...\n");

	// Use the exact format from anthropic_oauth_test.rs that supposedly worked
	let system_prompt: serde_json::Value = serde_json::json!([
		{
			"type": "text",
			"text": "You are Claude Code, Anthropic's official CLI for Claude.",
			"cache_control": {"type": "ephemeral"}
		}
	]);

	let request_body = serde_json::json!({
		"model": MODEL,
		"max_tokens": 100,
		"system": system_prompt,
		"messages": [{"role": "user", "content": "Say 'OAuth works!' and nothing else"}]
	});

	println!("Request body: {}", serde_json::to_string_pretty(&request_body)?);

	let client = reqwest::Client::new();
	let response = client
		.post("https://api.anthropic.com/v1/messages")
		.header("Content-Type", "application/json")
		.header("Authorization", format!("Bearer {}", access_token))
		.header("anthropic-version", "2023-06-01")
		.header("anthropic-beta", "oauth-2025-04-20")
		.json(&request_body)
		.send()
		.await?;

	let status = response.status();
	let body = response.text().await?;

	println!("Response status: {}", status);
	println!("Response body: {}", body);

	if status.is_success() {
		let json: serde_json::Value = serde_json::from_str(&body)?;
		if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
			for item in content {
				if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
					println!("\n[PASS] Raw reqwest response: {}", text);
				}
			}
		}
	} else {
		println!("\n[FAIL] Raw reqwest failed with status {}", status);
		// Don't return error - continue to other tests to compare
	}

	Ok(())
}

async fn callback_handler(
	Query(params): Query<CallbackParams>,
	axum::extract::State(state): axum::extract::State<Arc<tokio::sync::Mutex<AppState>>>,
) -> impl IntoResponse {
	let mut state_guard = state.lock().await;
	if let Some(tx) = state_guard.tx.take() {
		let _ = tx.send(params);
	}

	Html(
		r#"
		<!DOCTYPE html>
		<html>
		<head>
			<title>OAuth Complete</title>
			<style>
				body { font-family: system-ui; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #f5f5f5; }
				.card { background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); text-align: center; }
				h1 { color: #22c55e; margin-bottom: 0.5rem; }
				p { color: #666; }
			</style>
		</head>
		<body>
			<div class="card">
				<h1>Authentication Successful!</h1>
				<p>You can close this window and return to the terminal.</p>
			</div>
		</body>
		</html>
	"#,
	)
}
