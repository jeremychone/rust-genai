//! Cost cap: run a chat loop with a hard USD budget from the llm-budget
//! crate. Each response's provider-reported usage is charged at the model's
//! real price; once the cap is spent, later questions are refused before any
//! API call goes out.
//!
//! Uses claude-haiku-4-5 (needs ANTHROPIC_API_KEY), or gpt-5-nano when only
//! OPENAI_API_KEY is set.

#![allow(unused)]

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};
use llm_budget::AgentBudget;

const MODEL_ANTHROPIC: &str = "claude-haiku-4-5";
const MODEL_OPENAI: &str = "gpt-5-nano";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let model = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
		MODEL_ANTHROPIC
	} else {
		MODEL_OPENAI
	};

	let client = Client::new()?;
	// $0.01 covers several short haiku answers, but not dozens.
	let budget = AgentBudget::builder("chat-loop", 0.01).build()?;

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
		if budget.is_exhausted() {
			println!("{question}\n  -> skipped: ${:.4} of ${:.4} spent", budget.spent_usd(), budget.limit_usd());
			continue;
		}

		let chat_req = ChatRequest::new(vec![
			ChatMessage::system("Answer in one word."),
			ChatMessage::user(question),
		]);
		let chat_res = client.exec_chat(model, chat_req, None).await?;
		let answer = chat_res.first_text().unwrap_or("NO ANSWER").to_string();
		let usage = chat_res.usage;

		let input = u32::try_from(usage.prompt_tokens.unwrap_or(0)).unwrap_or(0);
		let output = u32::try_from(usage.completion_tokens.unwrap_or(0)).unwrap_or(0);
		let receipt = budget.charge_model(model, input, output, None, None).await?;

		println!(
			"{question}\n  -> {answer} (${:.6} this call, ${:.6} of ${:.4} spent)",
			receipt.cost_usd, receipt.total_spent_usd, budget.limit_usd()
		);
	}

	Ok(())
}
