//! USD cost of a response from its `Usage` (feature `pricing`, off by default).
//!
//! Prices come from the [`llm-prices`](https://docs.rs/llm-prices) crate: a dated table of
//! list prices per 1M tokens where each row cites the vendor pricing page it was read from.
//! genai keeps no price table of its own; it only maps its normalized `Usage` onto that table.
//!
//! What is priced:
//! - Uncached input, output (reasoning tokens included, as `completion_tokens` already counts them),
//!   cache reads (`cached_tokens`), and cache writes (`cache_creation_tokens`, split by
//!   `cache_creation_details` into 5-minute and 1-hour lifetimes; writes without a breakdown
//!   count as 5-minute, the Anthropic default).
//! - Long-context tiers apply when the prompt is over the model's threshold.
//!
//! What returns `None`:
//! - Adapters that are gateways, local servers, or providers the table does not list (e.g., Ollama).
//! - Models the table does not list for that provider.
//! - A `Usage` with no prompt or completion counts.
//! - Cache writes on a model with no matching cache-write price.
//!
//! The price is chosen by `AdapterKind`, so a request routed to another endpoint with a
//! `ServiceTargetResolver` is still priced as the vendor's own API.

use crate::ModelIden;
use crate::adapter::AdapterKind;
use crate::chat::{ChatResponse, Usage};

impl Usage {
	/// List-price cost of this usage in USD for `model_iden`, from the `llm-prices` table.
	///
	/// Cache reads and cache writes are billed at their own rates, so a response that mostly
	/// reads from the prompt cache costs what the provider charges for it, not
	/// `prompt_tokens` times the input price.
	///
	/// Returns `None` when the provider or model is not in the table, or when this usage
	/// carries no token counts (see the module docs).
	pub fn cost_usd(&self, model_iden: &ModelIden) -> Option<f64> {
		if self.prompt_tokens.is_none() && self.completion_tokens.is_none() {
			return None;
		}
		let provider = pricing_provider(model_iden.adapter_kind)?;
		let (_, model_name) = model_iden.model_name.namespace_and_name();
		let price = provider.lookup(model_name)?;
		price.cost_usage(&self.to_llm_prices_usage()).map(|cost| cost.total)
	}

	/// Splits genai's normalized counts (where `prompt_tokens` includes cache reads and writes)
	/// into the disjoint buckets `llm_prices::Usage` bills.
	fn to_llm_prices_usage(&self) -> llm_prices::Usage {
		let prompt = non_negative(self.prompt_tokens);
		let output = non_negative(self.completion_tokens);

		let details = self.prompt_tokens_details.as_ref();
		let cache_read = non_negative(details.and_then(|d| d.cached_tokens));
		let cache_creation = non_negative(details.and_then(|d| d.cache_creation_tokens));
		let ttl = details.and_then(|d| d.cache_creation_details.as_ref());
		let write_5m = non_negative(ttl.and_then(|t| t.ephemeral_5m_tokens));
		let write_1h = non_negative(ttl.and_then(|t| t.ephemeral_1h_tokens));

		// Cache creation tokens not covered by the TTL breakdown default to the 5-minute lifetime.
		let unsplit = cache_creation.saturating_sub(write_5m.saturating_add(write_1h));
		let cache_write_5m = write_5m.saturating_add(unsplit);
		let cache_write_1h = write_1h;

		let cached_total = cache_read.saturating_add(cache_write_5m).saturating_add(cache_write_1h);

		llm_prices::Usage {
			input: prompt.saturating_sub(cached_total),
			output,
			cache_read,
			cache_write_5m,
			cache_write_1h,
			batch: false,
		}
	}
}

impl ChatResponse {
	/// List-price cost of this response in USD, from `self.usage` (feature `pricing`).
	///
	/// Uses the provider-reported model (often a dated snapshot) first, then the resolved
	/// model. See [`Usage::cost_usd`].
	pub fn cost_usd(&self) -> Option<f64> {
		self.usage
			.cost_usd(&self.provider_model_iden)
			.or_else(|| self.usage.cost_usd(&self.model_iden))
	}
}

