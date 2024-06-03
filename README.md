# genai - Multiprovider generative AI client

The goal of this library is to provide a common and ergonomic single API to many generative AI providers, such as OpenAI and Ollama.

**IMPORTANT 1** This is in development, `0.1.x` will have some breaking change on patches, and thefore, make sure to **lock** your version e.g., `genai = "=0.1.0"` 

**IMPORTANT 2** This is not intended to be a replacement for [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs), but rather to tackle the simpler lowest common denominator of chat generation in a single API.

## Library focus:

- Focuses on ergonomic and commonolity first, and depth second. (if you need client api completness, use [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs))

- For starter, it will mostly focus on text chat API (no simple generation, just chat API which is typically what is needed for many usecases)

- The `0.1.x` will work, but the APIs will change in the patch version, not following semver strictly.

- For now, focuses on OpenAI and Ollama, using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs)

- Version `0.2.x` will follow semver more strictly.

## Notes on possible direction


- It's probable that for `0.2.x` will have [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs)) under features. 

- One of the goal is to support server less (i.e. without ollama server) support for open models. [floneum](https://github.com/floneum/floneum) seems very promissing but probably heavy (hence the feature approach for that adapter)