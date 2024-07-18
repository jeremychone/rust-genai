# genai - Multi-AI Providers Library for Rust.

Currently supports natively: **Ollama**, **OpenAI**, **Gemini**, **Anthropic**, **Cohere** (more to come)

<div align="center">

<a href="https://crates.io/crates/genai"><img src="https://img.shields.io/crates/v/genai.svg" /></a>
<a href="https://github.com/jeremychone/rust-genai"><img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699"></a>
<a href="https://www.youtube.com/watch?v=uqGso3JD3eE&list=PL7r-PXl6ZPcCIOFaL7nVHXZvBmHNhrh_Q"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube_genai_Intro-Video?style=flat&logo=youtube&color=%23ff0000"></a>

</div>

```toml
# cargo.toml
genai = "=0.1.3" # Version lock for `0.1.x`
```

<br />

The goal of this library is to provide a common and ergonomic single API to many generative AI Providers, such as OpenAI, Anthropic, Cohere, Ollama.

- **IMPORTANT 1** `0.1.x` will still have some breaking changes in patches, so make sure to **lock** your version, e.g., `genai = "=0.1.3"`. In short, `0.1.x` can be considered "beta releases." Version `0.2.x` will follow semver more strictly.

- **IMPORTANT 2** `genai` is focused on normalizing chat completion APIs across AI providers and is not intended to be a full representation of a given AI provider. For this, there are excellent libraries such as [async-openai](https://crates.io/search?q=async-openai) for OpenAI and [ollama-rs](https://crates.io/crates/ollama-rs) for Ollama.

[Examples](#examples) | [Thanks](#thanks) | [Library Focus](#library-focus) | [Changelog](CHANGELOG.md) | [ChatRequestOptions Provider Mapping](#chatrequestoptions)

## Examples

[examples/c00-readme.rs](examples/c00-readme.rs)

```rust
use genai::chat::{ChatMessage, ChatRequest};
use genai::utils::{print_chat_stream, PrintChatStreamOptions};
use genai::Client;

const MODEL_OPENAI: &str = "gpt-4o-mini";
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
const MODEL_COHERE: &str = "command-light";
const MODEL_GEMINI: &str = "gemini-1.5-flash-latest";
const MODEL_GROQ: &str = "gemma-7b-it";
const MODEL_OLLAMA: &str = "gemma:2b"; // sh: `ollama pull gemma:2b`

// NOTE: Those are the default environment keys for each AI Adapter Type.
//       Can be customized, see `examples/c02-auth.rs`
const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	// -- de/activate models/providers
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_GEMINI, "GEMINI_API_KEY"),
	(MODEL_GROQ, "GROQ_API_KEY"),
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
// Can be customized, see `examples/c03-kind.rs`

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
		// Skip if does not have the environment name set
		if !env_name.is_empty() && std::env::var(env_name).is_err() {
			println!("===== Skipping model: {model} (env var not set: {env_name})");
			continue;
		}

		let adapter_kind = client.resolve_adapter_kind(model)?;

		println!("\n===== MODEL: {model} ({adapter_kind}) =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer:");
		let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
		println!("{}", chat_res.content.as_deref().unwrap_or("NO ANSWER"));

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
- [examples/c03-kind.rs](examples/c03-kind.rs) - Demonstrates how to provide a custom `AdapterKindResolver` to customize the "model name" to "adapter kind" mapping.
- [examples/c04-chat-options.rs](examples/c04-chat-options.rs) - Demonstrates how to set chat generation options such as `temperature` and `max_tokens` at the client level (for all requests) and per request level.
- [examples/c05-model-names.rs](examples/c05-model-names.rs) - Show how to get model names per AdapterKind.

## Thanks

- Thanks to [@stargazing-dino](https://github.com/stargazing-dino) for [PR #2](https://github.com/jeremychone/rust-genai/pull/2) - implement groq completions


## Library Focus:

- Focuses on standardizing chat completion APIs across major AI Services.

- Native implementation, meaning no per-service SDKs. 
    - Reason: While there are some variations between all of the various APIs, they all follow the same pattern and high-level flow and constructs. Managing the differences at a lower layer is actually simpler and more cumulative accross services than doing sdks gymnastic.

- Prioritizes ergonomics and commonality, with depth being secondary. (If you require complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are both excellent and easy to use.)

- Initially, this library will mostly focus on text chat API (images, or even function calling in the first stage).

- The `0.1.x` version will work, but the APIs will change in the patch version, not following semver strictly.

- Version `0.2.x` will follow semver more strictly.

## ChatRequestOptions

| Property    | OpenAI      | Anthropic                 | Ollama      | Groq        | Gemini                           | Cohere      |
|-------------|-------------|---------------------------|-------------|-------------|----------------------------------|-------------|
| temperature | temperature | temperature               | temperature | temperature | generationConfig.temperature     | temperature |
| max_tokens  | max_tokens  | max_tokens (default 1024) | max_tokens  | max_tokens  | generationConfig.maxOutputTokens | max_tokens  |
| top_p       | top_p       | top_p                     | top_p       | top_p       | generationConfig.topP            | p           |

## MetaUsage

| Property        | OpenAI <br />`usage.` | Ollama <br />`usage.`   | Groq `x_groq.usage.` | Anthropic `usage.`      | Gemini `usageMetadata.`    | Cohere `meta.tokens.` |
|-----------------|-----------------------|-------------------------|----------------------|-------------------------|----------------------------|-----------------------|
| `input_tokens`  | `prompt_tokens`       | `prompt_tokens` (1)     | `prompt_tokens`      | `input_tokens` (added)  | `promptTokenCount` (2)     | `input_tokens`        |
| `output_tokens` | `completion_tokens`   | `completion_tokens` (1) | `completion_tokens`  | `output_tokens` (added) | `candidatesTokenCount` (2) | `output_tokens`       |
| `total_tokens`  | `total_tokens`        | `total_tokens` (1)      | `completion_tokens`  | (computed)              | `totalTokenCount`  (2)     | (computed)            |

> **Note (1)**: At this point, `Ollama` does not emit input/output tokens when streaming due to the Ollama OpenAI compatibility layer limitation. (see [ollama #4448 - Streaming Chat Completion via OpenAI API should support stream option to include Usage](https://github.com/ollama/ollama/issues/4448))

> **Note (2)** Right now, with [Gemini Stream API](https://ai.google.dev/api/rest/v1beta/models/streamGenerateContent), it's not really clear if the usage for each event is cumulative or needs to be added. Currently, it appears to be cumulative (i.e., the last message has the total amount of input, output, and total tokens), so that will be the assumption. See [possible tweet answer](https://twitter.com/jeremychone/status/1813734565967802859) for more info. 


## Notes on Possible Direction

- Will add more data on ChatResponse and ChatStream, especially metadata about usage.
- Add vision/image support to chat messages and responses.
- Add function calling support to chat messages and responses.
- Add `embbed` and `embbed_batch`
- Add the AWS Bedrock variants (e.g., Mistral, and Anthropic). Most of the work will be on "interesting" token signature scheme (without having to drag big SDKs, might be below feature).
- Add the Google VertexAI variants.
- (might) add the Azure OpenAI variant (not sure yet).


## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)
- Sponsored by [BriteSnow](https://britesnow.com) (Jeremy Chones's consulting company)