/// The `llm-prices` provider whose list prices apply to an adapter, or `None` when the adapter
/// is a gateway or host the table does not price.
fn pricing_provider(adapter_kind: AdapterKind) -> Option<llm_prices::Provider> {
	use llm_prices::Provider;

	let provider = match adapter_kind {
		AdapterKind::OpenAI | AdapterKind::OpenAIResp => Provider::OpenAi,
		AdapterKind::Gemini | AdapterKind::GeminiIx => Provider::Google,
		AdapterKind::Anthropic => Provider::Anthropic,
		AdapterKind::Fireworks => Provider::Fireworks,
		AdapterKind::Together => Provider::Together,
		AdapterKind::Groq => Provider::Groq,
		AdapterKind::Xai => Provider::Xai,
		AdapterKind::DeepSeek => Provider::DeepSeek,
		AdapterKind::Mistral => Provider::Mistral,
		AdapterKind::Cohere => Provider::Cohere,
		AdapterKind::Vertex => Provider::Vertex,
		AdapterKind::BedrockApi => Provider::Bedrock,
		#[cfg(feature = "bedrock-sigv4")]
		AdapterKind::BedrockSigv4 => Provider::Bedrock,
		AdapterKind::OpenRouter => Provider::OpenRouter,
		_ => return None,
	};

	Some(provider)
}

