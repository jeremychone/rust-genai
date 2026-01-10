# GenAI API Reference for LLMs

Dry, concise reference for the `genai` library.

## Core Concepts

- **Client**: Main entry point (`genai::Client`). Thread-safe (`Arc` wrapper).
- **ModelIden**: `AdapterKind` + `ModelName`. Identifies which provider to use.
- **ServiceTarget**: Resolved `ModelIden`, `Endpoint`, and `AuthData`.
- **Resolvers**: Hooks to customize model mapping, authentication, and service endpoints.
- **AdapterKind**: Supported: `OpenAI`, `OpenAIResp`, `Gemini`, `Anthropic`, `Fireworks`, `Together`, `Groq`, `Mimo`, `Nebius`, `Xai`, `DeepSeek`, `Zai`, `BigModel`, `Cohere`, `Ollama`.

## Client & Configuration

### `Client`
- `Client::default()`: Standard client.
- `Client::builder()`: Returns `ClientBuilder`.
- `exec_chat(model, chat_req, options)`: Returns `ChatResponse`.
- `exec_chat_stream(model, chat_req, options)`: Returns `ChatStreamResponse`.
- `exec_embed(model, embed_req, options)`: Returns `EmbedResponse`.
- `embed(model, text, options)`: Convenience for single text embedding.
- `embed_batch(model, texts, options)`: Convenience for batch embedding.
- `resolve_service_target(model_name)`: Returns `ServiceTarget`.
- `all_model_names(adapter_kind)`: Returns a list of models for a provider (Ollama is dynamic).

### `ClientBuilder`
- `with_auth_resolver(resolver)` / `with_auth_resolver_fn(f)`: Set sync/async auth lookup.
- `with_service_target_resolver(resolver)` / `with_service_target_resolver_fn(f)`: Full control over URL/Headers/Auth per call.
- `with_model_mapper(mapper)` / `with_model_mapper_fn(f)`: Map model names before execution.
- `with_chat_options(options)`: Set client-level default chat options.
- `with_web_config(web_config)`: Configure `reqwest` (timeouts, proxies, default headers).

## Chat Request Structure

### `ChatRequest`
- `system`: Initial system string (optional).
- `messages`: `Vec<ChatMessage>`.
- `tools`: `Vec<Tool>` (optional).
- `from_system(text)`, `from_user(text)`, `from_messages(vec)`: Constructors.
- `append_message(msg)`: Adds a message to the sequence.
- `append_messages(iter)`: Adds multiple messages.
- `append_tool(tool)`: Adds a single tool definition.
- `append_tool_use_from_stream_end(end, tool_response)`: Simplifies tool-use loops by appending the assistant turn (with thoughts/tools) and the tool result.
- `join_systems()`: Concatenates all system content (top-level + system-role messages) into one string.

### `ChatMessage`
- `role`: `System`, `User`, `Assistant`, `Tool`.
- `content`: `MessageContent` (multipart).
- `options`: `MessageOptions` (e.g., `cache_control: Ephemeral` for Anthropic).
- **Constructors**: `ChatMessage::system(text)`, `user(text)`, `assistant(text)`.
- **Tool Handoff**: `assistant_tool_calls_with_thoughts(calls, thoughts)` for continuing tool exchanges where thoughts must precede tool calls.

### `MessageContent` (Multipart)
- Transparent wrapper for `Vec<ContentPart>`.
- **Constructors**: `from_text(text)`, `from_parts(vec)`, `from_tool_calls(vec)`.
- **Methods**: `joined_texts()` (joins with blank line), `first_text()`, `prepend(part)`, `extend_front(parts)`.
- `ContentPart` variants:
    - `Text(String)`: Plain text.
    - `Binary(Binary)`: Images/PDFs/Audio.
    - `ToolCall(ToolCall)`: Model-requested function call.
    - `ToolResponse(ToolResponse)`: Result of function call.
    - `ThoughtSignature(String)`: Reasoning/thoughts (e.g., Gemini/Anthropic).

### `Binary`
- `content_type`: MIME (e.g., `image/jpeg`, `application/pdf`).
- `source`: `Url(String)` or `Base64(Arc<str>)`.
- `from_file(path)`: Reads file and detects MIME.
- `is_image()`, `is_audio()`, `is_pdf()`: Type checks.
- `size()`: Approximate in-memory size in bytes.

## Chat Options & Features

