# genai - Multiprovider Generative AI Client

<div align="center">

<a href="https://crates.io/crates/genai">
<img src="https://img.shields.io/crates/v/genai.svg" />
</a> 
<a href="https://github.com/jeremychone/rust-genai">
<img alt="Static Badge" src="https://img.shields.io/badge/GitHub-Repo?color=%23336699">
</a>

</div>

<br />

The goal of this library is to provide a common and ergonomic single API to many generative AI providers, such as OpenAI and Ollama.

- **IMPORTANT 1** `0.0.x` is still in heavy development. Cherry-pick code, don't depend on it. `0.0.x` releases will be yanked.

- **IMPORTANT 2** `0.1.x` will still have some breaking changes in patches, so make sure to **lock** your version, e.g., `genai = "=0.1.0"`. In short, `0.1.x` can be considered "beta releases."

- **IMPORTANT 3** This is not intended to be a replacement for [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs), but rather to tackle the simpler lowest common denominator of chat generation use cases, where API depth is less a priority than API commonality.

## Library Focus:

- Focuses on ergonomics and commonality first, and depth second. (If you need client API completeness, use [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs); they are awesome and relatively simple to use.)

- Initially, this library will mostly focus on text chat API (no simple generation, images, or even function calling in the first stage).

- The `0.1.x` version will work, but the APIs will change in the patch version, not following semver strictly.

- For now, it focuses on OpenAI and Ollama, using [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs).

- Version `0.2.x` will follow semver more strictly.

## Notes on Possible Direction

- Function calling will probably come before image support. The challenge is to normalize it between the OpenAI function API, which is relatively mature, and the open model ones, which are a little more ad hoc but still relatively well supported by some open models.

- It's probable that for `0.2.x` [async-openai](https://crates.io/search?q=async-openai) and [ollama-rs](https://crates.io/crates/ollama-rs) will be under features.

- One of the goals is to support serverless (i.e., without Ollama server) support for open models. [floneum](https://github.com/floneum/floneum) seems very promising but probably heavy (hence the feature approach for that provider).

## Links

- crates.io: [crates.io/crates/genai](https://crates.io/crates/genai)
- GitHub: [github.com/jeremychone/rust-genai](https://github.com/jeremychone/rust-genai)