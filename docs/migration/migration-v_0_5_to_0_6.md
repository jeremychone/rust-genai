# Migration Guide: v0.5.x to v0.6.0-beta.x

This guide highlights the `genai` API changes needed when moving from the 0.5.x line to the 0.6.0 beta line.

## API Changes

- **New Adapters Added**:
  - AWS Bedrock (`BedrockApi` and `BedrockSigv4` adapters).
  - OpenRouter (`OpenRouter` adapter).
  - Google Vertex (`Vertex` adapter with Gemini and Anthropic support).
  - GitHub Copilot (`GithubCopilot` adapter for GitHub Models API).
  - OpenCode Go (`OpenCodeGo` adapter).
  - Baidu (`Baidu` adapter).
  - Aliyun (`Aliyun` adapter).
  - Moonshot AI (`Moonshot` adapter).
  - AIHubMix (`Aihubmix` adapter).

- **Groq Namespace Requirement**: Groq models must now be addressed via namespaced models (e.g. `groq::llama-3.1-8b-instant`).

- **All Model Names API Signature**: `Client::all_model_names(adapter_kind, impl Into<ProviderConfig>)` now takes a second Provider Config parameter, which can be `None` for default behavior. (see [Client::all_model_names](#all_model_names))

- **Custom Part Model Identity**: `ContentPart::CustomPart.model_iden` is now an `Option<ModelIden>`.

## Code Examples


### all_model_names

```rust
// -- v0.5.x

let models = client.all_model_names(adapter_kind).await?;

// -- v0.0.6

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
