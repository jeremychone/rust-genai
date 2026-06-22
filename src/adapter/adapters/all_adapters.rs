//! Central module for all adapter types.
//!
//! This module re-exports all adapter structs, whether custom (full protocol
//! implementations) or pass-through (macro-generated delegating adapters). It
//! serves as the single entry point for the rest of the crate.

use crate::adapter::AdapterKind;
use crate::impl_pass_through_adapter;

// region:    --- Custom adapters (full protocol or specific implementations)

// -- Protocol Adapters
pub use super::anthropic::AnthropicAdapter;
pub use super::cohere::CohereAdapter;
pub use super::gemini::GeminiAdapter;
pub use super::ollama::OllamaAdapter;
pub use super::openai::OpenAIAdapter;
pub use super::openai_resp::OpenAIRespAdapter;

// -- Specific Adapters
pub use super::bedrock::BedrockApiAdapter;
#[cfg(feature = "bedrock-sigv4")]
pub use super::bedrock::BedrockSigv4Adapter;
pub use super::bigmodel::BigModelAdapter;
pub use super::custom::CustomAdapter;
pub use super::fireworks::FireworksAdapter;
pub use super::ollama_cloud::OllamaCloudAdapter;
pub use super::vertex::VertexAdapter;
// has 2 protocol providers
pub use super::baidu::BaiduAdapter;
pub use super::opencode_go::OpenCodeGoAdapter;
// Has unit tests
pub use super::github_copilot::GithubCopilotAdapter;
// Has namespace based endpoint routing
pub use super::zai::ZaiAdapter;

// -- To review (might be able to be Pass-through)
pub use super::aliyun::AliyunAdapter;
pub use super::groq::GroqAdapter;
pub use super::open_router::OpenRouterAdapter;
pub use super::xai::XaiAdapter;

// endregion: --- Custom adapters

// region:    --- Pass-through adapters (with macros)

// -- Aihubmix
pub struct AihubmixAdapter;
impl_pass_through_adapter!(
	name: AihubmixAdapter,
	kind: AdapterKind::Aihubmix,
	key_env: Some("AIHUBMIX_API_KEY"),
	endpoint: "https://aihubmix.com/v1/",
	delegate: OpenAIAdapter,
);

// -- DeepSeek
pub struct DeepSeekAdapter;
impl_pass_through_adapter!(
	name: DeepSeekAdapter,
	kind: AdapterKind::DeepSeek,
	key_env: Some("DEEPSEEK_API_KEY"),
	endpoint: "https://api.deepseek.com/v1/",
	delegate: OpenAIAdapter,
);

// -- MiniMax (AnthropicAdapter, and fix/empty model names for now)
pub struct MinimaxAdapter;
impl_pass_through_adapter!(
	name: MinimaxAdapter,
	kind: AdapterKind::MiniMax,
	key_env: Some("MINIMAX_API_KEY"),
	endpoint: "https://api.minimax.io/anthropic/v1/",
	delegate: AnthropicAdapter,
	all_model_names: |_kind, _endpoint, _auth, _web_client| {
		Ok(vec![])
	},
	unsupported: [embeddings],
);

// -- Mimo
pub struct MimoAdapter;
impl_pass_through_adapter!(
	name: MimoAdapter,
	kind: AdapterKind::Mimo,
	key_env: Some("MIMO_API_KEY"),
	endpoint: "https://api.mimo.com/openai/v1/",
	delegate: OpenAIAdapter,
);

// -- Moonshot
pub struct MoonshotAdapter;
impl_pass_through_adapter!(
	name: MoonshotAdapter,
	kind: AdapterKind::Moonshot,
	key_env: Some("MOONSHOT_API_KEY"),
	endpoint: "https://api.moonshot.cn/v1/",
	delegate: OpenAIAdapter,
);

// -- Nebius
pub struct NebiusAdapter;
impl_pass_through_adapter!(
	name: NebiusAdapter,
	kind: AdapterKind::Nebius,
	key_env: Some("NEBIUS_API_KEY"),
	endpoint: "https://api.studio.nebius.ai/v1/",
	delegate: OpenAIAdapter,
);

// -- TogetherAdapter
pub struct TogetherAdapter;
impl_pass_through_adapter!(
	name: TogetherAdapter,
	kind: AdapterKind::Together,
	key_env: Some("TOGETHER_API_KEY"),
	endpoint: "https://api.together.xyz/v1/",
	delegate: OpenAIAdapter,
);

// endregion: --- Pass-through adapters (with macros)
