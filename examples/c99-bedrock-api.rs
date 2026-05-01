//! AWS Bedrock Converse via the simple Bearer-token adapter.
//!
//! This adapter (`BedrockApiAdapter`) ships with genai by default — no Cargo feature required
//! and no extra dependencies pulled in. It authenticates with a Bedrock API key passed in the
//! `Authorization: Bearer` header.
//!
//! Required env vars:
//!   - BEDROCK_API_KEY: a Bedrock API key (see
//!     https://docs.aws.amazon.com/bedrock/latest/userguide/api-keys.html)
//!   - AWS_REGION (optional): defaults to `us-east-1`. Model availability varies by region.
//!
//! Run with: `cargo run --example c99-bedrock-api`

use genai::Client;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use tracing_subscriber::EnvFilter;

// Bedrock model IDs include the publisher prefix and usually a version suffix.
// Cross-region inference profiles are also supported (e.g. `us.anthropic.claude-...`).
const CLAUDE_MODEL: &str = "bedrock_api::us.anthropic.claude-sonnet-4-6";
const NOVA_MODEL: &str = "bedrock_api::global.amazon.nova-2-lite-v1:0";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		.init();

	if std::env::var("BEDROCK_API_KEY").is_err() {
		println!("Set BEDROCK_API_KEY to run this example.");
		return Ok(());
	}

	let client = Client::default();

	// -- Example 1: Claude via Bedrock Converse
	println!("--- Claude on Bedrock ---");
	let chat_request = ChatRequest::default()
		.with_system("Answer in one sentence")
		.append_message(ChatMessage::user("Why is the sky blue?"));
	let stream = client.exec_chat_stream(CLAUDE_MODEL, chat_request, None).await?;
	print_chat_stream(stream, None).await?;

	// -- Example 2: Amazon Nova via Bedrock Converse
	println!("\n--- Nova on Bedrock ---");
	let chat_request = ChatRequest::default()
		.with_system("Answer in one sentence")
		.append_message(ChatMessage::user("Why is the sky blue?"));
	let stream = client.exec_chat_stream(NOVA_MODEL, chat_request, None).await?;
	print_chat_stream(stream, None).await?;

	Ok(())
}
