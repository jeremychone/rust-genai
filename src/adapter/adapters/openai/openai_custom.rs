//! OpenAI Custom Types to allow OpenAI adapter variant to customize the default OpenAI Behavior

/// Custom OpenAI structure for Adapters to use to customize
/// the default [`OpenAIAdapter::util_to_web_request_data`]
pub struct ToWebRequestCustom {
	pub default_max_tokens: Option<u32>,
}
