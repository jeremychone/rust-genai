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

- `Tool`: Represents a tool definition (metadata and parameters).
    - Properties: `name`, `description`, `schema`, `config`.
    - `Tool::new(name)`: Primary constructor.
    - `with_schema(value)`: Builder-style method to set the JSON parameters schema.
    - `with_description(text)`: Builder-style method to set the tool description.
- `ToolCall`: Represents an invocation request emitted by the model.
    - Properties: `call_id`, `fn_name`, `fn_arguments`, `thought_signatures`.
- `ToolResponse`: Represents the result of a tool execution.
    - Properties: `call_id`, `content`.
    - `ToolResponse::new(call_id, content)`: Links the execution output back to the original call.

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
