use crate::adapter::AdapterKind;
use crate::chat::{CacheControl, ChatOptionsSet, ChatRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenAiProtocol {
	ChatCompletions,
	Responses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenAiPromptCacheMode {
	Implicit,
	Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenAiPromptCachePolicy {
	pub(crate) mode: OpenAiPromptCacheMode,
	pub(crate) ttl: Option<&'static str>,
	pub(crate) controlled_message_count: usize,
}

pub(crate) fn is_gpt_5_6_or_later(model_name: &str) -> bool {
	let Some(version_and_suffix) = model_name.strip_prefix("gpt-") else {
		return false;
	};

	let version = version_and_suffix
		.split_once('-')
		.map_or(version_and_suffix, |(version, _)| version);
	let mut components = version.split('.');

	let Some(major) = components.next().and_then(|value| value.parse::<u32>().ok()) else {
		return false;
	};
	let Some(minor) = components.next().and_then(|value| value.parse::<u32>().ok()) else {
		return false;
	};

	major > 5 || major == 5 && minor >= 6
}

pub(crate) fn openai_prompt_cache_ttl(_cache_control: &CacheControl) -> &'static str {
	"30m"
}

pub(crate) fn openai_prompt_cache_policy(
	adapter_kind: AdapterKind,
	model_name: &str,
	chat_req: &ChatRequest,
	options: &ChatOptionsSet<'_, '_>,
	_protocol: OpenAiProtocol,
) -> Option<OpenAiPromptCachePolicy> {
	if !matches!(adapter_kind, AdapterKind::OpenAI | AdapterKind::OpenAIResp) || !is_gpt_5_6_or_later(model_name) {
		return None;
	}

	let controlled_message_count = chat_req
		.messages
		.iter()
		.filter(|message| {
			message
				.options
				.as_ref()
				.and_then(|options| options.cache_control.as_ref())
				.is_some()
		})
		.count();

	let has_explicit_placement = controlled_message_count > 0;
	let has_general_cache_intent = options.prompt_cache_key().is_some() || options.cache_control().is_some();

	let mode = if has_explicit_placement || !has_general_cache_intent {
		OpenAiPromptCacheMode::Explicit
	} else {
		OpenAiPromptCacheMode::Implicit
	};

	let has_cache_control = has_explicit_placement || options.cache_control().is_some();
	let ttl = has_cache_control.then(|| openai_prompt_cache_ttl_from_request(chat_req, options));

	Some(OpenAiPromptCachePolicy {
		mode,
		ttl,
		controlled_message_count,
	})
}

fn openai_prompt_cache_ttl_from_request(chat_req: &ChatRequest, options: &ChatOptionsSet<'_, '_>) -> &'static str {
	if let Some(cache_control) = options.cache_control() {
		return openai_prompt_cache_ttl(cache_control);
	}

	if let Some(cache_control) = chat_req
		.messages
		.iter()
		.find_map(|message| message.options.as_ref().and_then(|options| options.cache_control.as_ref()))
	{
		return openai_prompt_cache_ttl(cache_control);
	}

	"30m"
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::adapter::AdapterKind;
	use crate::chat::{ChatMessage, ChatOptions, Tool};

	#[test]
	fn test_adapter_adapters_openai_is_gpt_5_6_or_later() -> Result<()> {
		assert!(is_gpt_5_6_or_later("gpt-5.6"));
		assert!(is_gpt_5_6_or_later("gpt-5.6-mini"));
		assert!(is_gpt_5_6_or_later("gpt-5.6-preview"));
		assert!(is_gpt_5_6_or_later("gpt-5.10"));
		assert!(is_gpt_5_6_or_later("gpt-6.0"));
		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_is_gpt_5_6_or_later_rejects_older_names() -> Result<()> {
		assert!(!is_gpt_5_6_or_later("gpt-5.5"));
		assert!(!is_gpt_5_6_or_later("gpt-5"));
		assert!(!is_gpt_5_6_or_later("gpt-4.1"));
		assert!(!is_gpt_5_6_or_later("claude-sonnet-4-6"));
		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_prompt_cache_ttl_maps_all_controls() -> Result<()> {
		let controls = [
			CacheControl::Ephemeral,
			CacheControl::Memory,
			CacheControl::Ephemeral5m,
			CacheControl::Ephemeral1h,
			CacheControl::Ephemeral24h,
		];

		for control in &controls {
			assert_eq!(openai_prompt_cache_ttl(control), "30m");
		}

		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_prompt_cache_policy_no_configuration_is_explicit() -> Result<()> {
		let request = ChatRequest::from_user("hello");
		let options = ChatOptionsSet::default();
		let policy = openai_prompt_cache_policy(
			AdapterKind::OpenAI,
			"gpt-5.6",
			&request,
			&options,
			OpenAiProtocol::ChatCompletions,
		)
		.ok_or("supported OpenAI model should have a cache policy")?;

		assert_eq!(policy.mode, OpenAiPromptCacheMode::Explicit);
		assert_eq!(policy.ttl, None);
		assert_eq!(policy.controlled_message_count, 0);
		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_prompt_cache_policy_general_intent_is_implicit() -> Result<()> {
		let request = ChatRequest::from_user("hello");
		let chat_options = ChatOptions::default().with_prompt_cache_key("stable-key");
		let options = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
		let policy = openai_prompt_cache_policy(
			AdapterKind::OpenAI,
			"gpt-5.6-mini",
			&request,
			&options,
			OpenAiProtocol::ChatCompletions,
		)
		.ok_or("supported OpenAI model should have a cache policy")?;

		assert_eq!(policy.mode, OpenAiPromptCacheMode::Implicit);
		assert_eq!(policy.ttl, None);
		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_prompt_cache_policy_message_control_is_explicit() -> Result<()> {
		let request = ChatRequest::new(vec![
			ChatMessage::user("stable content").with_options(CacheControl::Ephemeral),
		]);
		let options = ChatOptionsSet::default();
		let policy = openai_prompt_cache_policy(
			AdapterKind::OpenAIResp,
			"gpt-5.6",
			&request,
			&options,
			OpenAiProtocol::Responses,
		)
		.ok_or("supported OpenAI model should have a cache policy")?;

		assert_eq!(policy.mode, OpenAiPromptCacheMode::Explicit);
		assert_eq!(policy.ttl, Some("30m"));
		assert_eq!(policy.controlled_message_count, 1);
		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_prompt_cache_policy_ignores_tool_control() -> Result<()> {
		let tool = Tool::new("get_weather").with_cache_control(CacheControl::Ephemeral);
		let request = ChatRequest::from_user("hello").append_tool(tool);
		let chat_options = ChatOptions::default().with_prompt_cache_key("stable-key");
		let options = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
		let policy = openai_prompt_cache_policy(
			AdapterKind::OpenAIResp,
			"gpt-5.6",
			&request,
			&options,
			OpenAiProtocol::Responses,
		)
		.ok_or("supported OpenAI model should have a cache policy")?;

		assert_eq!(policy.mode, OpenAiPromptCacheMode::Implicit);
		assert_eq!(policy.ttl, None);
		assert_eq!(policy.controlled_message_count, 0);
		Ok(())
	}

	#[test]
	fn test_adapter_adapters_openai_prompt_cache_policy_ignores_unsupported_scope() -> Result<()> {
		let request = ChatRequest::from_user("hello");
		let options = ChatOptionsSet::default();
		assert!(
			openai_prompt_cache_policy(
				AdapterKind::Together,
				"gpt-5.6",
				&request,
				&options,
				OpenAiProtocol::ChatCompletions,
			)
			.is_none()
		);
		assert!(
			openai_prompt_cache_policy(
				AdapterKind::OpenAI,
				"gpt-5.5",
				&request,
				&options,
				OpenAiProtocol::ChatCompletions,
			)
			.is_none()
		);
		Ok(())
	}
}

// endregion: --- Tests
