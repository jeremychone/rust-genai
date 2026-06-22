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

// -- Cloud Infra providers
pub use super::bedrock::BedrockApiAdapter;
#[cfg(feature = "bedrock-sigv4")]
pub use super::bedrock::BedrockSigv4Adapter;
pub use super::vertex::VertexAdapter;

// -- Has custom max token
pub use super::fireworks::FireworksAdapter;

// -- Ollama prototocl
pub use super::ollama_cloud::OllamaCloudAdapter;

// -- Has 2 protocol providers
pub use super::baidu::BaiduAdapter;

// -- Has unit tests
pub use super::github_copilot::GithubCopilotAdapter;
pub use super::opencode_go::OpenCodeGoAdapter;

// -- Has namespace based endpoint routing
pub use super::zai::ZaiAdapter;

// -- Custom (the genai_n:: special adapter)
pub use super::custom::CustomAdapter;

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

// -- Aliyun
pub struct AliyunAdapter;
impl_pass_through_adapter!(
	name: AliyunAdapter,
	kind: AdapterKind::Aliyun,
	key_env: Some("ALIYUN_API_KEY"),
	endpoint: "https://dashscope.aliyuncs.com/compatible-mode/v1/",
	delegate: OpenAIAdapter,
);

// -- BigModel
/// BigModel adapter.
///
/// API Documentation: <https://bigmodel.cn/dev/api>
/// Model Names:       <https://bigmodel.cn/dev/howuse/model>
/// Pricing:           <https://bigmodel.cn/pricing>
pub struct BigModelAdapter;
impl_pass_through_adapter!(
	name: BigModelAdapter,
	kind: AdapterKind::BigModel,
	key_env: Some("BIGMODEL_API_KEY"),
	endpoint: "https://open.bigmodel.cn/api/paas/v4/",
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

// -- Groq
pub struct GroqAdapter;
impl_pass_through_adapter!(
	name: GroqAdapter,
	kind: AdapterKind::Groq,
	key_env: Some("GROQ_API_KEY"),
	endpoint: "https://api.groq.com/openai/v1/",
	delegate: OpenAIAdapter,
	unsupported: [embeddings],
);

// -- Kimi (Moonshot.ai)
// API Doc: https://platform.kimi.ai/docs/api/overview
// TODO: need to support thinking with extra_body
pub struct KimiAdapter;
impl_pass_through_adapter!(
	name: KimiAdapter,
	kind: AdapterKind::Kimi,
	key_env: Some("KIMI_API_KEY"),
	endpoint: "https://api.moonshot.ai/v1/",
	delegate: OpenAIAdapter,
	unsupported: [embeddings],
);

// -- Moonshot (Moonshot.cn)
pub struct MoonshotAdapter;
impl_pass_through_adapter!(
	name: MoonshotAdapter,
	kind: AdapterKind::Moonshot,
	key_env: Some("MOONSHOT_API_KEY"),
	endpoint: "https://api.moonshot.cn/v1/",
	delegate: OpenAIAdapter,
);

// -- MiniMax (AnthropicAdapter)
pub struct MinimaxAdapter;
impl_pass_through_adapter!(
	name: MinimaxAdapter,
	kind: AdapterKind::MiniMax,
	key_env: Some("MINIMAX_API_KEY"),
	endpoint: "https://api.minimax.io/anthropic/v1/",
	delegate: AnthropicAdapter,
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

// -- OpenRouter
pub struct OpenRouterAdapter;
impl_pass_through_adapter!(
	name: OpenRouterAdapter,
	kind: AdapterKind::OpenRouter,
	key_env: Some("OPEN_ROUTER_API_KEY"),
	endpoint: "https://openrouter.ai/api/v1/",
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

// -- XaiAdapter
pub struct XaiAdapter;
impl_pass_through_adapter!(
	name: XaiAdapter,
	kind: AdapterKind::Xai,
	key_env: Some("XAI_API_KEY"),
	endpoint: "https://api.x.ai/v1/",
	delegate: OpenAIAdapter,
);

// endregion: --- Pass-through adapters (with macros)
