use super::DeepSeekAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ReasoningEffort};
use crate::resolver::{AuthData, Endpoint};
use crate::{ModelIden, ServiceTarget};
use serde_json::Value;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// region:    --- DeepSeek

#[test]
fn test_deepseek_managed_body_thinking_enables_non_zero_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = Some(ReasoningEffort::Max);

	// -- Exec
	let payload = support_deepseek_payload(reasoning_effort)?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "enabled");
	assert_eq!(payload["reasoning_effort"], "max");

	Ok(())
}

#[test]
fn test_deepseek_managed_body_thinking_disables_zero_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = Some(ReasoningEffort::Zero);

	// -- Exec
	let payload = support_deepseek_payload(reasoning_effort)?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "disabled");
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

#[test]
fn test_deepseek_managed_body_thinking_omits_fields_without_effort() -> Result<()> {
	// -- Setup & Fixtures
	let reasoning_effort = None;

	// -- Exec
	let payload = support_deepseek_payload(reasoning_effort)?;

	// -- Check
	assert!(payload.get("thinking").is_none());
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

// endregion: --- DeepSeek

// region:    --- Support

fn support_deepseek_payload(reasoning_effort: Option<ReasoningEffort>) -> Result<Value> {
	let chat_options = reasoning_effort.map(|effort| ChatOptions::default().with_reasoning_effort(effort));
	let options_set = ChatOptionsSet::default().with_chat_options(chat_options.as_ref());
	let request = DeepSeekAdapter::to_web_request_data(
		ServiceTarget {
			model: ModelIden::new(AdapterKind::DeepSeek, "deepseek-v4-flash"),
			auth: AuthData::from_single("test-key"),
			endpoint: Endpoint::from_static("https://api.deepseek.com/v1/"),
		},
		ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
	)?;

	Ok(request.payload)
}

// endregion: --- Support
