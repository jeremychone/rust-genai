use crate::adapter::adapter_macros::define_adapter_kinds;
use crate::adapter::adapters::baidu::BAIDU_CODING_ANTHROPIC_NAMESPACE;
use crate::adapter::adapters::baidu::BAIDU_CODING_OPENAI_NAMESPACE;
use crate::adapter::zai;
use crate::{ModelName, Result};

// Read the macro definition before editing this part.
define_adapter_kinds![
	/// For OpenAI Chat Completions and also can be used for OpenAI compatible APIs
	/// NOTE: This adapter share some behavior that other adapters can use while still providing some variant
	OpenAI => openai,
	/// For OpenAI Responses API
	OpenAIResp => openai_resp,
	/// Gemini adapter supports gemini native protocol. e.g., support thinking budget.
	Gemini => gemini,
	/// Anthopric native protocol as well
	Anthropic => anthropic,
	/// For fireworks.ai, mostly OpenAI.
	Fireworks => fireworks,
	/// Together AI (Mostly uses OpenAI-compatible protocol)
	Together => together,
	/// Reuse some of the OpenAI adapter behavior, customize some (e.g., normalize thinking budget)
	Groq => groq,
	/// For AIHubMix (Mostly use OpenAI)
	Aihubmix => aihubmix,
	/// For Mimo (Mostly use OpenAI)
	Mimo => mimo,
	/// For Moonshot AI (Mostly use OpenAI)
	Moonshot => moonshot,
	/// For Nebius (Mostly use OpenAI)
	Nebius => nebius,
	/// For xAI (Mostly use OpenAI)
	Xai => xai,
	/// For DeepSeek (Mostly use OpenAI)
	DeepSeek => deepseek,
	/// For ZAI (Mostly use OpenAI)
	Zai => zai,
	/// For big model (only accessible via namespace bigmodel::)
	BigModel => bigmodel,
	/// For aliyun (Mostly use OpenAI)
  Aliyun => aliyun,
	/// For baidu (Mostly use OpenAI)
	Baidu => baidu,
	/// Cohere today use it's own native protocol but might move to OpenAI Adapter
	Cohere => cohere,
	/// OpenAI shared behavior + some custom. (currently, localhost only, can be customize with ServerTargetResolver).
	Ollama => ollama,
	/// For Ollama Cloud (ollama.com) - uses native Ollama protocol with Bearer auth
	OllamaCloud => ollama_cloud,
	/// Google Vertex AI (Model Garden). Supports Gemini and Claude models via publishers/google and publishers/anthropic.
	/// Uses namespace routing: `vertex::gemini-2.5-flash`, `vertex::claude-sonnet-4-6`
	Vertex => vertex,
	/// GitHub Models inference API (multi-publisher gateway for OpenAI, Anthropic, and Google models).
	/// Uses namespace routing: `github_copilot::openai/gpt-4.1-mini`, `github_copilot::anthropic/claude-sonnet-4-6`, `github_copilot::google/gemini-2.5-pro`
	GithubCopilot => github_copilot,
	/// OpenCode Go proxy (OpenAI-compatible adapter for the OpenCode ecosystem).
	/// Namespace: `opencode_go::model-name` — route any model via the OpenCode Go gateway.
	OpenCodeGo => opencode_go,
	/// AWS Bedrock Converse API, authenticated with a simple Bearer token from
	/// `BEDROCK_API_KEY`. Always available — no extra Cargo feature or dependencies required.
	/// Namespace: `bedrock_api::anthropic.claude-sonnet-4-5-20250929-v1:0`.
	BedrockApi => bedrock as bedrock_api,
	/// AWS Bedrock Converse API, authenticated via SigV4 + the AWS credential chain
	/// (env, profile, SSO, IMDS, AssumeRole).
	/// Namespace: `bedrock_sigv4::anthropic.claude-sonnet-4-5-20250929-v1:0`.
	/// Requires the `bedrock-sigv4` Cargo feature.
  @cfg(feature = "bedrock-sigv4")
	BedrockSigv4 => bedrock as bedrock_sigv4,
	/// OpenRouter — OpenAI-compatible gateway for many providers (OpenAI, Anthropic, Google, etc.).
	/// Namespace: `open_router::openai/gpt-4.1`, `open_router::anthropic/claude-sonnet-4-5`.
	/// Uses `OPEN_ROUTER_API_KEY`.
	OpenRouter => open_router
];

