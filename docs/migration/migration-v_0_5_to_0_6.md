# Migration Guide: v0.5.x to v0.6.0-beta.x

This guide highlights the `genai` API changes needed when moving from the 0.5.x line to the 0.6.0 beta line.

## API Changes

- `Client::all_model_names(adapter_kind, impl Into<ProviderConfig>)` now takes a second Provider Config (can be `None` for default). (see [Client::all_model_names](#all_model_names))



## Code Examples


### all_model_names

```rust
// -- v0.5.x

let models = client.all_model_names(adapter_kind).await?;

// -- v0.0.6

// Same as v0.5.x
let models = client.all_model_names(adapter_kind, None).await?;

// ProviderConfig { 	pub endpoint: Option<Endpoint>, pub auth: Option<AuthData>}

// Remote Ollama endpoint override
let models = client
	.all_model_names(
		AdapterKind::Ollama,
		Endpoint::from_static("http://remote-host:11434/"),
	)
	.await?;

// Custom openai compatible provider
let models = client
	.all_model_names(
		AdapterKind::OpenAI,
		(
			Endpoint::from_static("http://fair-llm.ai/"),
			AuthData::from_env("FAIR_LLM_API_KEY"),
		),
	)
	.await?;
```
