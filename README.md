# genai, Native-Protocol Multi-AI Provider Library for Rust

Provides a single, ergonomic Rust API for native-protocol multi-AI provider access, including Anthropic, OpenAI, Gemini, xAI, Ollama, Groq, and more.

Currently natively supports over **25 providers**: `OpenAI`, `OpenAI Responses`, `Anthropic`, `Gemini`, `xAI`, `Ollama`, `Ollama Cloud`, `OpenCode Go`, `Groq`, `DeepSeek`, `Cohere`, `Together`, `Fireworks`, `Nebius`, `Mimo`, `Zai` (Zhipu AI), `BigModel`, `Aliyun`, `Baidu`, `Moonshot`, `Google Vertex`, `GitHub Copilot` (GitHub Models API), `AIHubMix`, `AWS Bedrock`, `OpenRouter`.

Also supports a custom Endpoint and Auth with `ServiceTargetResolver` (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs)).

**NOTE:** Use `genai = "0.6.0-beta.20"` or later for improved robustness, even compared to `0.5.x`, along with many more providers, fixes, performance improvements, and API enhancements. `v0.6.0` is coming soon.


<div align="center">

<a href="https://crates.io/crates/genai"><img src="https://img.shields.io/crates/v/genai.svg" /></a>
<a href="https://github.com/jeremychone/rust-genai"><img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699"></a>
<a href="https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_genai_Intro-Video?style=flat&logo=youtube&color=%23ff0000"></a>

</div>


[Docs for LLMs](docs/for-llm/api-reference-for-llm.md) | [CHANGELOG](CHANGELOG.md) | [BIG THANKS](BIG-THANKS.md)

## Model to Adapter Resolution

By default, the library resolves the `AdapterKind` (AI Provider) based on the model name prefix:

- **OpenAI**: `gpt-*` (most), `o1-*`, `o3-*`, `o4-*`, `chatgpt-*`, `codex-*`
- **OpenAI Responses**: `gpt-5-*`, `gpt-*` (containing `codex` or `pro`)
- **Anthropic**: `claude-*`
- **Gemini**: `gemini-*`
- **xAI**: `grok-*`
- **DeepSeek**: `deepseek-*`
- **Moonshot**: `moonshot-*`
- **Zai**: `glm-*`
- **Cohere**: `command-*`, `embed-*`
- **Mimo**: `mimo-*`
- **OpenCode Go**: Namespace `opencode_go::` only
- **Fireworks**: Models containing `fireworks`
- **Ollama**: Fallback for any other names, defaulting to local Ollama.

### Namespacing (Forcing an Adapter)

You can force a specific adapter by using the `adapter_kind::model_name` syntax. This is the recommended way for many providers and for disambiguating OpenAI-compatible services.

- `groq::llama-3.1-8b-instant` (Forces **Groq** adapter)
- `together::meta-llama/Llama-3-8b-chat-hf` (Forces **Together** adapter)
- `ollama_cloud::gemma3:4b` (Forces **OllamaCloud** adapter)
- `github_copilot::openai/gpt-4.1-mini` (Forces **GithubCopilot** adapter)
- `nebius::Qwen/Qwen3-235B-A22B` (Forces **Nebius** adapter)
- `aliyun::qwen-plus` (Forces **Aliyun** adapter)
- `vertex::gemini-2.5-flash` (Forces **Google Vertex** adapter)
- `moonshot::moonshot-v1-8k` (Forces **Moonshot** adapter)
- `baidu::ernie-4.0` (Forces **Baidu** adapter)
- `coding::glm-4.6` (Special namespace for **Zai** coding subscription)
- `opencode_go::minimax-m2.5` (Forces **OpenCodeGo** adapter)
- `bedrock_api::anthropic.claude-v2` (Forces **AWS Bedrock** adapter)
- `open_router::google/gemini-2.0-flash-001` (Forces **OpenRouter** adapter)

For a complete list of `AdapterKind`, see the [AdapterKind enum](src/adapter/adapter_kind.rs).

## v0.6.x - (2026-05-16...)

- **What's new**:
    - **New Adapters**: AWS Bedrock (API and SigV4), OpenRouter, Baidu, Moonshot, and many others.
    - **Expanded Provider Support**: Comprehensive coverage of major AI ecosystems.
    - **Updated API**: Refined `ReasoningContent` and `StopReason` handling (v0.6.0-beta.20).
    - Numerous fixes, optimizations, and API enhancements.

