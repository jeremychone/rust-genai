# genai

**A Native-Protocol Multi-AI Provider Library for Rust**

```toml
genai = "0.6"
```

<div align="center">

<a href="https://crates.io/crates/genai"><img src="https://img.shields.io/crates/v/genai.svg" /></a>
<a href="https://github.com/jeremychone/rust-genai"><img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699"></a>

</div>

`genai` provides a single, ergonomic Rust API for **native-protocol** multi-AI provider access, including Anthropic, OpenAI, Gemini, xAI, Ollama, Groq, and more.

Over 200+ LLM models, 25+ LLM providers out of the box, including **Ollama** for local execution.

Out-of-the-box providers: `openai`, `openai_resp`, `anthropic`, `gemini`, `ollama`, `ollama_cloud`, `vertex`, `bedrock_api`, `bedrock_sigv4`, `github_copilot`, `opencode_go`, `groq`, `together`, `fireworks`,  `cohere`, `nebius`, `mimo`, `deepseek`, `minimax`, `zai`, `zai_coding`, `bigmodel`, `aliyun`, `baidu`, `moonshot`, `aihubmix`, `open_router`, `xai`

Also supports custom endpoints and auth with `ServiceTargetResolver` (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs)) to support any other providers.


```rust

// Can talk to any models / providers
let client = Client::default();

let question = "Why is the sky red?";

let chat_req = ChatRequest::new(vec![
	ChatMessage::system("Answer in one sentence"),
	ChatMessage::user(question),
]);

// Model names can even have a reasoning effort suffix, such as "-high", which will be set, and then removed from name when sent to the provider.
let chat_res = client.exec_chat("gpt-5.4-mini-high", chat_req, None).await?;
	
println!("{}", chat_res.first_text().unwrap_or("NO ANSWER"));
	
```

[Docs for LLMs](docs/for-llm/api-reference-for-llm.md) | [CHANGELOG](CHANGELOG.md) | [BIG THANKS](BIG-THANKS.md)

## v0.7.0-beta.x on going 

- Same quality as v0.6.x, with some new/updated features
- **NEW: Custom Adapter** – Use `genai_n::model_name` to target a custom OpenAI-compatible endpoint.
  - Configure the endpoint with the environment variable `GENAI_{n}_ENDPOINT` and the API key with `GENAI_{n}_API_KEY`.
  - The adapter uses the OpenAI protocol internally (for now, might be configurable in the future)
  - Example: `genai_1::my-model-7b` with `GENAI_1_ENDPOINT=https://my-host/v1/` and `GENAI_1_API_KEY=sk-...`.

