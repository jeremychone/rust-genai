# genai - Multi-AI Providers Library for Rust

Currently supports natively: **DeepSeek** (deepseek.com & Groq), **OpenAI**, **Anthropic**, **Groq**, **Ollama**, **Gemini**, **Cohere** (more to come)

<div align="center">

<a href="https://crates.io/crates/genai"><img src="https://img.shields.io/crates/v/genai.svg" /></a>
<a href="https://github.com/jeremychone/rust-genai"><img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699"></a>
<a href="https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_genai_Intro-Video?style=flat&logo=youtube&color=%23ff0000"></a>

</div>

```toml
# cargo.toml
genai = "0.1.21"
```

<br />

Provides a common and ergonomic single API to many generative AI providers, such as Anthropic, OpenAI, Gemini, xAI, Ollama, Groq, and more.

Check out [devai.run](https://devai.run), the **Iterate to Automate** command-line application that leverages **genai** for multi-AI capabilities.

## Thanks

- [@GustavoWidman](https://github.com/GustavoWidman) for the intial gemini tool/function support!! [PR #41](https://github.com/jeremychone/rust-genai/pull/41)
- [@AdamStrojek](https://github.com/AdamStrojek) for initial image support [PR #36](https://github.com/jeremychone/rust-genai/pull/36)
- [@semtexzv](https://github.com/semtexzv) for `stop_sequences` Anthropic support [PR #34](https://github.com/jeremychone/rust-genai/pull/34)
- [@omarshehab221](https://github.com/omarshehab221) for de/serialize on structs [PR #19](https://github.com/jeremychone/rust-genai/pull/19)
- [@tusharmath](https://github.com/tusharmath) for make webc::Error [PR #12](https://github.com/jeremychone/rust-genai/pull/12)
- [@giangndm](https://github.com/giangndm) for make stream is send [PR #10](https://github.com/jeremychone/rust-genai/pull/10)
- [@stargazing-dino](https://github.com/stargazing-dino) for [PR #2](https://github.com/jeremychone/rust-genai/pull/2) - implement Groq completions



## Key Features

- DeepSeekR1 support, with `reasoning_content` (and stream support) + DeepSeek Groq and Ollama support (and `reasoning_content` normalization)
- Native Multi-AI Provider/Model: OpenAI, Anthropic, Gemini, Ollama, Groq, xAI, DeepSeek (Direct chat and stream) (see [examples/c00-readme.rs](examples/c00-readme.rs))
- Image Analysis (for OpenAI, Gemini flash-2, Anthropic) (see [examples/c07-image.rs](examples/c07-image.rs))
- Custom Auth/API Key (see [examples/c02-auth.rs](examples/c02-auth.rs))
- Model Alias (see [examples/c05-model-names.rs](examples/c05-model-names.rs))
- Custom Endpoint, Auth, and Model Identifier (see [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs))

[Examples](#examples) | [Thanks](#thanks) | [Library Focus](#library-focus) | [Changelog](CHANGELOG.md) | Provider Mapping: [ChatOptions](#chatoptions) | [MetaUsage](#metausage)

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
const MODEL_GEMINI: &str = "gemini-1.5-flash-latest";
const MODEL_GROQ: &str = "llama3-8b-8192";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`
const MODEL_XAI: &str = "grok-beta";
const MODEL_DEEPSEEK: &str = "deepseek-chat";

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
	(MODEL_OLLAMA, ""),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - starts_with "gemini"   -> Gemini
//  - model in Groq models   -> Groq
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

		let adapter_kind = client.resolve_service_target(model)?.model.adapter_kind;

		println!("\n===== MODEL: {model} ({adapter_kind}) =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer:");
		let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
		println!("{}", chat_res.content_text_as_str().unwrap_or("NO ANSWER"));

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
- [examples/c04-chat-options.rs](examples/c04-chat-options.rs) - Demonstrates how to set chat generation options such as `temperature` and `max_tokens` at the client level (for all requests) and per request level.
- [examples/c05-model-names.rs](examples/c05-model-names.rs) - Shows how to get model names per AdapterKind.
- [examples/c06-target-resolver.rs](examples/c06-target-resolver.rs) - For custom Auth, Endpoint, and Model.
- [examples/c07-image.rs](examples/c07-image.rs) - Image Analysis support

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
    - Reason: While there are some variations between all of the various APIs, they all follow the same pattern and high-level flow and constructs. Managing the differences at a lower layer is actually simpler and more cumulative across services than doing SDKs gymnastics.

- Prioritizes ergonomics and commonality, with depth being secondary. (If you require a complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are both excellent and easy to use.)

- Initially, this library will mostly focus on text chat API (images, or even function calling in the first stage).

- The `0.1.x` version will work, but the APIs will change in the patch version, not following semver strictly.

- Version `0.2.x` will follow semver more strictly.

## ChatOptions

- **(1)** - **OpenAI compatibles** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI

| Property      | OpenAI Compatibles (*1) | Anthropic                   | Gemini `generationConfig.` | Cohere        |
|---------------|-------------------------|-----------------------------|----------------------------|---------------|
| `temperature` | `temperature`           | `temperature`               | `temperature`              | `temperature` |
| `max_tokens`  | `max_tokens`            | `max_tokens` (default 1024) | `maxOutputTokens`          | `max_tokens`  |
| `top_p`       | `top_p`                 | `top_p`                     | `topP`                     | `p`           |

## MetaUsage

| Property                    | OpenAI Compatibles (1)      | Anthropic `usage.`      | Gemini `usageMetadata.`    | Cohere `meta.tokens.` |
|-----------------------------|-----------------------------|-------------------------|----------------------------|-----------------------|
| `prompt_tokens`             | `prompt_tokens`             | `input_tokens` (added)  | `promptTokenCount` (2)     | `input_tokens`        |
| `completion_tokens`         | `completion_tokens`         | `output_tokens` (added) | `candidatesTokenCount` (2) | `output_tokens`       |
| `total_tokens`              | `total_tokens`              | (computed)              | `totalTokenCount`  (2)     | (computed)            |
| `prompt_tokens_details`     | `prompt_tokens_details`     | N/A for now             | N/A for now                | N/A for now           |
| `completion_tokens_details` | `completion_tokens_details` | N/A for now             | N/A for now                | N/A for now           |


- **(1)** - **OpenAI compatibles** notes
	- Models: OpenAI, DeepSeek, Groq, Ollama, xAI
	- For **Groq**, the property `x_groq.usage.`  
	- At this point, **Ollama** does not emit input/output tokens when streaming due to the Ollama OpenAI compatibility layer limitation. (see [ollama #4448 - Streaming Chat Completion via OpenAI API should support stream option to include Usage](https://github.com/ollama/ollama/issues/4448))
	- `prompt_tokens_details` and `completion_tokens_details` will have the value sent by the compatible provider (or None)

- **(2)**: **Gemini** tokens
	- Right now, with [Gemini Stream API](https://ai.google.dev/api/rest/v1beta/models/streamGenerateContent), it's not really clear if the usage for each event is cumulative or needs to be added. Currently, it appears to be cumulative (i.e., the last message has the total amount of input, output, and total tokens), so that will be the assumption. See [possible tweet answer](https://twitter.com/jeremychone/status/1813734565967802859) for more info.


## Notes on Possible Direction

- Will add more data on ChatResponse and ChatStream, especially metadata about usage.
- Add vision/image support to chat messages and responses.
- Add function calling support to chat messages and responses.
- Add `embed` and `embed_batch`
- Add the AWS Bedrock variants (e.g., Mistral, and Anthropic). Most of the work will be on "interesting" token signature scheme (without having to drag big SDKs, might be below feature).
- Add the Google VertexAI variants.
- (might) add the Azure OpenAI variant (not sure yet).


## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
- Sponsored by [BriteSnow](https://britesnow.com) (Jeremy Chone's consulting company)
