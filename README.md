# genai - Multiprovider generative AI client

The goal of this library is to provide a common and ergonomic single API to many generative AI providers, such as OpenAI and Ollama.

**IMPORTANT 1** This is in development, `0.0.x` are not intended to work. Do not use at this point. Wait for `0.1.x` see below.

**IMPORTANT 2** This is not intended to be a replacement for [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs), but rather to tackle the simpler lowest common denominator of chat generation in a single API.

Scope for now:

- Focuses on OpenAI and Ollama, using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs)

- For the `0.0.x` version, it is just code that might not even work. Don't waste your time.

- The `0.1.x` version will start to work, but the APIs will change in the patch version, not following semver strictly.

- Version `0.2.x` will follow semver more strictly.