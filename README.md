# genai - Multiprovider Generative AI Client

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
genai = {version: '=0.0.3', features = ["with-all-providers"]}
```

<br />

The goal of this library is to provide a common and ergonomic single API to many generative AI providers, such as OpenAI and Ollama.

- **IMPORTANT 1** `0.0.x` is still in heavy development. Cherry-pick code, don't depend on it. `0.0.x` releases will be yanked.

- **IMPORTANT 2** `0.1.x` will still have some breaking changes in patches, so make sure to **lock** your version, e.g., `genai = "=0.1.0"`. In short, `0.1.x` can be considered "beta releases."

- **IMPORTANT 3** This is not intended to be a replacement for [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs), but rather to tackle the simpler lowest common denominator of chat generation use cases, where API depth is less a priority than API commonality.

## Library Focus:

- Focuses on standardizing chat completion APIs across major AI Providers.

- Prioritizes ergonomics and commonality, with depth being secondary. (If you require complete client API, consider using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are both excellent and easy to use.)

- Initially, this library will mostly focus on text chat API (no simple generation, images, or even function calling in the first stage).

- The `0.1.x` version will work, but the APIs will change in the patch version, not following semver strictly.

- Version `0.2.x` will follow semver more strictly.

## Notes on Possible Direction

- Currently, it uses [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs) to communicate with those respective services. However, the goal is to implement native request implementation for those services. All of these web APIs are quite similar, and they all use Server-Sent Events (SSE) for streaming. Managing the differences at a lower layer is actually simpler and more cumulative, requiring less complex handling overall.

- Function calling will probably come before image support. The challenge is to normalize it between the OpenAI function API, which is relatively mature, and the open model ones, which are a little more ad hoc but still relatively well supported by some open models.


## Dev Commands

Here are some quick dev commands. 

```sh
# cargo watch c01-simple
cargo watch -q -x "run -q --example c01-simple"

# cargo watch c02-stream
cargo watch -q -x "run -q --example c02-stream"

# cargo watch c03-conv
cargo watch -q -x "run -q --example c03-conv"

```

## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)