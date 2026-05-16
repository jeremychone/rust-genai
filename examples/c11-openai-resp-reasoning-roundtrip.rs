//! Demonstrate the OpenAI Responses prefix-cache + reasoning round-trip.
//!
//! Reasoning models on the Responses API (gpt-5.x, o-series) emit
//! `type:"reasoning"` items with an `encrypted_content` blob during
//! streaming. To maintain reasoning state — and to keep the
//! provider-side prefix cache warm — the next request in a
//! conversation must replay those blobs as inputs alongside the
//! assistant message they belong to.
//!
//! This example performs two turns:
//!   1. Ask a reasoning-triggering question with `reasoning_effort=Medium`
//!      and `capture_reasoning_content=true` (which causes the adapter
//!      to set `include: ["reasoning.encrypted_content"]` on the request).
//!   2. Re-send the conversation, attaching the captured
//!      `ContentPart::ThoughtSignature` parts to the assistant turn.
//!      The adapter emits each blob as a top-level `type:"reasoning"`
//!      input item before the assistant message.
//!
//! Inspect `Usage.prompt_tokens_details.cached_tokens` on turn 2: when
//! the backend honors prompt caching and the prefix matches, it will be
//! non-zero. Cache-hit ratios vary by backend and conversation shape.
//!
//! Requires: OPENAI_API_KEY environment variable.
//!
//! Run: `OPENAI_API_KEY=sk-... cargo run --example c11-openai-resp-reasoning-roundtrip`

use std::env;

use futures::StreamExt;
use genai::Client;
use genai::ModelIden;
use genai::adapter::AdapterKind;
use genai::chat::{
	ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, ContentPart, MessageContent, ReasoningEffort,
};
use genai::resolver::{AuthData, AuthResolver};

/// A small reasoning-class model on the Responses API. Bump to a larger
/// model (e.g. `gpt-5.1`, `gpt-5`) for richer reasoning traces — the
/// shape of the response stream and the round-trip protocol is the same.
const MODEL: &str = "gpt-5.4-mini";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let api_key =
		env::var("OPENAI_API_KEY").map_err(|_| "set OPENAI_API_KEY to a key with access to the Responses API")?;

	let auth_resolver =
		AuthResolver::from_resolver_fn(move |_iden: ModelIden| Ok(Some(AuthData::from_single(api_key.clone()))));
	let client = Client::builder()
		.with_adapter_kind(AdapterKind::OpenAIResp)
		.with_auth_resolver(auth_resolver)
		.build();

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_usage(true)
		.with_capture_reasoning_content(true)
		.with_reasoning_effort(ReasoningEffort::Medium)
		// Stable across turns in a session — lets the Responses API
		// recognise the cacheable prefix even when content hashes shift
		// in cheap ways. Hit-rate shows up as `cached_tokens` in usage.
		.with_prompt_cache_key("genai-example-reasoning-roundtrip");

	// ============== Turn 1 ==============
	let req1 = ChatRequest::default()
		.with_system("You are a careful assistant. Show your reasoning before each answer.")
		.append_message(ChatMessage::user(
			"A farmer has 17 cows. If all but 9 die, how many are left? \
             Reason carefully — many people get this wrong by misreading.",
		));
	let (text1, sigs1, usage1) = run_turn(&client, MODEL, req1.clone(), &options).await?;
	print_turn(1, &text1, &sigs1, &usage1);

	// ============== Turn 2 ==============
	// Build the assistant turn with the captured reasoning blobs FIRST
	// (the adapter emits them as top-level `type:"reasoning"` input
	// items that precede the assistant message), then the text reply.
	let mut asst_content = MessageContent::default();
	for blob in &sigs1 {
		asst_content.push(ContentPart::ThoughtSignature(blob.clone()));
	}
	if !text1.is_empty() {
		asst_content.push(ContentPart::Text(text1.clone()));
	}

	let req2 = req1
		.append_message(ChatMessage::assistant(asst_content))
		.append_message(ChatMessage::user(
			"What if the puzzle said 'all but 3 die' instead? Reason briefly.",
		));
	let (text2, sigs2, usage2) = run_turn(&client, MODEL, req2, &options).await?;
	print_turn(2, &text2, &sigs2, &usage2);

	Ok(())
}

#[derive(Default)]
struct TurnUsage {
	prompt: i32,
	completion: i32,
	cached: i32,
	reasoning: i32,
}

fn print_turn(n: usize, text: &str, sigs: &[String], u: &TurnUsage) {
	println!("\n=== turn {n} ===");
	println!(
		"  usage: prompt={} cached={} completion={} reasoning_tokens={}",
		u.prompt, u.cached, u.completion, u.reasoning
	);
	println!("  thought_signatures captured: {}", sigs.len());
	let preview: String = text.lines().take(6).collect::<Vec<_>>().join("\n  ");
	println!("  reply:\n  {preview}");
}

async fn run_turn(
	client: &Client,
	model: &str,
	req: ChatRequest,
	options: &ChatOptions,
) -> Result<(String, Vec<String>, TurnUsage), Box<dyn std::error::Error>> {
	let stream_res = client.exec_chat_stream(model, req, Some(options)).await?;
	let mut stream = stream_res.stream;
	let mut text = String::new();
	let mut end_opt = None;
	while let Some(event) = stream.next().await {
		match event? {
			ChatStreamEvent::Chunk(c) => text.push_str(&c.content),
			ChatStreamEvent::End(e) => {
				end_opt = Some(e);
				break;
			}
			_ => {}
		}
	}
	let end = end_opt.ok_or("stream ended without End event")?;
	let sigs: Vec<String> = end
		.captured_thought_signatures()
		.map(|v| v.into_iter().map(String::from).collect())
		.unwrap_or_default();
	let usage = end.captured_usage.as_ref();
	let prompt = usage.and_then(|u| u.prompt_tokens).unwrap_or(0);
	let completion = usage.and_then(|u| u.completion_tokens).unwrap_or(0);
	let cached = usage
		.and_then(|u| u.prompt_tokens_details.as_ref())
		.and_then(|d| d.cached_tokens)
		.unwrap_or(0);
	let reasoning = usage
		.and_then(|u| u.completion_tokens_details.as_ref())
		.and_then(|d| d.reasoning_tokens)
		.unwrap_or(0);
	let captured_text = end
		.captured_content
		.as_ref()
		.and_then(|c| c.first_text())
		.map(String::from)
		.unwrap_or(text);
	Ok((
		captured_text,
		sigs,
		TurnUsage {
			prompt,
			completion,
			cached,
			reasoning,
		},
	))
}
