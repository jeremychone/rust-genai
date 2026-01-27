//! This example demonstrates how to use OAuth authentication with Claude Code CLI tokens.
//!
//! OAuth tokens (starting with `sk-ant-oat-`) require different handling:
//! - Uses `Authorization: Bearer {token}` instead of `x-api-key`
//! - Automatically injects Claude Code system prompt
//! - Automatically prefixes/strips `proxy_` for tool names
//! - Automatically injects fake `metadata.user_id`
//!
//! To run this example, set the environment variable:
//! ```
//! export ANTHROPIC_OAUTH_TOKEN="sk-ant-oat-..."
//! cargo run --example c12-oauth
//! ```

use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest, Tool, ToolResponse};
use genai::resolver::{AuthData, AuthResolver, OAuthCredentials};
use genai::{Client, ModelIden};
use serde_json::json;
use tracing_subscriber::EnvFilter;

const MODEL: &str = "claude-sonnet-4-20250514";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		.init();

	// -- Get OAuth token from environment
	let oauth_token = std::env::var("ANTHROPIC_OAUTH_TOKEN").map_err(|_| {
		"ANTHROPIC_OAUTH_TOKEN environment variable not set. \
		Please set it to your Claude Code CLI OAuth token (starts with sk-ant-oat-)"
	})?;

	// Verify it's an OAuth token
	if !OAuthCredentials::is_oauth_token(&oauth_token) {
		return Err("Token doesn't appear to be an OAuth token (should contain 'sk-ant-oat')".into());
	}

	// -- Create OAuth credentials
	// Option 1: Simple - just the token
	let creds = OAuthCredentials::new(&oauth_token);

	// Option 2: With refresh token (if you have one)
	// let creds = OAuthCredentials::new(&oauth_token)
	//     .with_refresh_token("your-refresh-token");

	// -- Build client with OAuth authentication
	let auth_resolver = AuthResolver::from_resolver_fn(
		move |model_iden: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
			println!("\n>> OAuth auth for {} <<", model_iden.adapter_kind);
			// Only use OAuth for Anthropic models
			if model_iden.adapter_kind == genai::adapter::AdapterKind::Anthropic {
				Ok(Some(AuthData::from_oauth(creds.clone())))
			} else {
				// Fall back to default auth for other providers
				Ok(None)
			}
		},
	);

	let client = Client::builder().with_auth_resolver(auth_resolver).build();

	// -- Test 1: Simple chat
	println!("\n=== Test 1: Simple Chat ===\n");

	let chat_req = ChatRequest::default()
		.with_system("You are a helpful assistant. Keep responses brief.")
		.append_message(ChatMessage::user("Hello! What's 2+2?"));

	let chat_res = client.exec_chat_stream(MODEL, chat_req, None).await?;
	print_chat_stream(chat_res, None).await?;

	// -- Test 2: Tool use (demonstrates automatic proxy_ prefixing/stripping)
	println!("\n\n=== Test 2: Tool Use (with automatic proxy_ handling) ===\n");

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

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 15 * 7?")])
		.with_tools(vec![calculator_tool]);

	println!("--- Getting tool call from model");
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;

	let tool_calls = chat_res.into_tool_calls();

	if tool_calls.is_empty() {
		println!("No tool calls received (model may have responded directly)");
	} else {
		println!("--- Tool calls received:");
		for tool_call in &tool_calls {
			// Note: fn_name will be "calculate", not "proxy_calculate"
			// The proxy_ prefix is automatically stripped
			println!("  Function: {}", tool_call.fn_name);
			println!("  Arguments: {}", tool_call.fn_arguments);
		}

		// Simulate tool execution
		let first_tool_call = &tool_calls[0];
		let tool_response = ToolResponse::new(first_tool_call.call_id.clone(), "105".to_string());

		// Continue conversation with tool result
		let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

		println!("\n--- Getting final response with tool result");
		let chat_res = client.exec_chat_stream(MODEL, chat_req, None).await?;

		println!("\n--- Response:");
		print_chat_stream(chat_res, None).await?;
	}

	println!("\n\n=== OAuth Example Complete ===");

	Ok(())
}
