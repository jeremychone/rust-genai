//! Cost cap: run a chat loop with a hard USD budget from the llm-budget crate.
//!
//! - Before each call, the worst case is reserved (prompt estimate plus
//!   `max_tokens` of output). If that would cross the cap, the question is
//!   skipped and no request goes out.
//! - After the call, the hold is settled with what the provider reported.
//!   genai's `prompt_tokens` already includes cache reads and cache writes,
//!   so they are split out and billed at their own rates (a cache read costs
//!   a tenth of fresh input on Anthropic) instead of all at the input price.
//!
//! Uses claude-haiku-4-5 (needs ANTHROPIC_API_KEY), or gpt-5-nano when only
//! OPENAI_API_KEY is set.

use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Usage};
use llm_budget::AgentBudget;

const MODEL_ANTHROPIC: &str = "claude-haiku-4-5";
const MODEL_OPENAI: &str = "gpt-5-nano";
const MAX_OUTPUT_TOKENS: u32 = 64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let model = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
		MODEL_ANTHROPIC
	} else {
		MODEL_OPENAI
	};

	let client = Client::new()?;
	let options = ChatOptions::default().with_max_tokens(MAX_OUTPUT_TOKENS);
	// $0.002 covers a few one-word answers, but not all of them.
	let budget = AgentBudget::builder("chat-loop", 0.002).build()?;

	for question in [
		"What is the capital of France?",
		"And of Japan?",
		"And of Brazil?",
		"And of Canada?",
		"And of Australia?",
		"And of Kenya?",
		"And of Chile?",
		"And of Egypt?",
	] {
		let chat_req = ChatRequest::new(vec![
			ChatMessage::system("Answer in one word."),
			ChatMessage::user(question),
		]);

		// Reserve the worst case before sending anything.
		let estimate = llm_budget::Usage::new(estimate_prompt_tokens(&chat_req), MAX_OUTPUT_TOKENS as u64);
		let hold = match budget.reserve_for(model, &estimate) {
			Ok(hold) => hold,
			Err(err) => {
				println!("{question}\n  -> skipped, not sent: {err}");
				continue;
			}
		};

		let chat_res = match client.exec_chat(model, chat_req, Some(&options)).await {
			Ok(res) => res,
			Err(err) => {
				drop(hold); // releases the reservation, nothing is booked
				return Err(err.into());
			}
		};
		let answer = chat_res.first_text().unwrap_or("NO ANSWER").to_string();

		// Book what the call actually cost, cache reads and writes at their own rates.
		let outcome = hold.settle_usage(model, &billable_usage(&chat_res.usage));
		println!(
			"{question}\n  -> {answer} (${:.6} this call, ${:.6} of ${:.4} spent)",
			outcome.recorded_usd,
			outcome.total_spent_usd,
			budget.limit_usd()
		);
		if outcome.over_cap {
			println!("  -> cap reached, stopping");
			break;
		}
	}

	Ok(())
}

/// genai normalizes usage across providers: `prompt_tokens` includes cache
/// reads (`cached_tokens`) and cache writes (`cache_creation_tokens`, or the
/// 5m/1h breakdown when the provider gives one), and `completion_tokens`
/// includes reasoning tokens. Split the prompt so each part is priced right.
fn billable_usage(usage: &Usage) -> llm_budget::Usage {
	let count = |v: Option<i32>| u64::try_from(v.unwrap_or(0)).unwrap_or(0);
	let input = count(usage.prompt_tokens);
	let details = usage.prompt_tokens_details.as_ref();
	let cache_read = count(details.and_then(|d| d.cached_tokens)).min(input);
	let cache_write = details
		.map(|d| {
			let by_ttl = d
				.cache_creation_details
				.as_ref()
				.map(|c| count(c.ephemeral_5m_tokens) + count(c.ephemeral_1h_tokens))
				.unwrap_or(0);
			count(d.cache_creation_tokens).max(by_ttl)
		})
		.unwrap_or(0)
		.min(input - cache_read);
	llm_budget::Usage::from_parts(
		input - cache_read - cache_write,
		cache_read,
		cache_write,
		count(usage.completion_tokens),
	)
}

/// Rough prompt size for the reservation: about 4 characters per token plus
/// a little per message. The real count from the provider replaces it.
fn estimate_prompt_tokens(req: &ChatRequest) -> u64 {
	let chars: usize = req.system.as_deref().map(str::len).unwrap_or(0)
		+ req
			.messages
			.iter()
			.map(|m| m.content.joined_texts().map(|t| t.len()).unwrap_or(0))
			.sum::<usize>();
	(chars / 4 + 8 * (req.messages.len() + 1)) as u64
}
