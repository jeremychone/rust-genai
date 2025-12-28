# genai - Multi-AI Providers Library for Rust

Currently natively supports: **OpenAI**, **Anthropic**, **Gemini**, **XAI/Grok**, **Ollama**, **Groq**, **DeepSeek** (deepseek.com & Groq), **Cohere**, **Mimo** (more to come)

Also allows a custom URL with `ServiceTargetResolver` (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs))

<div align="center">

<a href="https://crates.io/crates/genai"><img src="https://img.shields.io/crates/v/genai.svg" /></a>
<a href="https://github.com/jeremychone/rust-genai"><img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699"></a>
<a href="https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_genai_Intro-Video?style=flat&logo=youtube&color=%23ff0000"></a>

</div>

<br />

Provides a single, ergonomic API to many generative AI providers, such as Anthropic, OpenAI, Gemini, xAI, Ollama, Groq, and more.

**NOTE:** Big update with **v0.4.x** - More adapters, PDF and image support, embeddings, custom headers, and transparent support for the OpenAI Responses API (gpt-5-codex)

## v0.4.x Big Release

- **What's new**:
    - **PDF and Images** support (thanks to [Andrew Rademacher](https://github.com/AndrewRademacher))
    - **Embedding** support (thanks to [Jesus Santander](https://github.com/jsantanders))
    - **Custom Headers** support (for AWS Bedrock, Vertex, etc.) (thanks to [Adrien](https://github.com/XciD)/[Julien Chaumond](https://github.com/julien-c))
    - **Simpler, flatter `MessageContent`** multi-part format (API change) (thanks to [Andrew Rademacher](https://github.com/AndrewRademacher) for insights)
    - **Raw body capture** with `ChatOptions::with_capture_raw_body(true)` (thanks to [4t145](https://github.com/4t145))
    - **Transparent gpt-5-codex support with the Responses API**, even if gpt-5-codex uses a new API protocol (OpenAI Responses API)
- **What's still awesome**:
    - Normalized and ergonomic Chat API across all providers
    - Most providers built in (OpenAI, Gemini, Anthropic, xAI, Groq, Together.ai, Fireworks.ai, ...)
    - Native protocol support for Gemini and Anthropic protocols, for example allowing full budget controls with Gemini models
    - Can override auth, endpoint, and headers to connect to AWS Bedrock, Vertex AI, etc.

See:
    - [migration from v0.3 to v0.4](doc/migration/migration-v_0_3_to_0_4.md)
    - [CHANGELOG](CHANGELOG.md)

## Big Thanks to

- [Vagmi Mudumbai](https://github.com/vagmi)) for [#96](https://github.com/jeremychone/rust-genai/pull/96) openai audio_type
- [Himmelschmidt](https://github.com/Himmelschmidt) for [#98](https://github.com/jeremychone/rust-genai/pull/98) openai service_tier
- [Bart Carroll](https://github.com/bartCarroll) for [#91](https://github.com/jeremychone/rust-genai/pull/91) Fixed streaming tool calls for openai models
- [Rui Andrada](https://github.com/shingonoide) for [#95](https://github.com/jeremychone/rust-genai/pull/95) refactoring ZHIPU adapter to ZAI
- [Adrien](https://github.com/XciD) Extra headers in requests, seed for chat requests, and fixes (with [Julien Chaumond](https://github.com/julien-c) for extra headers)
- [Andrew Rademacher](https://github.com/AndrewRademacher) for PDF support, Anthropic streamer, and insight on flattening the message content (e.g., ContentParts)
- [Jesus Santander](https://github.com/jsantanders) Embedding support [PR #83](https://github.com/jeremychone/rust-genai/pull/83)
- [4t145](https://github.com/4t145) for raw body capture [PR #68](https://github.com/jeremychone/rust-genai/pull/68)
- [Vagmi Mudumbai](https://github.com/vagmi) exec_chat bug fix [PR #86](https://github.com/jeremychone/rust-genai/pull/86)
- [Maximilian Goisser](https://github.com/hobofan) Fix OpenAI adapter to use ServiceTarget
- [ClanceyLu](https://github.com/ClanceyLu) for Tool Use Streaming support, web configuration support, and fixes
- [@SilasMarvin](https://github.com/SilasMarvin) for fixing content/tools issues with some Ollama models [PR #55](https://github.com/jeremychone/rust-genai/pull/55)
- [@una-spirito](https://github.com/luna-spirito) for Gemini `ReasoningEffort::Budget` support
- [@jBernavaPrah](https://github.com/jBernavaPrah) for adding tracing (it was long overdue). [PR #45](https://github.com/jeremychone/rust-genai/pull/45)
- [@GustavoWidman](https://github.com/GustavoWidman) for the initial Gemini tool/function support! [PR #41](https://github.com/jeremychone/rust-genai/pull/41)
- [@AdamStrojek](https://github.com/AdamStrojek) for initial image support [PR #36](https://github.com/jeremychone/rust-genai/pull/36)
- [@semtexzv](https://github.com/semtexzv) for `stop_sequences` Anthropic support [PR #34](https://github.com/jeremychone/rust-genai/pull/34)
- [@omarshehab221](https://github.com/omarshehab221) for de/serialize on structs [PR #19](https://github.com/jeremychone/rust-genai/pull/19)
- [@tusharmath](https://github.com/tusharmath) for making webc::Error [PR #12](https://github.com/jeremychone/rust-genai/pull/12)
- [@giangndm](https://github.com/giangndm) for making stream Send [PR #10](https://github.com/jeremychone/rust-genai/pull/10)
- [@stargazing-dino](https://github.com/stargazing-dino) for [PR #2](https://github.com/jeremychone/rust-genai/pull/2) - implement Groq completions

## Usage examples

- Check out [AIPACK](https://aipack.ai), which wraps this **genai** library into an agentic runtime to run, build, and share AI Agent Packs. See [`pro@coder`](https://www.youtube.com/watch?v=zL1BzPVM8-Y&list=PL7r-PXl6ZPcB2zN0XHsYIDaD5yW8I40AE) for a simple example of how I use AI PACK/genai for production coding.

> Note: Feel free to send me a short description and a link to your application or library using genai.

## Key Features

- Native Multi-AI Provider/Model: OpenAI, Anthropic, Gemini, Ollama, Groq, xAI, DeepSeek (Direct chat and stream) (see [examples/c00-readme.rs](examples/c00-readme.rs))
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
const MODEL_COHERE: &str = "command-light";
const MODEL_GEMINI: &str = "gemini-2.0-flash";
const MODEL_GROQ: &str = "llama-3.1-8b-instant";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`
const MODEL_XAI: &str = "grok-beta";
const MODEL_DEEPSEEK: &str = "deepseek-chat";
const MODEL_MIMO: &str = "mimo-v2-flash";

// NOTE: These are the default environment keys for each AI Adapter Type.
//       They can be customized; see `examples/c02-auth.rs`
const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	// -- De/activate models/providers
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_GEMINI, "GEMINI_API_KEY"),
	(MODEL_GROQ, "GROQ_API_KEY"),
	(MODEL_XAI, "XAI_API_KEY"),
	(MODEL_DEEPSEEK, "DEEPSEEK_API_KEY"),
	(MODEL_MIMO, "MIMO_API_KEY"),
	(MODEL_OLLAMA, ""),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - starts_with "gemini"   -> Gemini
//  - model in Groq models   -> Groq
//  - starts_with "mimo"     -> Mimo
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
- [examples/c04-chat-options.rs](examples/c04-chat-options.rs) - Demonstrates how to set chat generation options such as `temperature` and `max_tokens` at the client level (for all requests) and per-request level.
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

- Focuses on standardizing chat completion APIs across major AI services.

- Native implementation, meaning no per-service SDKs.
    - Reason: While there are some variations across the various APIs, they all follow the same pattern and high-level flow and constructs. Managing the differences at a lower layer is actually simpler and more cumulative across services than doing SDK gymnastics.

- Prioritizes ergonomics and commonality, with depth being secondary. (If you require a complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are both excellent and easy to use.)

- Initially, this library will mostly focus on text chat APIs, with images and function calling coming later.

## ChatOptions

- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo

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
| `total_tokens`              | `total_tokens`              | (computed)              | `totalTokenCount`  (2)     | (computed)            |
| `prompt_tokens_details`     | `prompt_tokens_details`     | `cached/cache_creation` | N/A for now                | N/A for now           |
| `completion_tokens_details` | `completion_tokens_details` | N/A for now             | N/A for now                | N/A for now           |


- **(1)** - **OpenAI-compatible** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI, Mimo
	- For **Groq**, the property `x_groq.usage.`  
	- At this point, **Ollama** does not emit input/output tokens when streaming due to the Ollama OpenAI compatibility layer limitation. (see [ollama #4448 - Streaming Chat Completion via OpenAI API should support stream option to include Usage](https://github.com/ollama/ollama/issues/4448))
	- `prompt_tokens_details` and `completion_tokens_details` will have the value sent by the compatible provider (or None)

- **(2)**: **Gemini** tokens
	- Right now, with the [Gemini Stream API](https://ai.google.dev/api/rest/v1beta/models/streamGenerateContent), it's not clear whether usage for each event is cumulative or must be summed. It appears to be cumulative, meaning the last message shows the total amount of input, output, and total tokens, so that is the current assumption. See [possible tweet answer](https://twitter.com/jeremychone/status/1813734565967802859) for more info.


## Notes on Possible Direction

- Will add more data on ChatResponse and ChatStream, especially metadata about usage.
- Add vision/image support to chat messages and responses.
- Add function calling support to chat messages and responses.
- Add `embed` and `embed_batch`.
- Add the AWS Bedrock variants (e.g., Mistral and Anthropic). Most of the work will be on the "interesting" token signature scheme; trying to avoid bringing in large SDKs, this might be a lower-priority feature.
- Add the Google Vertex AI variants.
- May add the Azure OpenAI variant (not sure yet).

## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
- Sponsored by [BriteSnow](https://britesnow.com) (Jeremy Chone's consulting company)