(see [genai releases](https://crates.io/crates/genai/versions))

## v0.6.x Released 🎉 

New provider since v0.6.0: `minimax`

_v0.6.0 release date: 2026-05-23_

Here’s what’s new:

- **New Adapters**:
    - AWS Bedrock (`bedrock_api` and `bedrock_sigv4` adapters)
    - `open_router`
    - `vertex` (with Gemini and Anthropic support)
    - `github_copilot` (GitHub Models API)
    - `opencode_go`
    - `baidu`
    - `aliyun`
    - `moonshot`
    - `aihubmix`
    - `ollama_cloud` (Ollama Cloud)
- **Reasoning effort additions**: Added `ReasoningEffort::Max` for Anthropic and `ReasoningEffort::XHigh` for OpenAI.
- **ProviderConfig for model listing**: `Client::all_model_names(adapter_kind, provider_config)` now accepts endpoint and auth overrides, including remote Ollama hosts and custom OpenAI-compatible model listing.
- **Ollama and Ollama Cloud**: Now use the native Ollama API protocol.
- **Gemini schema compatibility**: Gemini and Vertex Gemini structured output and tool schemas now normalize common JSON Schema shapes, including `const`, nullable schema patterns, `additionalProperties`, and JSON Schema-only keywords rejected by Vertex.
- **Bound adapter clients**: `ClientBuilder::with_adapter_kind(...)` and `ClientConfig::with_adapter_kind(...)` bind a client to a single provider adapter, which is useful for proxies, gateways, Azure-style deployment names, and OpenAI-compatible providers with nonstandard model names.
- **ModelSpec and ServiceTarget**: Model arguments can be represented as a model name, explicit `ModelIden`, or complete `ServiceTarget`, enabling custom endpoints, auth, and model identity without relying on model-name inference.
- **OpenAI Responses stateful sessions**: OpenAI Responses supports session continuity with `previous_response_id`, request `store`, and returned `response_id`.
- **Chat extra body**: `ChatOptions::with_extra_body(...)` provides a low-level request body extension point for provider-specific fields in OpenAI-compatible chat payloads.
- **Tool choice**: `ChatOptions::with_tool_choice(...)` adds provider-neutral tool selection hints for automatic, disabled, required, or specific tool calls.
- **Built-in tools and WebSearch**: Added typed built-in tool support, including `ToolName`, `ToolConfig`, `WebSearch`, and provider mappings for Anthropic, OpenAI, and Gemini.
- **Prompt cache controls**: Chat-level `CacheControl` support adds provider-specific prompt caching options, including OpenAI `prompt_cache_key` and cache retention. On Anthropic, `Tool::with_cache_control` marks individual tools, and request-level `ChatOptions::with_cache_control` auto-applies a breakpoint to the static (tools+system) prefix.
- **Updated API**: Refined `ReasoningContent` and `StopReason` handling (v0.6.0-beta.20), including `ContentPart::ReasoningContent` and provider stop reasons.
- **Perf Improvements**: HTTP requests use performance optimizations such as gzip, `TCP_NODELAY`, and HTTP/2 tuning.
- Numerous fixes, optimizations, and API enhancements.

See [v0.5.x to v0.6.x migration](docs/migration/migration_v_0_5_to_0_6.md)

See [CHANGELOG](CHANGELOG.md)

See [BIG-THANKS](BIG-THANKS.md) for contributors

## Key Features

- Multi-AI provider/model access optimized per provider: native protocols when available, OpenAI-compatible APIs when appropriate or required, and one common Rust API for OpenAI, OpenAI Responses, Anthropic, Gemini, Ollama, Ollama Cloud, OpenCode Go, Groq, xAI, DeepSeek, Cohere, Together, Fireworks, Nebius, Mimo, Zai, BigModel, Aliyun, Google Vertex, and GitHub Copilot (direct chat and streaming) (see [examples/c00-readme.rs](examples/c00-readme.rs))
- Image analysis (for OpenAI, Gemini Flash-2, Anthropic) (see [examples/c07-image.rs](examples/c07-image.rs))
- Custom auth/API key (see [examples/c02-auth.rs](examples/c02-auth.rs))
- Model aliases (see [examples/c05-model-names.rs](examples/c05-model-names.rs))
- Custom endpoint, auth, and model identifier (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs))
- And much more

[Examples](#examples) | [Thanks](BIG-THANKS.md) | [Library Focus](#library-focus) | [Changelog](CHANGELOG.md) | Provider Mapping: [ChatOptions](#chatoptions) | [Usage](#usage)


## Model to Adapter Resolution

By default, the library resolves the `AdapterKind` (AI provider) based on the model name prefix:

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

- `groq::openai/gpt-oss-20b` (Forces **Groq** adapter)
- `together::meta-llama/Llama-3-8b-chat-hf` (Forces **Together** adapter)
- `fireworks::glm-5p1` (for fireworks.ai)
- `ollama_cloud::gemma3:4b` (Forces **Ollama Cloud** adapter)
- `github_copilot::openai/gpt-5.4-mini` (Forces **GitHub Copilot** adapter)
- `nebius::Qwen/Qwen3-235B-A22B` (Forces **Nebius** adapter)
- `aliyun::qwen-plus` (Forces **Aliyun** adapter)
- `vertex::gemini-2.5-flash` (Forces **Google Vertex** adapter)
- `moonshot::moonshot-v1-8k` (Forces **Moonshot** adapter)
- `baidu::ernie-4.0` (Forces **Baidu** adapter)
- `zai_coding::glm-4.6` (Special namespace for **Zai** coding subscription)
- `zai_coding::glm-4.6` (Special namespace for **Zai** coding subscription)
- `opencode_go::minimax-m2.5` (Forces **OpenCode Go** adapter)
- `bedrock_api::anthropic.claude-v2` (Forces **AWS Bedrock** adapter)
- `open_router::google/gemini-2.0-flash-001` (Forces **OpenRouter** adapter)
- `genai_1::my-model-7b` (Forces **Custom** adapter with index 1)

For a complete list of `AdapterKind`, see the [AdapterKind enum](src/adapter/adapter_kind.rs).

## Examples

[examples/c00-readme.rs](examples/c00-readme.rs)

```rust
//! Base examples demonstrating the core capabilities of genai

use genai::chat::printer::{print_chat_stream, PrintChatStreamOptions};
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

const MODEL_OPENAI: &str = "gpt-5.4-mini";
const MODEL_ANTHROPIC: &str = "claude-haiku-4-5";
const MODEL_FIREWORKS: &str = "fireworks::gpt-oss-20b";
const MODEL_TOGETHER: &str = "together::openai/gpt-oss-20b";
const MODEL_GEMINI: &str = "gemini-3-flash-preview";
const MODEL_GROQ: &str = "groq::openai/gpt-oss-20b";
const MODEL_OLLAMA: &str = "gemma4:e2b"; // sh: `ollama pull gemma:2b`
const MODEL_OLLAMA_CLOUD: &str = "ollama_cloud::gemma3:4b";
const MODEL_XAI: &str = "grok-3-mini";
const MODEL_DEEPSEEK: &str = "deepseek-chat";
const MODEL_ZAI: &str = "glm-4-plus";
const MODEL_ALIYUN: &str = "aliyun::qwen-plus"; // required namespace
// or any publisher: "github_copilot::anthropic/claude-sonnet-4-6", "github_copilot::google/gemini-2.5-pro", "github_copilot::xai/grok-3-mini"
const MODEL_GITHUB_COPILOT: &str = "github_copilot::openai/gpt-4.1-mini";

// NOTE: These are the default environment keys for each AI adapter type.
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
	(MODEL_MOONSHOT, "MOONSHOT_API_KEY"),
	(MODEL_BAIDU, "BAIDU_API_KEY"),
	(MODEL_BIGMODEL, "BIGMODEL_API_KEY"),
	(MODEL_ALIYUN, "ALIYUN_API_KEY"),
	(MODEL_GITHUB_COPILOT, "GITHUB_TOKEN"),
	(MODEL_OPEN_ROUTER, "OPEN_ROUTER_API_KEY"),
];

// NOTE: Model to AdapterKind (AI provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI (or OpenAI Responses for gpt-5/codex/pro)
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - starts_with "gemini"   -> Gemini
//  - model in Groq models    -> Groq
//  - starts_with "glm"       -> ZAI
//  - starts_with "ollama_cloud::" -> OllamaCloud
//  - For anything else       -> Ollama
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
- [examples/c02-auth.rs](examples/c02-auth.rs) - Demonstrates how to provide a custom `AuthResolver` to supply auth data, such as `api_key`, per adapter kind.
- [examples/c03-mapper.rs](examples/c03-mapper.rs) - Demonstrates how to provide a custom `AdapterKindResolver` to customize the "model name" to "adapter kind" mapping.
- [examples/c04-chat-options.rs](examples/c04-chat-options.rs) - Demonstrates how to set chat generation options such as `temperature` and `max_tokens` at the client level, for all requests, and at the per-request level.
- [examples/c05-model-names.rs](examples/c05-model-names.rs) - Shows how to get model names per `AdapterKind`.
- [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs) - For custom auth, endpoint, and model.
- [examples/c07-image.rs](examples/c07-image.rs) - Image analysis support

<br />
<a href="https://www.youtube.com/playlist?list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_JC_AI_Playlist-Video?style=flat&logo=youtube&color=%23ff0000"></a>
<br />

- [genai ModelMapper code demo (v0.1.7)](https://www.youtube.com/watch?v=5Enfcwrl7pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)

- [genai introduction (v0.1.0)](https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)

- **genai live coding, code design, and best practices**
    - [Adding **Gemini** Structured Output (vid-0060)](https://www.youtube.com/watch?v=GdFsqLJ1_pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [Adding **OpenAI** Structured Output (vid-0059)](https://www.youtube.com/watch?v=FpoNbQMhAH8&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [Splitting the JSON value extension trait into its own public crate, value-ext](https://www.youtube.com/watch?v=OS5KOz9y7Cg&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj) [value-ext](https://crates.io/crates/value-ext)
    - [(part 1/3) Module, Error, constructors/builders](https://www.youtube.com/watch?v=XCrZleaIUO4&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [(part 2/3) Extension Traits, Project Files, Versioning](https://www.youtube.com/watch?v=LRfDAZfo00o&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj)
    - [(part 3/3) When to Async? Project Files, Versioning strategy](https://www.youtube.com/watch?v=93SS3VGsKx4&list=PL7r-PXl6ZPcCIOFaL7nVHXZvBmHNhrh_Q)

## Library Focus:

- Focuses on standardizing chat completion APIs across major AI providers while preserving provider-specific strengths.

- Native implementation without per-service SDK dependencies.
    - Reason: genai uses each provider's native protocol when available, so features such as reasoning controls, thinking budgets, streaming metadata, and multimodal inputs can be represented more completely. When a provider primarily exposes an OpenAI-compatible API, genai uses that compatibility layer instead. Managing these protocol differences at the adapter layer is simpler and more scalable than dealing with multiple SDKs.

- Prioritizes ergonomics and commonality, while depth is secondary. (If you require a complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); both are excellent and easy to use.)

- This library focuses on text chat, vision, and function calling APIs. (If you require a complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); both are excellent and easy to use.)

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
| `completion_tokens`         | `completion_tokens`       | `output_tokens` (added) | `candidatesTokenCount` (2) | `output_tokens`       |
| `total_tokens`              | `total_tokens`              | (computed)              | `totalTokenCount` (2)      | (computed)            |
| `prompt_tokens_details`     | `prompt_tokens_details`     | `cached/cache_creation` | N/A for now                | N/A for now           |
| `completion_tokens_details` | `completion_tokens_details` | N/A for now             | N/A for now                | N/A for now           |

- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo, AIHubMix
	- For **Groq**, the property is `x_groq.usage.`
	- At this point, **Ollama** does not emit input/output tokens when streaming due to a limitation in the Ollama OpenAI compatibility layer. (see [ollama #4448 - Streaming Chat Completion via OpenAI API should support stream option to include Usage](https://github.com/ollama/ollama/issues/4448))
	- `prompt_tokens_details` and `completion_tokens_details` will have the value sent by the compatible provider, or `None`

- **(2)**: **Gemini** tokens
	- Right now, with the [Gemini Stream API](https://ai.google.dev/api/rest/v1beta/models/streamGenerateContent), it’s not clear whether usage for each event is cumulative or must be summed. It appears to be cumulative, meaning the last message shows the total number of input, output, and total tokens, so that is the current assumption. See [possible tweet answer](https://twitter.com/jeremychone/status/1813734565967802859) for more info.

## Usage examples

- [AIPack](https://aipack.ai) - Check out [AIPack](https://aipack.ai), which wraps this **genai** library into an agentic runtime to run, build, and share AI Agent Packs. See [`pro@coder`](https://news.aipack.ai/p/procoder-v052-demo-workbench) for a simple example of how I use AI PACK/genai for production coding.

- [zcoder](https://zcoder.run) - I am also in the process of building [zcoder](https://zcoder.run), which will be a parallel-first coding harness.

> Note: Feel free to send me a short description and a link to your application or library that uses genai. I'm happy to add it.

## TLS Backends

TLS is selectable via cargo features. The default works out of the box and is right for almost everyone.

| You want… | Use |
|---|---|
| A backend that just works (default) | nothing — `rustls-tls` (rustls + aws-lc-rs + OS trust store) |
| System trust policy / OpenSSL or SChannel | `genai = { default-features = false, features = ["native-tls"] }` |
| ring / FIPS / no `aws-lc-sys` (lean musl & cross builds) | `genai = { default-features = false }`, add your own `reqwest = { features = ["rustls-no-provider"] }` + a provider (e.g. `rustls` with `ring`), and call `CryptoProvider::install_default()` at startup |
| Static OpenSSL | `genai = { default-features = false }` + your own `reqwest = { features = ["native-tls-vendored"] }` |
| Custom root CA / mTLS / fully custom client | build your own `reqwest::Client` and inject it via `with_reqwest(...)` |

`rustls-tls` and `native-tls` are mutually exclusive. When selecting `native-tls`, set `default-features = false` (as shown above) — otherwise both backends compile and the build fails with a `compile_error!` telling you to do so.

`rustls-tls` reads the OS trust store via `rustls-platform-verifier`, so enterprise/custom CAs work without `native-tls`, and it needs no system OpenSSL. Note the default `aws-lc-rs` provider builds `aws-lc-sys` (a C/assembly crate); for a leaner static-musl or cross build, drop it with `default-features = false` and use the `ring` provider as in the table above.

For full control, build a `reqwest::Client` yourself and inject it:

```rust
let reqwest_client = reqwest::Client::builder()
    // .add_root_certificate(...), client identity for mTLS, custom CryptoProvider, etc.
    .build()?;

let client = genai::Client::builder()
    .with_reqwest(reqwest_client)
    .build();
```

If you set `default-features = false` on `genai` without enabling a TLS feature, add `reqwest` with a TLS feature directly to your own `Cargo.toml` (or use the `rustls-no-provider` path and install a `CryptoProvider`) — otherwise HTTPS requests fail at runtime.

## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
