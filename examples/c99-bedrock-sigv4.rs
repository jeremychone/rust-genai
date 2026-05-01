//! AWS Bedrock Converse via the SigV4-signed adapter (full AWS credential chain).
//!
//! This example uses the `BedrockSigv4Adapter`, which is gated behind the `bedrock-sigv4`
//! Cargo feature because it pulls in `aws-config` and `aws-sigv4`.
//!
//! Credentials come from the AWS default chain: environment variables, shared credentials file,
//! SSO session, IMDS, or an assumed role. No `API key` is needed — set up AWS credentials the
//! same way you would for the AWS CLI.
//!
//! Run with: `cargo run --example c99-bedrock-sigv4 --features bedrock-sigv4`
//!
//! Required env vars (any of these set up via `aws configure` / SSO / IAM role / etc.):
//!   - AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY (+ AWS_SESSION_TOKEN for STS credentials), OR
//!   - AWS_PROFILE pointing at a profile in ~/.aws/credentials, OR
//!   - IMDS (on EC2 / ECS / EKS), OR
//!   - SSO session
//!
//! Optional:
//!   - AWS_REGION / AWS_DEFAULT_REGION: defaults to `us-east-1`. Model availability varies by
//!     region.

#[cfg(not(feature = "bedrock-sigv4"))]
fn main() {
	eprintln!("This example requires the `bedrock-sigv4` Cargo feature.");
	eprintln!("Run with: cargo run --example c99-bedrock-sigv4 --features bedrock-sigv4");
}

#[cfg(feature = "bedrock-sigv4")]
#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	use genai::Client;
	use genai::chat::printer::print_chat_stream;
	use genai::chat::{ChatMessage, ChatRequest};
	use tracing_subscriber::EnvFilter;

	// Bedrock model IDs include the publisher prefix and usually a version suffix.
	// Newer models (e.g. Claude Sonnet 4.5) are only available via cross-region inference
	// profiles — use the `us.`/`eu.`/`apac.` prefix for those.
	const CLAUDE_MODEL: &str = "bedrock_sigv4::us.anthropic.claude-sonnet-4-5-20250929-v1:0";
	const NOVA_MODEL: &str = "bedrock_sigv4::us.amazon.nova-pro-v1:0";

	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::new("genai=debug"))
		.init();

	let client = Client::default();

	// -- Example 1: Claude via Bedrock Converse + SigV4
	println!("--- Claude on Bedrock (SigV4) ---");
	let chat_request = ChatRequest::default()
		.with_system("Answer in one sentence")
		.append_message(ChatMessage::user("Why is the sky blue?"));
	let stream = client.exec_chat_stream(CLAUDE_MODEL, chat_request, None).await?;
	print_chat_stream(stream, None).await?;

	// -- Example 2: Amazon Nova via Bedrock Converse + SigV4
	println!("\n--- Nova on Bedrock (SigV4) ---");
	let chat_request = ChatRequest::default()
		.with_system("Answer in one sentence")
		.append_message(ChatMessage::user("Why is the sky blue?"));
	let stream = client.exec_chat_stream(NOVA_MODEL, chat_request, None).await?;
	print_chat_stream(stream, None).await?;

	Ok(())
}
