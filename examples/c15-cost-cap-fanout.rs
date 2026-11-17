//! Cost cap with fan-out: sub-agents run in parallel, each on a slice of one budget.
//!
//! c14-cost-cap caps one chat loop. When an orchestrator hands work to several
//! sub-agents at once, telling each one "what is left" lets them together spend a
//! multiple of it, because they all read it before any of them spends. Here each
//! sub-agent gets a slice reserved from the orchestrator's budget before it starts
//! (`AgentBudget::child` in llm-budget):
//!
//! - The slices are taken atomically, so they never add up to more than the
//!   orchestrator holds. A slice that does not fit is refused before any request
//!   goes out.
//! - Inside its slice a sub-agent reserves before each call and settles after it,
//!   as in c14. Everything it books counts on the orchestrator at once.
//! - When a sub-agent finishes, the unused part of its slice goes back.
//!
//! Uses claude-haiku-4-5 (needs ANTHROPIC_API_KEY), or gpt-5-nano when only
//! OPENAI_API_KEY is set.

use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Usage};
use llm_budget::{AgentBudget, Limit, ScopeGuard};

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
	let orchestrator = AgentBudget::builder("orchestrator", 0.004).build()?;

	// One sub-agent per country, all started together, $0.001 each:
	// the fifth slice does not fit in $0.004, so that sub-agent never starts.
	let mut sub_agents = Vec::new();
	for country in ["France", "Japan", "Brazil", "Canada", "Kenya"] {
		let slice = match orchestrator.child(country, Limit::Usd(0.001)) {
			Ok(slice) => slice,
			Err(err) => {
				println!("{country}: not started, {err}");
				continue;
			}
		};
		let client = client.clone();
		sub_agents.push(tokio::spawn(sub_agent(client, model, country, slice)));
	}

	for sub_agent in sub_agents {
		let report = sub_agent.await?;
		println!("{report}");
	}

	println!(
		"orchestrator: ${:.6} of ${:.4} spent, ${:.6} left",
		orchestrator.spent_usd(),
		orchestrator.limit_usd(),
		orchestrator.remaining_usd()
	);
	Ok(())
}

/// Asks a few questions about one country inside its slice of the budget.
async fn sub_agent(client: Client, model: &'static str, country: &'static str, slice: ScopeGuard) -> String {
	let options = ChatOptions::default().with_max_tokens(MAX_OUTPUT_TOKENS);
	let mut answers = Vec::new();

	for question in [
		"What is the capital of",
		"What is the currency of",
		"What is the largest city of",
	] {
		let chat_req = ChatRequest::new(vec![
			ChatMessage::system("Answer in one word."),
			ChatMessage::user(format!("{question} {country}?")),
		]);

		// Reserve the worst case inside this sub-agent's slice before sending anything.
		let estimate = llm_budget::Usage::new(estimate_prompt_tokens(&chat_req), MAX_OUTPUT_TOKENS as u64);
		let hold = match slice.reserve_for(model, &estimate) {
			Ok(hold) => hold,
			Err(err) => {
				answers.push(format!("(not sent: {err})"));
				break;
			}
		};

		match client.exec_chat(model, chat_req, Some(&options)).await {
			Ok(chat_res) => {
				// Booked on the slice and on the orchestrator at the same time.
				hold.settle_usage(model, &billable_usage(&chat_res.usage));
				answers.push(chat_res.first_text().unwrap_or("NO ANSWER").to_string());
			}
			Err(err) => {
				drop(hold); // releases the reservation, nothing is booked
				answers.push(format!("(error: {err})"));
				break;
			}
		}
	}

	// Close the slice: what it did not use goes back to the orchestrator.
	let receipt = slice.settle();
	format!(
		"{country}: {} (${:.6} spent of a ${:.4} slice, ${:.6} returned)",
		answers.join(", "),
		receipt.spent_usd,
		receipt.held_usd,
		receipt.released_usd
	)
}

/// Same as in c14-cost-cap: genai's `prompt_tokens` includes cache reads and
/// writes, so split them out to price each part at its own rate.
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