/// From Model implementations
impl AdapterKind {
	/// This is a default static mapping from model names to AdapterKind.
	///
	/// When more control is needed, the `ServiceTypeResolver` can be used
	/// to map a model name to any adapter and endpoint.
	///
	///  - OpenAI     - starts_with "gpt", "o3", "o1", "chatgpt"
	///  - Gemini     - starts_with "gemini"
	///  - Anthropic  - starts_with "claude"
	///  - Fireworks  - contains "fireworks" (might add leading or trailing '/' later)
	///  - Groq       - model in Groq models
	///  - DeepSeek   - model in DeepSeek models (deepseek.com)
	///  - Zhipu      - starts_with "glm"
	///  - Cohere     - starts_with "command"
	///  - Ollama     - For anything else
	///
	/// Other Some adapters have to have model name namespaced to be used,
	/// - e.g., for together.ai `together::meta-llama/Llama-3-8b-chat-hf`
	/// - e.g., for nebius with `nebius::Qwen/Qwen3-235B-A22B`
	/// - e.g., for ZAI coding plan with `zai_coding::glm-4.6`
	/// - e.g., for vertex with `vertex::gemini-2.5-flash` or `vertex::claude-sonnet-4-6`
	///
	/// And all adapters can be force namspaced as well.
	///
	/// Note: At this point, this will never fail as the fallback is the Ollama adapter.
	///       This might change in the future, hence the Result return type.
	///
	/// IMPORTANT: Since v0.6.0, Groq and Deepseek models needs to be namespaced e.g., `groq::_model_name_`
	//             (because now, list_names are dynamic, so, automatic mapping can only be done base on clear model "prefixes")
	pub fn from_model(model: &str) -> Result<Self> {
		// -- First check if namespaced
		if let Some(adapter) = Self::from_model_namespace(model) {
			return Ok(adapter);
		};

		// -- Otherwise, Resolve from modelname
		if model.starts_with("o3")
			|| model.starts_with("o4")
			|| model.starts_with("o1")
			|| model.starts_with("chatgpt")
			|| model.starts_with("codex")
			|| (model.starts_with("gpt") && !model.starts_with("gpt-oss"))
			|| model.starts_with("text-embedding")
		// migh be a little generic on this one
		{
			if model.starts_with("gpt-5")
				|| (model.starts_with("gpt") && (model.contains("codex") || model.contains("pro")))
			{
				Ok(Self::OpenAIResp)
			} else {
				Ok(Self::OpenAI)
			}
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if model.contains("fireworks") {
			Ok(Self::Fireworks)
		} else if model.starts_with("mimo-") {
			Ok(Self::Mimo)
		} else if model.starts_with("command") || model.starts_with("embed-") {
			Ok(Self::Cohere)
		} else if model.starts_with("grok") {
			Ok(Self::Xai)
		} else if model.starts_with("glm") {
			Ok(Self::Zai)
		} else if model.starts_with("deepseek-") {
			Ok(Self::DeepSeek)
		} else if model.starts_with("moonshot-") {
			Ok(Self::Moonshot)
		}
		// For now, fallback to Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

// region:    --- Support

/// Inner api to return
impl AdapterKind {
	pub(crate) fn from_model_namespace(model: &str) -> Option<Self> {
		let (namespace, _) = ModelName::split_as_namespace_and_name(model);
		let namespace = namespace?;

		// -- First, check if simple adapter lower string match
		if let Some(adapter) = Self::from_lower_str(namespace) {
			Some(adapter)
		}
		// -- Second, custom namespaces
		else if namespace == zai::ZAI_CODING_NAMESPACE {
			Some(Self::Zai)
		} else if namespace == BAIDU_CODING_OPENAI_NAMESPACE || namespace == BAIDU_CODING_ANTHROPIC_NAMESPACE {
			Some(Self::Baidu)
		}
		//
		// -- Otherwise, no adapter from namespace, because no matching namespace
		else {
			None
		}
	}
}

// endregion: --- Support
