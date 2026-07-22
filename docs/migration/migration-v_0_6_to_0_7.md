# Migration Guide: v0.6.x to v0.7.0


### `Tool.custom_format`

`Tool` now includes `custom_format: Option<serde_json::Value>` for OpenAI Responses freeform custom tools. Existing `Tool` struct literals must add `custom_format: None`.

```rust
let tool = Tool {
    // existing fields
    custom_format: None,
};
```

Use `Tool::with_custom_format(...)` for OpenAI Responses `type: "custom"` tools.
