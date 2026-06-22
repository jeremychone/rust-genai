//! Central module for all adapter types.
//!
//! This module re-exports all adapter structs, whether custom (full protocol
//! implementations) or pass-through (macro-generated delegating adapters). It
//! serves as the single entry point for the rest of the crate.

#![allow(unused_imports)]

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
pub use super::fireworks::FireworksAdapter;
pub use super::vertex::VertexAdapter;

// endregion: --- Custom adapters

// region:    --- Pass-through adapters (with macros)

// -- MiniMax
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

// region:    --- Pass-through adapters (still manual)

pub use super::aihubmix::AihubmixAdapter;
pub use super::aliyun::AliyunAdapter;
pub use super::baidu::BaiduAdapter;
pub use super::bigmodel::BigModelAdapter;
pub use super::custom::CustomAdapter;
pub use super::deepseek::DeepSeekAdapter;
pub use super::github_copilot::GithubCopilotAdapter;
pub use super::groq::GroqAdapter;
pub use super::moonshot::MoonshotAdapter;
pub use super::ollama_cloud::OllamaCloudAdapter;
pub use super::open_router::OpenRouterAdapter;
pub use super::opencode_go::OpenCodeGoAdapter;
pub use super::xai::XaiAdapter;
pub use super::zai::ZaiAdapter;

// endregion: --- Pass-through adapters