fn non_negative(tokens: Option<i32>) -> u64 {
	tokens.map(|t| t.max(0) as u64).unwrap_or(0)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;
	use crate::chat::{CacheCreationDetails, MessageContent, PromptTokensDetails};
	use llm_prices::Rate;

	const M: f64 = 1_000_000.0;

	fn usage(prompt: i32, completion: i32, details: Option<PromptTokensDetails>) -> Usage {
		Usage {
			prompt_tokens: Some(prompt),
			prompt_tokens_details: details,
			completion_tokens: Some(completion),
			completion_tokens_details: None,
			total_tokens: Some(prompt + completion),
		}
	}

	fn rate(provider: llm_prices::Provider, model: &str, rate: Rate<'_>) -> Result<f64> {
		provider
			.lookup(model)
			.and_then(|p| p.rate(rate))
			.ok_or_else(|| format!("no {rate:?} price for {model}").into())
	}

	fn assert_close(actual: f64, expected: f64) {
		assert!(
			(actual - expected).abs() < 1e-12,
			"actual {actual}, expected {expected}"
		);
	}

	#[test]
	fn test_pricing_openai_plain_usage() -> Result<()> {
		// -- Setup & Fixtures
		let model = ModelIden::new(AdapterKind::OpenAI, "gpt-4o");
		let usage = usage(10_000, 2_000, None);
		let input = rate(llm_prices::Provider::OpenAi, "gpt-4o", Rate::Input)?;
		let output = rate(llm_prices::Provider::OpenAi, "gpt-4o", Rate::Output)?;

		// -- Exec
		let cost = usage.cost_usd(&model).ok_or("should price gpt-4o")?;

		// -- Check
		assert_close(cost, (10_000.0 * input + 2_000.0 * output) / M);

		Ok(())
	}

	#[test]
	fn test_pricing_anthropic_cache_read_and_write_billed_at_their_rates() -> Result<()> {
		// -- Setup & Fixtures
		// Normalized like the Anthropic adapter: prompt = input (1_000) + cache creation (3_000) + cache read (96_000).
		let model = ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5");
		let usage = usage(
			100_000,
			500,
			Some(PromptTokensDetails {
				cache_creation_tokens: Some(3_000),
				cache_creation_details: Some(CacheCreationDetails {
					ephemeral_5m_tokens: Some(2_000),
					ephemeral_1h_tokens: Some(1_000),
				}),
				cached_tokens: Some(96_000),
				audio_tokens: None,
			}),
		);
		let p = llm_prices::Provider::Anthropic;
		let m = "claude-haiku-4-5";
		let expected = (1_000.0 * rate(p, m, Rate::Input)?
			+ 96_000.0 * rate(p, m, Rate::CacheRead)?
			+ 2_000.0 * rate(p, m, Rate::CacheWrite("5m"))?
			+ 1_000.0 * rate(p, m, Rate::CacheWrite("1h"))?
			+ 500.0 * rate(p, m, Rate::Output)?)
			/ M;
		let naive = (100_000.0 * rate(p, m, Rate::Input)? + 500.0 * rate(p, m, Rate::Output)?) / M;

		// -- Exec
		let cost = usage.cost_usd(&model).ok_or("should price claude-haiku-4-5")?;

		// -- Check
		assert_close(cost, expected);
		// Charging every prompt token at the input price overstates a cache-heavy call.
		assert!(cost < naive / 3.0, "cost {cost} should be far below naive {naive}");

		Ok(())
	}

	#[test]
	fn test_pricing_cache_creation_without_ttl_breakdown_is_5m() -> Result<()> {
		// -- Setup & Fixtures
		let model = ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5");
		let usage = usage(
			5_000,
			0,
			Some(PromptTokensDetails {
				cache_creation_tokens: Some(4_000),
				..Default::default()
			}),
		);
		let p = llm_prices::Provider::Anthropic;
		let m = "claude-haiku-4-5";
		let expected = (1_000.0 * rate(p, m, Rate::Input)? + 4_000.0 * rate(p, m, Rate::CacheWrite("5m"))?) / M;

		// -- Exec
		let cost = usage.cost_usd(&model).ok_or("should price cache writes")?;

		// -- Check
		assert_close(cost, expected);

		Ok(())
	}

	#[test]
	fn test_pricing_namespaced_and_platform_models() -> Result<()> {
		// -- Setup & Fixtures
		let usage = usage(1_000, 1_000, None);
		let anthropic = usage
			.cost_usd(&ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"))
			.ok_or("anthropic")?;

		// -- Exec
		let namespaced = usage.cost_usd(&ModelIden::new(AdapterKind::Anthropic, "anthropic::claude-haiku-4-5"));
		let vertex = usage.cost_usd(&ModelIden::new(AdapterKind::Vertex, "vertex::claude-haiku-4-5"));

		// -- Check
		// Vertex global endpoints bill Claude at Anthropic's list price.
		assert_eq!(namespaced, Some(anthropic));
		assert_eq!(vertex, Some(anthropic));

		Ok(())
	}

	#[test]
	fn test_pricing_unknown_is_none() -> Result<()> {
		// -- Setup & Fixtures
		let usage = usage(1_000, 1_000, None);

		// -- Exec & Check
		assert!(
			usage
				.cost_usd(&ModelIden::new(AdapterKind::OpenAI, "not-a-real-model-xyz"))
				.is_none()
		);
		assert!(usage.cost_usd(&ModelIden::new(AdapterKind::Ollama, "gemma3:4b")).is_none());
		// Groq does not serve Claude, so no other vendor's price is borrowed.
		assert!(usage.cost_usd(&ModelIden::new(AdapterKind::Groq, "claude-haiku-4-5")).is_none());
		// No counts (e.g., a stream without `capture_usage`) is not a $0 call.
		let empty = Usage::default();
		assert!(empty.cost_usd(&ModelIden::new(AdapterKind::OpenAI, "gpt-4o")).is_none());

		Ok(())
	}

	#[test]
	fn test_pricing_chat_response_uses_provider_model() -> Result<()> {
		// -- Setup & Fixtures
		let res = ChatResponse {
			content: MessageContent::default(),
			reasoning_content: None,
			model_iden: ModelIden::new(AdapterKind::Anthropic, "my-alias"),
			provider_model_iden: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5-20251001"),
			stop_reason: None,
			usage: usage(10_000, 1_000, None),
			captured_raw_body: None,
			response_id: None,
		};
		let expected = res
			.usage
			.cost_usd(&ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"))
			.ok_or("anthropic")?;

		// -- Exec
		let cost = res.cost_usd().ok_or("should price the dated snapshot")?;

		// -- Check
		assert_close(cost, expected);

		Ok(())
	}
}

// endregion: --- Tests
