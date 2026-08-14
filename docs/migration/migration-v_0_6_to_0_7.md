# Migration Guide: v0.6.x to v0.7.0


### `Error::HttpError.headers`

`Error::HttpError` now includes `headers: Box<HeaderMap>` for failed streaming HTTP responses. Existing exhaustive matches and constructors must add `headers` or use `..`; retry metadata such as `retry-after` is now available.

### `Tool.custom_format`

`Tool` now includes `custom_format: Option<serde_json::Value>` for OpenAI Responses freeform custom tools. Existing `Tool` struct literals must add `custom_format: None`.

```rust
let tool = Tool {
    // existing fields
    custom_format: None,
};
```

Use `Tool::with_custom_format(...)` for OpenAI Responses `type: "custom"` tools.

### OpenAI-compatible video content

`Binary` and `ContentPart` now expose `is_video()` helpers. OpenAI-compatible adapters serialize video binaries as `video_url` content parts instead of generic `file` parts.

Existing integrations that implement or inspect content-part handling may use these helpers to detect video attachments.

### JSON Schema sanitization

JSON Schema handling for OpenAI and Anthropic structured outputs and strict tools is now provider-specific.

The public `JsonSpec::schema_with_additional_properties_false` helper has been removed. Provider adapters now sanitize schemas as required by their target API. The new public `JsonSchemaDialect` enum and `sanitize_json_schema(...)` function are available for callers that need the same provider-aware behavior.

Non-strict tool schemas are forwarded unchanged. Strict schemas may have provider-required constraints added, such as `additionalProperties: false` and required property entries.
