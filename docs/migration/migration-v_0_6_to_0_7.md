# Migration Guide: v0.6.x to v0.7.0

## API Breaking Changes

### Fallible `Client` construction and builder

`Client` building is now fallible to enforce genai's zero-panic strategy, eliminating internal `.expect(...)` calls during `reqwest` client initialization. `Client::default()` has been removed to align with **genai's zero-panic strategy**.

- `Client::new()` now returns `Result<Client>`
- `ClientBuilder::build()` now returns `Result<Client>`.
- `Client::default()` has been removed.

Sorry for the inconvenience, but this was a necessary update.

**Now:**

```rust
let client = Client::new()?;
let client = Client::builder().with_chat_options(options).build()?;
```

**Before:**

```rust
let client = Client::default();
let client = Client::builder().with_chat_options(options).build();
```

### `ReasoningEffort::None` renamed to `ReasoningEffort::Zero`

`ReasoningEffort::None` has been renamed to `ReasoningEffort::Zero` to avoid confusion with `Option::None`. The canonical keyword is now `"zero"`. `from_keyword()` still accepts `"none"` as a backward-compatible alias.

### `JsonSpec::schema_with_additional_properties_false` removed

The public `JsonSpec::schema_with_additional_properties_false` helper has been removed. Provider adapters now sanitize schemas as required by their target API. The new public `JsonSchemaDialect` enum and `sanitize_json_schema(...)` function are available for callers that need explicit schema sanitization.

## API New Properties / Variants

### `Error::HttpError` response headers

`Error::HttpError` now includes `headers: Box<HeaderMap>` for failed streaming HTTP responses. Existing exhaustive matches and constructors must add `headers` or use `..`; retry metadata such as `retry-after` is now available.

### `Tool.custom_format` struct field

`Tool` now includes `custom_format: Option<serde_json::Value>` for OpenAI Responses freeform custom tools. Existing `Tool` struct literals must add `custom_format: None`, or migrate to `Tool::new(...)` and builder methods.

```rust
let tool = Tool {
    // existing fields
    custom_format: None,
};
```

Use `Tool::with_custom_format(...)` for OpenAI Responses `type: "custom"` tools.

### `ChatOptions.raw_frame_sink` struct field

`ChatOptions` now includes `raw_frame_sink: Option<Arc<dyn ChatFrameSink>>` for observing raw stream frames across providers. Existing `ChatOptions` struct literals must add `raw_frame_sink: None` or use `..Default::default()`.

```rust
let options = ChatOptions {
    // existing fields
    raw_frame_sink: None,
    ..Default::default()
};
```

Use `ChatOptions::with_raw_frame_sink(...)`, `with_raw_frame_sink_arc(...)`, or `with_raw_frame_fn(...)` to attach sinks during streaming calls.

## Behavior Refinement / Changes

### `ServiceTargetResolver` in `Client::all_model_names`

`Client::all_model_names()` now invokes `ServiceTargetResolver` in addition to `AuthResolver` to resolve custom endpoints.

Custom resolvers receive a `ModelIden` with an empty `model_name` for adapter-level requests. Implementations should handle empty model names when resolving endpoints.

### JSON Schema sanitization behavior

JSON Schema handling for OpenAI and Anthropic structured outputs and strict tools is now provider-specific. Non-strict tool schemas are forwarded unchanged. Strict schemas may have provider-required constraints added, such as `additionalProperties: false` and required property entries.

### Anthropic and Gemini reasoning effort behavior

- Anthropic: `ReasoningEffort::Zero` positively disables reasoning. Sonnet 5 sends `thinking: {"type": "disabled"}`. Fable and Mythos omit thinking since it is always active.
- Anthropic signed thinking: When continuing an extended-thinking conversation with tool use, signed thinking blocks are now preserved across turns. If an assistant message contains unpaired reasoning text and signatures (for example, across provider handoffs), thinking blocks are safely omitted with a warning.
- Gemini: `ReasoningEffort::Zero` maps to a thinking budget of `0`.

## Additive Features

### Declarative provider configuration on `ClientBuilder`

`ClientBuilder::append_provider_config` allows configuring per-adapter static endpoints and credentials without writing custom `AuthResolver` or `ServiceTargetResolver` closures:

```rust
let client = Client::builder()
    .append_provider_config(
        AdapterKind::OpenAI,
        (Endpoint::from_static("https://gateway.internal/v1/"), AuthData::from_env("GATEWAY_KEY")),
    )
    .append_provider_config(AdapterKind::Ollama, Endpoint::from_static("http://localhost:11434/v1/"))
    .build()?;
```

Configuration precedence is: built-in adapter defaults, then `append_provider_config`, then dynamic resolvers.

### `ChatResponse::into_assistant_message_for_tool_use`

`ChatResponse` now provides `into_assistant_message_for_tool_use()`, matching `StreamEnd::into_assistant_message_for_tool_use()`. This helper retains provider continuation metadata (such as signed thinking blocks and thought signatures) when constructing the assistant message for a tool continuation.

### OpenAI-compatible video content

`Binary` and `ContentPart` now expose `is_video()` helpers. OpenAI-compatible adapters serialize video binaries as `video_url` content parts instead of generic `file` parts.

Existing integrations that implement or inspect content-part handling may use these helpers to detect video attachments.
