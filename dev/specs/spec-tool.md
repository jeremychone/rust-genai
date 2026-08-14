# Tool Specification

## 1. Overview

The tool sub-system provides a unified interface for LLM function calling across different providers. It allows users to define function specifications that the model can choose to "call" when it determines that external information or action is required.

## 2. Code Design Pattern

This module follows a Unified Data Structure pattern. Instead of using provider-specific traits or complex abstraction layers, `genai` uses a single set of concrete structures that all adapters are responsible for mapping.

- **Symmetry**: `ToolCall` and `ToolResponse` are designed for easy conversion into `ChatMessage` variants, facilitating simple chat loop implementations.
- **Data Persistence**: All tool types implement `Serialize` and `Deserialize` to allow for easy storage and retrieval of chat histories that include tool interactions.

## 3. Public API

The tool system is exposed primarily through the `genai::chat` module.

### Core Types

#### `Tool` - Represents a tool definition (metadata and parameters).

```rust
pub struct Tool {
    pub name: ToolName,
    pub description: Option<String>,
    pub schema: Option<Value>,
    pub config: Option<ToolConfig>,
}
```

- `Tool::new(name)`: Primary constructor.
- `Tool::new_web_search()`: Constructor for the built-in web search tool.
- `with_schema(value)`: Builder-style method to set the JSON parameters schema.
- `with_description(text)`: Builder-style method to set the tool description.
- `with_config(value)`: Builder-style method to set provider-specific tool configuration.

#### `ToolName` - Normalized tool identifier.

- `ToolName::Custom(String)`: User-defined custom tool name, serialized as a bare string.
- `ToolName::WebSearch`: Built-in web search tool, serialized in a qualified form.

#### `ToolConfig` - Tool configuration payload.

- `ToolConfig::Custom(Value)`: Arbitrary JSON configuration for custom tools.
- `ToolConfig::WebSearch(WebSearchConfig)`: Typed configuration for the built-in web search tool.

#### `ToolCall` - Represents an invocation request emitted by the model.

```rust
pub struct ToolCall {
    pub call_id: String,
    pub fn_name: String,
    pub fn_arguments: Value,
    pub thought_signatures: Option<Vec<String>>,
}
```

#### `ToolResponse` - Represents the result of a tool execution.

```rust
pub struct ToolResponse {
    pub call_id: String,
    pub fn_name: Option<String>,
    pub content: String,
    pub parts: Option<Vec<Binary>>,
}
```

- `ToolResponse::new(call_id, content)`: Links the execution output back to the original call.
- `ToolResponse::from_tool_call(&tool_call, content)`: Convenience constructor that also captures `fn_name` (needed by Gemini's `functionResponse.name`).
- `with_fn_name(name)`: Builder-style method to set the function/tool name.
- `with_parts(parts)` / `append_binary(binary)`: Builder-style methods to attach binary parts (e.g., screenshots) to the tool result.

Adapter behavior for `parts` (image parts only; non-image parts are skipped with a warning on every adapter, and a `ToolResponse` without `parts` keeps its exact legacy serialization everywhere):

- **Anthropic**: image parts serialize natively as base64 `image` blocks inside the `tool_result` content array, after the text block (text block omitted when the text is empty). URL-based image sources are skipped with a warning (matching user-message image handling).
- **Bedrock (Converse)**: image parts serialize natively as `image` blocks inside the `toolResult` content array, after the text block. Base64 only (Converse binary handling does not support URLs).
- **OpenAI Responses**: image parts serialize natively: `function_call_output.output` becomes an array of `input_text` (when text is non-empty) plus `input_image` items (`detail: "auto"`, `image_url` as data URL or plain URL). `custom_tool_call_output` stays a raw string with the same placeholder rules as Chat Completions (`"(see attached image)"` / `"(no tool output)"`); its images are rescued into a follow-up `user` message input item (`input_text` label + `input_image` items), batched across a run of consecutive Tool messages and emitted after the run.
- **OpenAI Chat Completions** (shared by all OpenAI-compatible providers: Groq, Together, Fireworks, DeepSeek, etc.): the `tool` message stays text-only. When parts are present, its content is the text, or `"(see attached image)"` when the result has images but no text, or `"(no tool output)"` when it has neither. The images then ride in a follow-up `user` message with content `[{type: "text", text: "Attached image(s) from tool result:"}, ...image_url blocks]`. Images from a run of consecutive Tool messages are batched into ONE follow-up user message, emitted before the next non-tool message.
- **Gemini** (also Vertex/Google): the `functionResponse` content stays text-only with the same placeholder rules as Chat Completions. Images from Tool-role messages ride in a follow-up `user` turn (label text part + `inline_data` for base64 / `file_data` for URLs), batched across the run of Tool messages and emitted after them, so the `functionResponse` turns can still be merged into the single user turn the Gemini FC protocol requires. For a `ToolResponse` embedded in a User-role message, the images are appended inline in that same user turn instead.
- **Ollama (native)**: the `tool` message stays text-only with the same placeholder rules. Base64 images ride in a follow-up `user` message using the native `images` array (one follow-up per tool message); URL sources are skipped with a warning.

Notes:

- genai has no model-capability catalog, so the fallback is emitted whenever parts exist — attaching parts is the caller's opt-in that the target model accepts image input. No interstitial-assistant compatibility message is inserted, and Gemini is not version-gated for multimodal `functionResponse` nesting (the universal follow-up-user-turn form is used instead).

### Integration Points

- `ChatRequest::with_tools(iter)`: Registers available tools for the request.
- `ChatRequest::append_tool_use_from_stream_end(end, response)`: A high-level helper for handling the assistant turn and tool response in iterative loops.
- `ChatMessage::from(Vec<ToolCall>)`: Automatically creates an assistant message containing the tool calls.
- `ChatMessage::from(ToolResponse)`: Automatically creates a tool-role message.

## 4. Internal Implementation

The implementation is partitioned into focused files within `src/chat/tool/` to separate concerns:

- `tool_base.rs`: Defines the `Tool` structure used for request definitions.
- `tool_call.rs`: Handles the data structure for model-generated calls.
- `tool_response.rs`: Handles the data structure for user-provided execution results.

Adapters (e.g., `openai`, `anthropic`, `gemini`) are responsible for the bi-directional translation between these unified types and the specific JSON wire formats required by each provider.

## 5. Usage Example

```rust
// 1. Define the tool
let tool = Tool::new("get_weather")
    .with_description("Get the current weather for a location")
    .with_schema(serde_json::json!({
        "type": "object",
        "properties": {
            "location": { "type": "string", "description": "City and state" }
        },
        "required": ["location"]
    }));

// 2. Add it to the request
let chat_req = ChatRequest::from_user("What is the weather in Paris?")
    .with_tools([tool]);

// 3. Execute chat (omitted client setup)
let response = client.exec_chat(model, chat_req, None).await?;

// 4. Handle tool calls if present
if let Some(tool_call) = response.tool_calls().first() {
    let result = ToolResponse::new(&tool_call.call_id, "Rainy, 15°C");
}
```

For built-in provider tools, the normalized API can also use typed names and configs, for example `Tool::new_web_search()` together with `ToolConfig::WebSearch(...)`.

