# Migration Guide: v0.5.x to v0.6.0-beta.x

This guide highlights the `genai` API changes needed when moving from the 0.5.x line to the 0.6.0 beta line.

## API Changes

- **New Adapters Added**:
  - AWS Bedrock (`bedrock_api` and `bedrock_sigv4` adapters).
  - OpenRouter (`open_router` adapter).
  - Google Vertex (`vertex` adapter with Gemini and Anthropic support).
  - GitHub Copilot (`github_copilot` adapter for GitHub Models API).
  - OpenCode Go (`opencode_go` adapter).
  - Baidu (`baidu` adapter).
  - Aliyun (`aliyun` adapter).
  - Moonshot AI (`moonshot` adapter).
  - AIHubMix (`aihubmix` adapter).
  - Ollama Cloud (`ollama_cloud` adapter).

- **Groq Namespace Requirement**: Groq models must now be addressed via namespaced models (e.g. `groq::llama-3.1-8b-instant`).

- **All Model Names API Signature**: `Client::all_model_names(adapter_kind, impl Into<ProviderConfig>)` now takes a second Provider Config parameter, which can be `None` for default behavior. (see [Client::all_model_names](#all_model_names))

- **ProviderConfig for Model Listing**: The second `all_model_names` argument can override endpoint and auth for adapter-wide model listing, for example remote Ollama or OpenAI-compatible provider endpoints.

- **Bound Adapter Clients**: `ClientBuilder::with_adapter_kind(adapter_kind)` and `ClientConfig::with_adapter_kind(adapter_kind)` can bind a client to a single adapter. Bare model names route through that adapter. Explicit mismatched namespaces or mismatched `ModelIden` values return `AdapterKindMismatch`.

- **Chat Extra Body**: `ChatOptions::with_extra_body(serde_json::Value)` adds a low-level provider-specific body extension point. OpenAI-compatible chat payloads merge this value into Chat Completions and Responses request bodies.

- **Tool Choice**: `ChatOptions::with_tool_choice(...)` adds provider-neutral tool selection support, mapped to Gemini, OpenAI Chat Completions, OpenAI Responses, and Anthropic payloads.

- **Gemini Schema and Tool Compatibility**: Gemini schema conversion now normalizes provider-incompatible JSON Schema features, including converting `const` to single-value `enum`, preserving and sanitizing `additionalProperties`, and stripping JSON Schema-only keywords rejected by Vertex.

- **Custom Part Model Identity**: `ContentPart::CustomPart.model_iden` is now an `Option<ModelIden>`.

## Code Examples


### all_model_names

```rust
// -- v0.5.x

let models = client.all_model_names(adapter_kind).await?;

// -- v0.6.0

// Same as v0.5.x
let models = client.all_model_names(adapter_kind, None).await?;

// ProviderConfig { 	pub endpoint: Option<Endpoint>, pub auth: Option<AuthData>}

// Remote Ollama endpoint override
let models = client
	.all_model_names(
		AdapterKind::Ollama,
		Endpoint::from_static("http://remote-host:11434/"),
	)
	.await?;

// Custom openai compatible provider
let models = client
	.all_model_names(
		AdapterKind::OpenAI,
		(
			Endpoint::from_static("http://fair-llm.ai/"),
			AuthData::from_env("FAIR_LLM_API_KEY"),
		),
	)
	.await?;
```


### Bound adapter clients

A client can now be bound to a single adapter. This is useful for proxies, OpenAI-compatible gateways, Azure-style deployment names, or any setup where the model name should not be inferred from hardcoded prefixes.

```rust
use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint};

let client = genai::Client::builder()
	.with_adapter_kind(AdapterKind::OpenAIResp)
	.with_auth_resolver_fn(|_| Ok(Some(AuthData::from_env("PROXY_API_KEY"))))
	.with_service_target_resolver_fn(|mut target| {
		target.endpoint = Endpoint::from_static("https://proxy.example/v1/");
		Ok(target)
	})
	.build();

let response = client.exec_chat("custom-deployment-name", chat_req, None).await?;
```

Behavior notes:

- Bare model names route through the bound adapter.
- Namespaced model names that target another adapter return `AdapterKindMismatch`.
- `ModelSpec::Iden` values that target another adapter return `AdapterKindMismatch`.
- Fully resolved `ModelSpec::Target` values are unchanged.

### Chat extra_body

`ChatOptions::with_extra_body(...)` can pass provider-specific request fields that are not modeled as typed `genai` options yet. This is currently merged into OpenAI-compatible Chat Completions and Responses payloads.

```rust
use genai::chat::ChatOptions;
use serde_json::json;

let options = ChatOptions::default().with_extra_body(json!({
	"enable_thinking": false
}));

let response = client.exec_chat("aliyun::qwen-plus", chat_req, Some(&options)).await?;
```

Use typed `genai` options when available. Use `extra_body` as an escape hatch for provider-specific or newly introduced fields.

### Tool choice and Gemini schema normalization

`ChatOptions::with_tool_choice(...)` adds a provider-neutral tool selection hint. It is mapped for Gemini, OpenAI Chat Completions, OpenAI Responses, and Anthropic.

Gemini and Vertex Gemini schema conversion now accepts more common JSON Schema output, including schema produced by tools such as `schemars`, by normalizing provider-incompatible fields before sending the request.