## v0.5.x - (2026-01-09 onwards)

- **What's new**:
    - **New Adapters**: BigModel.cn and the MIMO model adapter (thanks to [Akagi201](https://github.com/Akagi201)).
    - **zai: updated namespace strategy**, using `zai::` for default and `zai-coding::` for subscriptions (same adapter).
    - **Gemini Thinking & Thought**: Full support for Gemini Thought signatures (thanks to [Himmelschmidt](https://github.com/Himmelschmidt)) and thinking levels.
    - **Reasoning Effort Control**: Support for `ReasoningEffort` for Anthropic (Claude 3.7/4.5) and Gemini (Thinking levels), including `ReasoningEffort::None`.
    - **Content & Binary Improvements**: Enhanced binary/PDF API and size tracking.
    - **Internal Stream Refactor**: Switched to a unified `EventSourceStream` and `WebStream` for better reliability and performance across all providers.
    - **Dependency Upgrade**: Now using `reqwest 0.13`.
- **Core Features**:
    - Normalized and ergonomic Chat API across all major providers.
    - Native protocol support for Gemini and Anthropic protocols (Reasoning/Thinking controls).
    - PDF, image, and embedding support.
    - Custom authentication, endpoint, and header overrides.

See [CHANGELOG](CHANGELOG.md)

## Usage examples

- Check out [AIPACK](https://aipack.ai), which wraps this **genai** library into an agentic runtime to run, build, and share AI Agent Packs. See [`pro@coder`](https://www.youtube.com/watch?v=zL1BzPVM8-Y&list=PL7r-PXl6ZPcB2zN0XHsYIDaD5yW8I40AE) for a simple example of how I use AI PACK/genai for production coding.

> Note: Feel free to send me a short description and a link to your application or library that uses genai.

## Key Features

- Multi-AI Provider/Model access optimized per provider: native protocols when available, OpenAI-compatible APIs when appropriate or required, and one common Rust API for OpenAI, OpenAI Responses, Anthropic, Gemini, Ollama, Ollama Cloud, OpenCode Go, Groq, xAI, DeepSeek, Cohere, Together, Fireworks, Nebius, Mimo, Zai, BigModel, Aliyun, Google Vertex, and GitHub Copilot (direct chat and streaming) (see [examples/c00-readme.rs](examples/c00-readme.rs))
- DeepSeekR1 support, with `reasoning_content` (and stream support), plus DeepSeek Groq and Ollama support (and `reasoning_content` normalization)
- Image Analysis (for OpenAI, Gemini flash-2, Anthropic) (see [examples/c07-image.rs](examples/c07-image.rs))
- Custom Auth/API Key (see [examples/c02-auth.rs](examples/c02-auth.rs))
- Model aliases (see [examples/c05-model-names.rs](examples/c05-model-names.rs))
- Custom endpoint, auth, and model identifier (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs))

[Examples](#examples) | [Thanks](#thanks) | [Library Focus](#library-focus) | [Changelog](CHANGELOG.md) | Provider Mapping: [ChatOptions](#chatoptions) | [Usage](#usage)

## Examples

[examples/c00-readme.rs](examples/c00-readme.rs)

```rust
//! Base examples demonstrating the core capabilities of genai

use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

const MODEL_OPENAI: &str = "gpt-4o-mini"; // o1-mini, gpt-4o-mini
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
// or namespaced with simple name "fireworks::qwen3-30b-a3b", or "fireworks::accounts/fireworks/models/qwen3-30b-a3b"
const MODEL_FIREWORKS: &str = "accounts/fireworks/models/qwen3-30b-a3b";
const MODEL_TOGETHER: &str = "together::openai/gpt-oss-20b";
const MODEL_GEMINI: &str = "gemini-2.0-flash";
const MODEL_GROQ: &str = "groq::llama-3.1-8b-instant";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`
const MODEL_OLLAMA_CLOUD: &str = "ollama_cloud::gemma3:4b";
const MODEL_XAI: &str = "grok-3-mini";
const MODEL_DEEPSEEK: &str = "deepseek-chat";
const MODEL_ZAI: &str = "glm-4-plus";
const MODEL_COHERE: &str = "command-r7b-12-2024";
const MODEL_MOONSHOT: &str = "moonshot::moonshot-v1-8k";
const MODEL_BAIDU: &str = "baidu::ernie-4.0";
const MODEL_BIGMODEL: &str = "bigmodel::glm-4-plus";
const MODEL_ALIYUN: &str = "aliyun::qwen-plus";
// or any publisher: "github_copilot::anthropic/claude-sonnet-4-6", "github_copilot::google/gemini-2.5-pro", "github_copilot::xai/grok-3-mini"
const MODEL_GITHUB_COPILOT: &str = "github_copilot::openai/gpt-4.1-mini";
const MODEL_OPEN_ROUTER: &str = "open_router::google/gemini-2.0-flash-001";

// NOTE: These are the default environment keys for each AI Adapter Type.
//       They can be customized; see `examples/c02-auth.rs`.
const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	// -- De/activate models/providers
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
	(MODEL_GEMINI, "GEMINI_API_KEY"),
	(MODEL_FIREWORKS, "FIREWORKS_API_KEY"),
	(MODEL_TOGETHER, "TOGETHER_API_KEY"),
	(MODEL_GROQ, "GROQ_API_KEY"),
	(MODEL_XAI, "XAI_API_KEY"),
	(MODEL_DEEPSEEK, "DEEPSEEK_API_KEY"),
	(MODEL_OLLAMA, ""),
	(MODEL_OLLAMA_CLOUD, "OLLAMA_API_KEY"),
	(MODEL_ZAI, "ZAI_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_MOONSHOT, "MOONSHOT_API_KEY"),
	(MODEL_BAIDU, "BAIDU_API_KEY"),
	(MODEL_BIGMODEL, "BIGMODEL_API_KEY"),
	(MODEL_ALIYUN, "ALIYUN_API_KEY"),
	(MODEL_GITHUB_COPILOT, "GITHUB_TOKEN"),
	(MODEL_OPEN_ROUTER, "OPEN_ROUTER_API_KEY"),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI (or OpenAI Responses for gpt-5/codex/pro)
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - starts_with "gemini"   -> Gemini
//  - model in Groq models   -> Groq
//  - starts_with "glm"      -> ZAI
//  - starts_with "ollama_cloud::" -> OllamaCloud
//  - For anything else      -> Ollama
//
// This can be customized; see `examples/c03-mapper.rs`

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	let print_options = PrintChatStreamOptions::from_print_events(false);

	for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
		// Skip if the environment name is not set
		if !env_name.is_empty() && std::env::var(env_name).is_err() {
			println!("===== Skipping model: {model} (env var not set: {env_name})");
			continue;
		}

		let adapter_kind = client.resolve_service_target(model).await?.model.adapter_kind;

		println!("\n===== MODEL: {model} ({adapter_kind}) =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer:");
		let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
		println!("{}", chat_res.first_text().unwrap_or("NO ANSWER"));

		println!("\n--- Answer: (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;
		print_chat_stream(chat_res, Some(&print_options)).await?;

		println!();
	}

	Ok(())
}
```

### More Examples

- [examples/c00-readme.rs](examples/c00-readme.rs) - Quick overview code with multiple providers and streaming.
- [examples/c01-conv.rs](examples/c01-conv.rs) - Shows how to build a conversation flow.
- [examples/c02-auth.rs](examples/c02-auth.rs) - Demonstrates how to provide a custom `AuthResolver` to provide auth data (i.e., for api_key) per adapter kind.
- [examples/c03-mapper.rs](examples/c03-mapper.rs) - Demonstrates how to provide a custom `AdapterKindResolver` to customize the "model name" to "adapter kind" mapping.
- [examples/c04-chat-options.rs](examples/c04-chat-options.rs) - Demonstrates how to set chat generation options such as `temperature` and `max_tokens` at the client level (for all requests) and at the per-request level.
- [examples/c05-model-names.rs](examples/c05-model-names.rs) - Shows how to get model names per AdapterKind.
- [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs) - For custom auth, endpoint, and model.
- [examples/c07-image.rs](examples/c07-image.rs) - Image analysis support

<br />
<a href="https://www.youtube.com/playlist?list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_JC_AI_Playlist-Video?style=flat&logo=youtube&color=%23ff0000"></a>
<br />

- [genai ModelMapper code demo (v0.1.7)](https://www.youtube.com/watch?v=5Enfcwrl7pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)

- [genai introduction (v0.1.0)](https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)

- **genai live coding, code design, & best practices**
    - [Adding **Gemini** Structured Output (vid-0060)](https://www.youtube.com/watch?v=GdFsqLJ1_pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [Adding **OpenAI** Structured Output (vid-0059)](https://www.youtube.com/watch?v=FpoNbQMhAH8&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [Splitting the json value extension trait to its own public crate value-ext](https://www.youtube.com/watch?v=OS5KOz9y7Cg&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj) [value-ext](https://crates.io/crates/value-ext)
    - [(part 1/3) Module, Error, constructors/builders](https://www.youtube.com/watch?v=XCrZleaIUO4&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [(part 2/3) Extension Traits, Project Files, Versioning](https://www.youtube.com/watch?v=LRfDAZfo00o&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [(part 3/3) When to Async? Project Files, Versioning strategy](https://www.youtube.com/watch?v=93SS3VGsKx4&list=PL7r-PXl6ZPcCIOFaL7nVHXZvBmHNhrh_Q)

## Library Focus:

- Focuses on standardizing chat completion APIs across major AI providers while preserving provider-specific strengths.

- Native implementation without per-service SDK dependencies.
    - Reason: genai uses each provider's native protocol when available, so features such as reasoning controls, thinking budgets, streaming metadata, and multimodal inputs can be represented more completely. When a provider primarily exposes an OpenAI-compatible API, genai uses that compatibility layer instead. Managing these protocol differences at the adapter layer is simpler and more cumulative than dealing with multiple SDKs.

- Prioritizes ergonomics and commonality, while depth is secondary. (If you require a complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); both are excellent and easy to use.)

- This library focuses on text chat APIs, with vision and function calling support being expanded.

## ChatOptions

- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo, Together, Fireworks, Nebius, Zai, AIHubMix

| Property      | OpenAI Compatibles (*1) | Anthropic                   | Gemini `generationConfig.` | Cohere        |
|---------------|-------------------------|-----------------------------|----------------------------|---------------|
| `temperature` | `temperature`           | `temperature`               | `temperature`              | `temperature` |
| `max_tokens`  | `max_tokens`            | `max_tokens` (default 1024) | `maxOutputTokens`          | `max_tokens`  |
| `top_p`       | `top_p`                 | `top_p`                     | `topP`                     | `p`           |

## Usage

| Property                    | OpenAI Compatibles (1)      | Anthropic `usage.`      | Gemini `usageMetadata.`    | Cohere `meta.tokens.` |
|-----------------------------|-----------------------------|-------------------------|----------------------------|-----------------------|
| `prompt_tokens`             | `prompt_tokens`             | `input_tokens` (added)  | `promptTokenCount` (2)     | `input_tokens`        |
| `completion_tokens`         | `completion_tokens`         | `output_tokens` (added) | `candidatesTokenCount` (2) | `output_tokens`       |
| `total_tokens`              | `total_tokens`              | (computed)              | `totalTokenCount` (2)      | (computed)            |
| `prompt_tokens_details`     | `prompt_tokens_details`     | `cached/cache_creation` | N/A for now                | N/A for now           |
| `completion_tokens_details` | `completion_tokens_details` | N/A for now             | N/A for now                | N/A for now           |

- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo, AIHubMix
	- For **Groq**, the property `x_groq.usage.`
	- At this point, **Ollama** does not emit input/output tokens when streaming due to a limitation in the Ollama OpenAI compatibility layer. (see [ollama #4448 - Streaming Chat Completion via OpenAI API should support stream option to include Usage](https://github.com/ollama/ollama/issues/4448))
	- `prompt_tokens_details` and `completion_tokens_details` will have the value sent by the compatible provider (or None)

- **(2)**: **Gemini** tokens
	- Right now, with the [Gemini Stream API](https://ai.google.dev/api/rest/v1beta/models/streamGenerateContent), it's not clear whether usage for each event is cumulative or must be summed. It appears to be cumulative, meaning the last message shows the total number of input, output, and total tokens, so that is the current assumption. See [possible tweet answer](https://twitter.com/jeremychone/status/1813734565967802859) for more info.

## Notes on Possible Direction

- Will add more data to ChatResponse and ChatStream, especially usage metadata.
- Add vision/image support to chat messages and responses.
- Add function calling support to chat messages and responses.
- Add `embed` and `embed_batch`.
- Add the AWS Bedrock variants (e.g., Mistral and Anthropic). Most of the work will be on the "interesting" token signature scheme. To avoid bringing in large SDKs, this might be a lower-priority feature.
- Add the Google Vertex AI variants.
- May add the Azure OpenAI variant (not sure yet).

## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
- Sponsored by [BriteSnow](https://britesnow.com) (Jeremy Chone's consulting company)