### `ChatOptions`
- `temperature`, `max_tokens`, `top_p`.
- `stop_sequences`: `Vec<String>`.
- `response_format`: `ChatResponseFormat::JsonMode` or `JsonSpec(name, schema)`.
- `reasoning_effort`: `Low`, `Medium`, `High`, `Budget(u32)`, `None`.
- `verbosity`: `Low`, `Medium`, `High` (e.g., for GPT-5).
- `normalize_reasoning_content`: Extract `<think>` blocks into response field.
- `capture_usage`, `capture_content`, `capture_reasoning_content`, `capture_tool_calls`: (Streaming) Accumulate results in `StreamEnd`.
- `seed`: Deterministic generation.
- `service_tier`: `Flex`, `Auto`, `Default` (OpenAI).
- `extra_headers`: `Headers` added to the request.

## Embedding

### `EmbedRequest`
- `input`: `EmbedInput` (Single string or Batch `Vec<String>`).

### `EmbedOptions`
- `dimensions`, `encoding_format` ("float", "base64").
- `user`, `truncate` ("NONE", "START", "END").
- `embedding_type`: Provider specific (e.g., "search_document" for Cohere, "RETRIEVAL_QUERY" for Gemini).

### `EmbedResponse`
- `embeddings`: `Vec<Embedding>` (contains `vector: Vec<f32>`, `index`, `dimensions`).
- `usage`: `Usage`.
- `model_iden`, `provider_model_iden`.

## Tooling

### `Tool`
- `name`, `description`, `schema` (JSON Schema).
- `config`: Optional provider-specific config.

### `ToolCall`
- `call_id`, `fn_name`, `fn_arguments` (JSON `Value`).
- `thought_signatures`: Leading thoughts associated with the call (captured during streaming).

### `ToolResponse`
- `call_id`, `content` (Result as string, usually JSON).

## Responses & Streaming

### `ChatResponse`
- `content`: `MessageContent`.
- `reasoning_content`: Extracted thoughts (if normalized).
- `usage`: `Usage`.
- `model_iden`, `provider_model_iden`.
- `first_text()`, `into_first_text()`, `tool_calls()`.

### `ChatStream`
- Sequence of `ChatStreamEvent`: `Start`, `Chunk(text)`, `ReasoningChunk(text)`, `ThoughtSignatureChunk(text)`, `ToolCallChunk(ToolCall)`, `End(StreamEnd)`.

### `StreamEnd`
- `captured_usage`: `Option<Usage>`.
- `captured_content`: Concatenated `MessageContent` (text, tools, thoughts).
- `captured_reasoning_content`: Concatenated reasoning content.
- `captured_first_text()`, `captured_tool_calls()`, `captured_thought_signatures()`.
- `into_assistant_message_for_tool_use()`: Returns a `ChatMessage` ready for the next request in a tool-use flow.

## Usage & Metadata

### `Usage`
- `prompt_tokens`: Total input tokens.
- `completion_tokens`: Total output tokens.
- `total_tokens`: Sum of input and output.
- `prompt_tokens_details`: `cache_creation_tokens`, `cached_tokens`, `audio_tokens`.
- `completion_tokens_details`: `reasoning_tokens`, `audio_tokens`, `accepted_prediction_tokens`, `rejected_prediction_tokens`.

## Resolvers & Auth

### `AuthData`
- `Key(String)`: The API key.
- `FromEnv(String)`: Env var name to lookup.
- `RequestOverride { url, headers }`: For unorthodox auth or endpoint overrides (e.g., Vertex AI, Bedrock).

### `AuthResolver`
- `from_resolver_fn(f)` / `from_resolver_async_fn(f)`.
- Resolves `AuthData` based on `ModelIden`.

### `ServiceTargetResolver`
- `from_resolver_fn(f)` / `from_resolver_async_fn(f)`.
- Maps `ServiceTarget` (Model, Auth, Endpoint) to a final call target.

### `Headers`
- `merge(overlay)`, `applied_to(target)`.
- Iteration and `From` conversions for `HashMap`, `Vec<(K,V)>`, etc.

## Model Resolution Nuances

- **Auto-detection**: `AdapterKind` inferred from name (e.g., `gpt-` -> `OpenAI`, `claude-` -> `Anthropic`, `gemini-` -> `Gemini`, `command` -> `Cohere`, `grok` -> `Xai`, `glm` -> `Zai`).
- **Namespacing**: `namespace::model_name` (e.g., `together::meta-llama/...`, `nebius::Qwen/...`).
- **Ollama Fallback**: Unrecognized names default to `Ollama` adapter (localhost:11434).
- **Reasoning**: Automatic extraction for DeepSeek/Ollama when `normalize_reasoning_content` is enabled.

## Error Handling

- `genai::Error`: Covers `ChatReqHasNoMessages`, `RequiresApiKey`, `WebModelCall`, `StreamParse`, `AdapterNotSupported`, `Resolver`, etc.
- `Result<T>`: Alias for `core::result::Result<T, genai::Error>`.
- `size()`: Many types implement `.size()` for approximate memory tracking.
