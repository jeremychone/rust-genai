# genai - Multiprovider Generative AI Client

Currently supports natively: **Ollama**, **OpenAI**, **Anthropic**, **Cohere** (more to come)

<div align="center">

<a href="https://crates.io/crates/genai">
<img src="https://img.shields.io/crates/v/genai.svg" />
</a> 
<a href="https://github.com/jeremychone/rust-genai">
<img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699">
</a>

</div>

```toml
# cargo.toml
genai = {version: '=0.0.9'}
```

<br />

The goal of this library is to provide a common and ergonomic single API to many generative AI Providers, such as OpenAI, Anthropic, Cohere, Ollama.

- **IMPORTANT 1** `0.0.x` is still in heavy development. Cherry-pick code, don't depend on it. (It's starting to work pretty well though)

- **IMPORTANT 2** `0.1.x` will still have some breaking changes in patches, so make sure to **lock** your version, e.g., `genai = "=0.1.0"`. In short, `0.1.x` can be considered "beta releases."

- **IMPORTANT 3** This is **NOT** intended to be a replacement for [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs), but rather to tackle the simpler lowest common denominator of chat generation use cases, where API depth is less a priority than API commonality.

## Library Focus:

- Focuses on standardizing chat completion APIs across major AI Services.

- Native implementation, meaning no per-service SDKs. 
	- Reason: While there are some variations between all of the various APIs, they all follow the same pattern and high-level flow and constructs. Managing the differences at a lower layer is actually simpler and more cumulative accross services than doing sdks gymnastic.

- Prioritizes ergonomics and commonality, with depth being secondary. (If you require complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are both excellent and easy to use.)

- Initially, this library will mostly focus on text chat API (images, or even function calling in the first stage).

- The `0.1.x` version will work, but the APIs will change in the patch version, not following semver strictly.

- Version `0.2.x` will follow semver more strictly.

## Example

[`examples/c00-readme.rs`](examples/c00-readme.rs)

```rust
mod support; // For examples support funtions
use crate::support::{has_env, print_chat_stream};

use genai::chat::{ChatMessage, ChatRequest};
use genai::client::Client;

const MODEL_OPENAI: &str = "gpt-3.5-turbo";
const MODEL_ANTHROPIC: &str = "claude-3-haiku-20240307";
const MODEL_COHERE: &str = "command-light"; // see: https://docs.cohere.com/docs/models
const MODEL_OLLAMA: &str = "mixtral";

const MODEL_AND_KEY_ENV_NAME_LIST: &[(&str, &str)] = &[
	// -- de/activate models/providers
	(MODEL_OPENAI, "OPENAI_API_KEY"),
	(MODEL_ANTHROPIC, "ANTHROPIC_API_KEY"),
	(MODEL_COHERE, "COHERE_API_KEY"),
	(MODEL_OLLAMA, ""),
];

// NOTE: Model to AdapterKind (AI Provider) type mapping rule
//  - starts_with "gpt"      -> OpenAI
//  - starts_with "claude"   -> Anthropic
//  - starts_with "command"  -> Cohere
//  - For anything else      -> Ollama
//
// Refined mapping rules will be added later and extended as provider support grows.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Why is the sky red?";

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		// ChatMessage::system("Answer in one sentence"),
		ChatMessage::user(question),
	]);

	let client = Client::default();

	for (model, env_name) in MODEL_AND_KEY_ENV_NAME_LIST {
		// Skip if does not have the environment name set
		if !env_name.is_empty() && !has_env(env_name) {
			continue;
		}

		println!("\n===== MODEL: {model} =====");

		println!("\n--- Question:\n{question}");

		println!("\n--- Answer: (oneshot response)");
		let chat_res = client.exec_chat(model, chat_req.clone()).await?;
		println!("{}", chat_res.content.as_deref().unwrap_or("NO ANSWER"));

		println!("\n--- Answer: (streaming)");
		let chat_res = client.exec_chat_stream(model, chat_req.clone()).await?;
		print_chat_stream(chat_res).await?;

		println!();
	}

	Ok(())
}
```


## Notes on Possible Direction

- Will add more data on ChatResponse and ChatStream, especially metadata about usage.

- Add vision/image support to the chat messages and responses

- Add function calling support to the chat messages and responses

- Add Google Gemini (note: Seems that gemini endpoints is diffferent than the google vertex AI as only the later seems to support Function & Instruction.)

- Add the AWS Berock variants (e.g., Mistral, and Anthropic). Most of the work will be on "interesting" token signature scheme (without having to drag big SDKs, might be below feature)


## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)