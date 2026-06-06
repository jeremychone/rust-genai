# genai - API Reference for LLMs

Comprehensive, dry reference for the `genai` Rust library.
All public types, methods, and module structure are documented below.

```toml
genai = "0.6"
```

## Crate Structure

```text
genai (crate root / lib.rs)
├── pub mod adapter     -- AdapterKind enum, adapter dispatch
├── pub mod chat        -- ChatRequest, ChatResponse, ChatStream, ChatOptions, Tools, ...
│   └── pub mod printer -- print_chat_stream utility
├── pub mod embed       -- EmbedRequest, EmbedResponse, EmbedOptions
├── pub mod resolver    -- AuthData, AuthResolver, Endpoint, ProviderConfig, ModelMapper, ServiceTargetResolver, Headers
├── pub mod webc        -- webc::Error (public), WebClient internals (crate-private)
├── Client, ClientBuilder, ClientConfig  (from client module, flattened)
├── ModelIden, ModelName                 (from common module, flattened)
├── ModelSpec, ServiceTarget, Headers, WebConfig (from client module, flattened)
├── Error, Result, BoxError              (from error module, flattened)
```

## Core Concepts

- **Client**: Main entry point (`genai::Client`). Thread-safe (`Arc` inner).
- **ModelIden**: `AdapterKind` + `ModelName`. Identifies which provider and model to use.
- **ModelSpec**: Specifies a model at three resolution levels: `Name`, `Iden`, or `Target`.
- **ServiceTarget**: Fully resolved call target: `ModelIden` + `Endpoint` + `AuthData`.
- **ProviderConfig**: Provider-level endpoint/auth overrides for adapter-wide operations such as model listing. Since v0.6.0.
- **Resolvers**: User hooks to customize model mapping, authentication, and service endpoints.
- **Bound Adapter**: Optional `Client`/`ClientBuilder` adapter constraint. Since v0.6.0. When set, bare model names route through the bound adapter instead of heuristic inference. Explicit mismatched namespaces or mismatched `ModelIden` values return `AdapterKindMismatch`.
- **AdapterKind**: Supported providers (updated in v0.6.x; custom added in v0.7.0-beta): `openai`, `openai_resp`, `anthropic`, `gemini`, `ollama`, `ollama_cloud`, `vertex`, `bedrock_api`, `bedrock_sigv4`, `github_copilot`, `opencode_go`, `groq`, `together`, `fireworks`,  `cohere`, `nebius`, `mimo`, `deepseek`, `minimax`, `zai`, `zai_coding`, `bigmodel`, `aliyun`, `baidu`, `moonshot`, `aihubmix`, `open_router`, `custom`, `xai`.
  - `github_copilot` (since v0.6.0) is a GitHub Models gateway with multi-publisher namespaced models such as `github_copilot::openai/gpt-4.1-mini`, `github_copilot::anthropic/claude-sonnet-4-6`, and `github_copilot::google/gemini-2.5-pro`.
  - `ollama_cloud` (since v0.6.0) is the hosted Ollama Cloud service (`ollama.com`). It uses the same native Ollama protocol as the local `ollama` adapter but authenticates with `Authorization: Bearer $OLLAMA_API_KEY`. Use via the `ollama_cloud::model_name` namespace, for example, `ollama_cloud::gemma3:4b`.
  - `vertex` (since v0.6.0) is Google Vertex AI Model Garden routing for Gemini and Anthropic models via the `vertex::` namespace.
  - `opencode_go` (since v0.6.0) is a curated open coding model proxy with dual-protocol routing via the `opencode_go::` namespace. Uses `OPENCODE_GO_API_KEY`.
  - `bedrock_api` (since v0.6.0) is AWS Bedrock Converse API, authenticated with a simple Bearer token from `BEDROCK_API_KEY`. Use via the `bedrock_api::` namespace.
  - `bedrock_sigv4` (since v0.6.0) is AWS Bedrock Converse API, authenticated via SigV4 + standard AWS credential chain. Requires the `bedrock-sigv4` feature. Use via the `bedrock_sigv4::` namespace.
  - `open_router` (since v0.6.0) is OpenRouter OpenAI-compatible gateway. Uses `OPEN_ROUTER_API_KEY`. Use via the `open_router::` namespace.
  - `baidu` (since v0.6.0) is Baidu's OpenAI/Anthropic compatible proxies.
  - `aliyun` (since v0.6.0) is Aliyun's namespace-only OpenAI-compatible service.
  - `moonshot` (since v0.6.0) is Moonshot AI adapter.
  - `aihubmix` (since v0.6.0) is AIHubMix adapter.

## Client & Configuration

### `Client`

Construct via `Client::default()` or `Client::builder().build()`.

- `Client::default()`: Standard client.
- `Client::builder()`: Returns `ClientBuilder`.
- `adapter_kind()`: Returns `Option<AdapterKind>` when this client is bound to a single adapter. Since v0.6.0.
- `exec_chat(model, chat_req, options)`: `model: impl Into<ModelSpec>`, `chat_req: ChatRequest`, `options: Option<&ChatOptions>` -> `Result<ChatResponse>`. Updated in v0.6.0 to accept `impl Into<ModelSpec>`.
- `exec_chat_stream(model, chat_req, options)`: Same signature pattern -> `Result<ChatStreamResponse>`. Updated in v0.6.0 to accept `impl Into<ModelSpec>`.
- `exec_embed(model, embed_req, options)`: `model: impl Into<ModelSpec>`, `embed_req: EmbedRequest`, `options: Option<&EmbedOptions>` -> `Result<EmbedResponse>`. Updated in v0.6.0 to accept `impl Into<ModelSpec>`.
- `embed(model, input, options)`: Convenience; wraps a single `impl Into<String>` into `EmbedRequest`. Updated in v0.6.0 to accept `impl Into<ModelSpec>`.
- `embed_batch(model, inputs, options)`: Convenience; wraps `Vec<String>` into `EmbedRequest`. Updated in v0.6.0 to accept `impl Into<ModelSpec>`.
- `resolve_service_target(model)`: Returns `ServiceTarget`. Updated in v0.6.0 to accept `impl Into<ModelSpec>`.
- `all_model_names(adapter_kind, provider_config)`: `provider_config: impl Into<resolver::ProviderConfig>` -> `Result<Vec<String>>`. Static list for most adapters; Ollama queries configured endpoint. Updated in v0.6.0 to take the second provider config argument so adapter-wide model listing can override endpoint and auth without using `ServiceTarget`.
  - Defaults: `client.all_model_names(AdapterKind::Ollama, None).await?`.
  - Endpoint override only: `client.all_model_names(AdapterKind::Ollama, Endpoint::from_static("http://remote-host:11434/")).await?`.
  - Auth override only: `client.all_model_names(AdapterKind::OpenAI, AuthData::from_env("OPENAI_API_KEY")).await?`.
- `default_model(model_name)`: `Result<ModelIden>`. Infers `AdapterKind` from model name string.

### `ClientBuilder`

- `with_auth_resolver(resolver)` / `with_auth_resolver_fn(f)`: Set sync/async auth lookup.
- `with_service_target_resolver(resolver)` / `with_service_target_resolver_fn(f)`: Full control over URL/Headers/Auth per call.
- `with_model_mapper(mapper)` / `with_model_mapper_fn(f)`: Map model names before execution.
- `with_adapter_kind(adapter_kind)`: Bind the client to one adapter. Since v0.6.0. Bare model names route through this adapter. Namespaced models or `ModelIden` values for a different adapter return `AdapterKindMismatch`.
- `with_chat_options(options)`: Set client-level default chat options.
- `with_web_config(web_config)`: Configure `reqwest` (timeouts, proxies, default headers).
- `with_reqwest(reqwest_client)`: Use a custom `reqwest::Client` directly.
- `with_config(config)`: Set a `ClientConfig` directly.
- `build()`: Consumes the builder and returns a `Client`.

### `ClientConfig`

Configuration container. Used internally by `Client`; can also be constructed directly.

- `with_auth_resolver(resolver)`: Sets the `AuthResolver`.
- `with_model_mapper(mapper)`: Sets the `ModelMapper`.
- `with_service_target_resolver(resolver)`: Sets the `ServiceTargetResolver`.
- `with_adapter_kind(adapter_kind)`: Sets the optional bound adapter for this client config. Since v0.6.0.
- `with_chat_options(options)`: Sets default `ChatOptions`.
- `with_embed_options(options)`: Sets default `EmbedOptions`.
- `with_web_config(web_config)`: Sets `WebConfig`.
- Getters: `auth_resolver()`, `service_target_resolver()`, `model_mapper()`, `adapter_kind()` (since v0.6.0), `chat_options()`, `embed_options()`, `web_config()`.

### `WebConfig`

HTTP client configuration applied to the internal `reqwest::Client`.

- `timeout`, `connect_timeout`, `read_timeout`: `Option<Duration>`.
- `default_headers`: `Option<reqwest::header::HeaderMap>`.
- `proxy`: `Option<reqwest::Proxy>`.
- Chainable setters: `with_timeout(d)`, `with_connect_timeout(d)`, `with_read_timeout(d)`, `with_default_headers(h)`, `with_proxy(p)`, `with_proxy_url(url)`, `with_https_proxy_url(url)`, `with_all_proxy_url(url)`.

### `ModelSpec`

Specifies how to identify and resolve a model for API calls. Since v0.6.0. All `exec_chat`, `exec_chat_stream`, `exec_embed`, etc. accept `impl Into<ModelSpec>`.

Variants:
- `ModelSpec::Name(ModelName)`: Just a model name string. Since v0.6.0. Adapter kind inferred, full resolution (mapper, auth, target resolver). With a bound adapter client, bare names route through the bound adapter.
- `ModelSpec::Iden(ModelIden)`: Explicit adapter kind. Since v0.6.0. Skips adapter inference, still resolves auth/endpoint. With a bound adapter client, the `ModelIden.adapter_kind` must match the bound adapter.
- `ModelSpec::Target(ServiceTarget)`: Complete target. Since v0.6.0. Only runs service target resolver.

Constructors, all since v0.6.0:

- `ModelSpec::from_name(name)`: Creates `ModelSpec::Name`.
- `ModelSpec::from_static_name(name)`: Creates `ModelSpec::Name` from a static string.
- `ModelSpec::from_iden(model_iden)`: Creates `ModelSpec::Iden`.
- `ModelSpec::from_target(target)`: Creates `ModelSpec::Target`.

`Into<ModelSpec>` is implemented for: `&str`, `&&str`, `String`, `&String`, `ModelName`, `&ModelName`, `ModelIden`, `&ModelIden`, `ServiceTarget`.

### `ModelName`

Efficient, clonable model name storage (`Arc<str>` or `&'static str` internally).

- `ModelName::new(name)`: From `impl Into<Arc<str>>`.
- `ModelName::from_static(name)`: From `&'static str`.
- `as_str()`: Returns `&str`.
- `namespace()`: Returns `Option<&str>` (e.g., `"openai"` from `"openai::gpt-4"`).
- `namespace_and_name()`: Returns `(Option<&str>, &str)`.
- `namespace_is(ns)`: Bool check.
- Implements `Deref<Target=str>`, `PartialEq<str>`, `PartialEq<&str>`, `PartialEq<String>`, `Display`, `From<&str>`, `From<String>`.

### `ModelIden`

Holds `adapter_kind: AdapterKind` + `model_name: ModelName`.

- `ModelIden::new(adapter_kind, model_name)`: Generic constructors.
- `ModelIden::from_static(adapter_kind, name)`: Static string variant.
- `from_name(new_name)`: Creates new `ModelIden` with same adapter, different name (clones if unchanged).
- `from_optional_name(Option<String>)`: Same as above but with optional name.
- Implements `Display` as `"{model_name} (adapter: {adapter_kind})"`, `Clone`, `Eq`, `Hash`, `Serialize`, `Deserialize`.
- `From<(AdapterKind, T)>` where `T: Into<ModelName>`.

### `ServiceTarget`

Fully resolved call target. Since v0.6.0, a complete `ServiceTarget` can be passed through `ModelSpec::Target` to bypass model mapping and auth resolution while still running the service target resolver.

- `endpoint: Endpoint`
- `auth: AuthData`
- `model: ModelIden`

### `ProviderConfig`

Provider-level endpoint/auth override for adapter-wide operations such as `Client::all_model_names`. Since v0.6.0.

- `endpoint: Option<Endpoint>`: `None` uses the adapter default endpoint. Since v0.6.0.
- `auth: Option<AuthData>`: `None` uses the adapter default auth resolution. `Some(AuthData::None)` explicitly uses no auth. Since v0.6.0.
- `ProviderConfig::default()`: Uses adapter defaults for both endpoint and auth.
- `ProviderConfig::from_endpoint(endpoint)`: Override endpoint only.
- `ProviderConfig::from_auth(auth)`: Override auth only.
- `with_endpoint(endpoint)`: Chainable endpoint override.
- `with_auth(auth)`: Chainable auth override.
- `From<()>`: Lightweight default value.
- `From<Option<ProviderConfig>>`: Enables `client.all_model_names(kind, None).await?`.
- `From<Endpoint>`: Endpoint-only override.
- `From<AuthData>`: Auth-only override.
- `From<(Endpoint, AuthData)>`: Override both endpoint and auth.
- From<(AuthData, Endpoint)>: Override both auth and endpoint in reverse tuple order.
- `From<(Option<Endpoint>, Option<AuthData>)>`: Partial tuple override.
- From<(Option<AuthData>, Option<Endpoint>)>: Partial tuple override in reverse tuple order.

Examples:

```rust
use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint, ProviderConfig};

let default_models = client.all_model_names(AdapterKind::Ollama, None).await?;

let remote_ollama_models = client
	.all_model_names(
		AdapterKind::Ollama,
		ProviderConfig::default()
			.with_endpoint(Endpoint::from_static("http://remote-host:11434/"))
			.with_auth(AuthData::None),
	)
	.await?;
```

## Chat Request Structure

### `ChatRequest`

- `system`: Initial system string (optional).
- `messages`: `Vec<ChatMessage>`.
- `tools`: `Vec<Tool>` (optional).
- `previous_response_id`: Previous response ID for stateful sessions, OpenAI Responses API. Since v0.6.0.
- `store`: Whether to store the response for later stateful continuation, OpenAI Responses API. Since v0.6.0.
- **Constructors**: `new(messages)`, `from_system(text)`, `from_user(text)`, `from_messages(vec)`.
- `with_system(text)`: Sets/replaces system prompt (chainable).
- `append_message(msg)`: Adds a message to the sequence.
- `append_messages(iter)`: Adds multiple messages.
- `with_tools(iter)`: Replaces the tool set.
- `append_tool(tool)`: Adds a single tool definition.
- `with_previous_response_id(id)`: Sets the previous response ID for stateful sessions. Since v0.6.0.
- `with_store(bool)`: Enables or disables response storage for stateful sessions. Since v0.6.0.
- `append_tool_use_from_stream_end(end, tool_response)`: Simplifies tool-use loops by appending the assistant turn (with thoughts/tools) and the tool result.
- `iter_systems()`: Iterator over all system content (top-level + system-role messages).
- `join_systems()`: Concatenates all system content into one string with blank line separators.

Stateful OpenAI Responses session support is available through `previous_response_id` and `store`. Since v0.6.0.

### `ChatMessage`

- `role`: `System`, `User`, `Assistant`, `Tool`.
- `content`: `MessageContent` (multipart).
- `options`: `Option<MessageOptions>`.
- **Constructors**: `ChatMessage::new(role, content)`, `system(text)`, `user(text)`, `assistant(text)`, `tool(text)`.
- `with_options(options)`: Attaches `MessageOptions` (chainable).
- `with_reasoning_content(reasoning: Option<String>)`: Appends `ContentPart::ReasoningContent` when provided. Since v0.6.0.
- `assistant_tool_calls_with_thoughts(calls, thoughts)`: For continuing tool exchanges where thoughts must precede tool calls. Since v0.6.0.
- `size()`: Approximate in-memory size in bytes.
- `From<Vec<ToolCall>>`: Creates assistant message with tool calls (auto-prepends thoughts if present on first call). Updated in v0.6.0 for thought-signature tool handoff support.
- `From<ToolResponse>`: Creates tool-role message.
- `From<Vec<ToolResponse>>`: Creates a tool-role multipart message.

### `ChatRole`

- Variants: `System`, `User`, `Assistant`, `Tool`.
- Implements `Display`, `PartialEq`, `Eq`, `Clone`, `Serialize`, `Deserialize`.

### `MessageOptions`

Per-message options.

- `cache_control: Option<CacheControl>`.
- `with_cache_control(cache_control)`: Sets the cache control hint.
- `From<CacheControl>`: Convenience conversion.

### `CacheControl`

Unified cache policy abstraction.

- `Ephemeral`: Default 5-minute TTL.
- `Memory`: Memory-oriented cache mode. On some providers this may be used as the request-level memory cache setting.
- `Ephemeral5m`: Explicit 5-minute TTL.
- `Ephemeral1h`: Extended 1-hour TTL (must appear before shorter TTLs in request order).
- `Ephemeral24h`: Extended 24-hour TTL.

On `ChatOptions`, this is a request-level cache preference. Updated in v0.6.0 for chat-level prompt cache control.

On `MessageOptions`, this is a per-message or per-content cache hint.

Different providers support different variants and scopes.

### `MessageContent` (Multipart)

- Transparent wrapper for `Vec<ContentPart>`.
- **Constructors**: `from_text(text)`, `from_parts(vec)`, `from_tool_calls(vec)`, `from_tool_responses(vec)`.
- **Mutation**: `push(part)`, `insert(idx, part)`, `prepend(part)`, `extend_front(parts)`, `append(part)` (chainable), `extended(iter)` (chainable).
- **Getters**: `parts()`, `into_parts()`, `texts()`, `into_texts()`, `binaries()`, `into_binaries()`, `thought_signatures()`, `into_thought_signatures()`, `tool_calls()`, `into_tool_calls()`, `tool_responses()`, `into_tool_responses()`, `custom_parts()`, `into_custom_parts()`.
- **Convenient**: `first_text()`, `into_first_text()`, `first_thought_signature()`, `into_first_thought_signature()`, `joined_texts()` (joins with blank line), `into_joined_texts()`.
- **Queries**: `is_empty()`, `len()`, `is_text_empty()`, `is_text_only()`, `contains_text()`, `contains_binary()`, `contains_tool_call()`, `contains_tool_response()`, `contains_thought_signature()`, `contains_custom()`.
- **Reasoning helpers**: `reasoning_contents()`, `into_reasoning_contents()`, `first_reasoning_content()`, `into_first_reasoning_content()`, `joined_reasoning_content()`, `contains_reasoning_content()`. Since v0.6.0.
- `size()`: Approximate in-memory size.
- Implements `IntoIterator`, `IntoIterator for &MessageContent`, `IntoIterator for &mut MessageContent`, `FromIterator<ContentPart>`, `Extend<ContentPart>`.
- `From<&str>`, `From<&String>`, `From<String>`, `From<Vec<ToolCall>>`, `From<ToolCall>`, `From<ToolResponse>`, `From<Vec<ToolResponse>>`, `From<ContentPart>`, `From<Binary>`, `From<CustomPart>`, `From<Vec<ContentPart>>`.

### `ContentPart`

A single content segment in a chat message.

- `Text(String)`: Plain text. `From<String>`, `From<&String>`, `From<&str>`.
- `Binary(Binary)`: Images/PDFs/Audio. `From<Binary>`.
- `ToolCall(ToolCall)`: Model-requested function call. `From<ToolCall>`.
- `ToolResponse(ToolResponse)`: Result of function call. `From<ToolResponse>`.
- `ThoughtSignature(String)`: Thought-signature metadata (for providers that emit signed thoughts). Not auto-from; use constructor.
- `ReasoningContent(String)`: Reasoning text content, distinct from thought signatures. Since v0.6.0.
- `Custom(CustomPart)`: Provider-specific custom content with optional `model_iden`. Updated in v0.6.0: `CustomPart.model_iden` is optional and is populated for response-originated custom parts when provider/model identity is available.
- **Constructors**: `from_text(text)`, `from_binary_base64(content_type, content, name)`, `from_binary_url(content_type, url, name)`, `from_binary_file(path)`, `from_custom(data, model_iden)`.
- **Accessors**: `as_text()`, `into_text()`, `as_tool_call()`, `into_tool_call()`, `as_tool_response()`, `into_tool_response()`, `as_binary()`, `into_binary()`, `as_thought_signature()`, `into_thought_signature()`, `as_reasoning_content()`, `into_reasoning_content()` (reasoning accessors since v0.6.0), `as_custom()`, `into_custom()`.
- **Queries**: `is_text()`, `is_binary()`, `is_image()`, `is_audio()`, `is_pdf()`, `is_tool_call()`, `is_tool_response()`, `is_thought_signature()`, `is_reasoning_content()` (`is_reasoning_content()` since v0.6.0), `is_custom()`.
- `size()`: Approximate in-memory size.

### `CustomPart`

Provider-specific custom content payload.

- `model_iden: Option<ModelIden>`: Provider/model identity for the part when available. Updated in v0.6.0 to be optional.
- `data: serde_json::Value`: Raw provider JSON payload.
- `data()`: Returns the raw JSON payload.
- `adapter_kind()`: Returns the adapter kind from `model_iden`, if available.
- `typ()`: Returns the string value of the raw JSON `"type"` field, if present.

### `Binary`

- `content_type`: MIME (e.g., `image/jpeg`, `application/pdf`).
- `source`: `BinarySource::Url(String)` or `BinarySource::Base64(Arc<str>)`.
- `name`: `Option<String>` (display name or filename).
- **Constructors**: `new(content_type, source, name)`, `from_base64(content_type, content, name)`, `from_url(content_type, url, name)`.
- `from_file(path)`: Reads file and detects MIME.
- `is_image()`, `is_audio()`, `is_pdf()`: Type checks.
- `into_url()`: Generates data URL (for base64) or returns the URL.
- `size()`: Approximate in-memory size in bytes.

## Chat Options & Features

### `ChatOptions`

All fields are `Option<T>` (unset = defer to client default or provider default).

- `temperature`, `max_tokens`, `top_p`.
- `stop_sequences`: `Vec<String>`.
- `response_format`: `ChatResponseFormat::JsonMode` or `JsonSpec(name, schema)`.
- `tool_choice`: Provider-neutral `ToolChoice` hint for how tool calls should be handled. Since v0.6.0.
- `reasoning_effort`: `ReasoningEffort` enum.
- `verbosity`: `Verbosity` enum (e.g., for GPT-5).
- `normalize_reasoning_content`: Extract `<think>` blocks into response field.
- `capture_usage`, `capture_content`, `capture_reasoning_content`, `capture_tool_calls`: (Streaming) Accumulate results in `StreamEnd`.
- `capture_raw_body`: Capture raw HTTP response body.
- `seed`: Deterministic generation.
- `service_tier`: `Flex`, `Auto`, `Default` (OpenAI).
- `prompt_cache_key`: OpenAI prompt cache key. Since v0.6.0.
- `cache_control`: `CacheControl` request-level cache preference. Updated in v0.6.0 for chat-level prompt cache control.
- `extra_headers`: `Headers` added to the request.
- `extra_body`: `serde_json::Value` merged into supported provider request bodies as a low-level escape hatch for provider-specific fields. Since v0.6.0. Currently used by OpenAI-compatible chat payloads, including Chat Completions and Responses.
- **Chainable setters**: `with_temperature(f64)`, `with_max_tokens(u32)`, `with_top_p(f64)`, `with_capture_usage(bool)`, `with_capture_content(bool)`, `with_capture_reasoning_content(bool)`, `with_capture_tool_calls(bool)`, `with_capture_raw_body(bool)`, `with_stop_sequences(vec)`, `with_normalize_reasoning_content(bool)`, `with_response_format(format)`, `with_tool_choice(choice)` (since v0.6.0), `with_reasoning_effort(effort)`, `with_verbosity(v)`, `with_seed(u64)`, `with_service_tier(tier)`, `with_prompt_cache_key(key)` (since v0.6.0), `with_cache_control(cache_control)` (updated in v0.6.0 for chat-level prompt cache control), `with_extra_headers(headers)`, `with_extra_body(value)` (since v0.6.0).
- Deprecated: `with_json_mode(bool)` in favor of `with_response_format(ChatResponseFormat::JsonMode)`.

### `ChatResponseFormat`

- `JsonMode`: Request JSON-mode output.
- `JsonSpec(JsonSpec)`: Structured output with schema.

### `JsonSpec`

Updated in v0.6.0.

- `name: String`: Spec name (OpenAI: only `-` and `_` allowed).
- `description: Option<String>`: Human-readable description.
- `schema: serde_json::Value`: Simplified JSON schema.
  - **Gemini & Vertex Normalization**: Updated in v0.6.0. `genai` normalizes common JSON Schema shapes (such as those generated by `schemars`) for provider compatibility, including:
    - Converting `const` to a single-value `enum`.
    - Preserving and sanitizing `additionalProperties`.
    - Stripping JSON Schema-only keywords rejected by Vertex.
- `JsonSpec::new(name, schema)`: Constructor.
- `with_description(desc)`: Chainable setter.

### `ReasoningEffort`

Provider-specific hint for reasoning intensity/budget.

- Variants: `None`, `Low`, `Medium`, `High`, `XHigh` (since v0.6.0), `Max` (since v0.6.0), `Budget(u32)`, `Minimal` (legacy, for <= gpt-5).
- `variant_name()`: Returns lowercase name (`"none"`, `"low"`, `"medium"`, `"high"`, `"xhigh"`, `"max"`, `"budget"`, `"minimal"`).
- `as_keyword()`: Returns `Option<&'static str>` (None for `Budget`).
- `from_keyword(name)`: Parses keyword string.
- `from_model_name(model_name)`: If model name ends with `-<effort>`, returns `(Some(effort), trimmed_name)`.
- Implements `Display`, `FromStr` (parses keywords and numeric budgets).

### `Verbosity`

Provider-specific verbosity hint.

- Variants: `Low`, `Medium`, `High`.
- `variant_name()`, `as_keyword()`, `from_keyword(name)`, `from_model_name(model_name)`.
- Implements `Display`, `FromStr`.

### `ServiceTier`

OpenAI service tier preference for flex processing.

- Variants: `Flex`, `Auto`, `Default`.
- `variant_name()`, `as_keyword()`, `from_keyword(name)`.
- Implements `Display`, `FromStr`.

## Embedding

### `EmbedRequest`

- `input`: `EmbedInput`.
- **Constructors**: `new(input)` (single), `new_batch(inputs)` (batch), `from_text(text)`, `from_texts(texts)`.
- **Getters**: `single_input()`, `inputs()` (always returns `Vec<&str>`), `is_batch()`, `input_count()`.

### `EmbedInput`

- `Single(String)`: One text input.
- `Batch(Vec<String>)`: Multiple text inputs.
- `From<String>`, `From<&str>`, `From<Vec<String>>`, `From<Vec<&str>>`.

### `EmbedOptions`

- `headers`: `Option<Headers>`.
- `capture_raw_body`: `Option<bool>`.
- `capture_usage`: `Option<bool>`.
- `dimensions`: `Option<usize>`.
- `encoding_format`: `Option<String>` ("float", "base64").
- `user`: `Option<String>`.
- `embedding_type`: `Option<String>`. Provider-specific (Cohere: "search_document", "search_query"; Gemini: "SEMANTIC_SIMILARITY", "RETRIEVAL_QUERY", "RETRIEVAL_DOCUMENT").
- `truncate`: `Option<String>` ("NONE", "START", "END"; Cohere).
- **Chainable setters**: `with_headers(h)`, `with_capture_raw_body(b)`, `with_capture_usage(b)`, `with_dimensions(n)`, `with_encoding_format(f)`, `with_user(u)`, `with_embedding_type(t)`, `with_truncate(t)`.

### `EmbedResponse`

- `embeddings`: `Vec<Embedding>` (contains `vector: Vec<f32>`, `index`, `dimensions`).
- `usage`: `Usage`.
- `model_iden`, `provider_model_iden`.
- `captured_raw_body`: `Option<serde_json::Value>`.
- **Getters**: `first_embedding()`, `first_vector()`, `vectors()`, `into_vectors()`, `is_single()`, `is_batch()`, `embedding_count()`.

### `Embedding`

- `vector: Vec<f32>`, `index: usize`, `dimensions: usize`.
- **Constructors**: `new(vector, index)` (dimensions auto-computed), `with_dimensions(vector, index, dimensions)`.
- **Getters**: `vector()`, `into_vector()`, `index()`, `dimensions()`.

## Tooling

### `Tool`

- `name: ToolName`, `description: Option<String>`, `schema: Option<Value>` (JSON Schema), `strict: Option<bool>`, `config: Option<ToolConfig>`. `config` is available since v0.6.0.
- `Tool::new(name)`: Constructor.
- `Tool::new_web_search()`: Constructor for the built-in web search tool. Since v0.6.0.
- `with_description(desc)`, `with_schema(parameters)`, `with_strict(bool)`, `with_config(config)`: Chainable setters. `with_config` is available since v0.6.0.
- `size()`: Approximate in-memory size.
- `config`: Optional provider-specific config.
- Gemini and Vertex Gemini tool schema conversion applies the same compatibility normalization used for structured JSON schemas. Updated in v0.6.0.
- Use [`ToolChoice`](#toolchoice) to control tool selection behavior.

### `ToolChoice`

Provider-neutral tool selection hint for chat requests. Set on `ChatOptions` with `with_tool_choice(choice)`.

Since v0.6.0.

- `Auto`: Let the model decide whether to call tools.
- `None`: Prevent tool calls.
- `Required`: Require the model to call at least one available tool.
- `Tool { name: String }`: Require the model to call a specific tool.
- `ToolChoice::tool(name)`: Convenience constructor for the `Tool` variant.
- Maps to Gemini, OpenAI Chat Completions, OpenAI Responses, and Anthropic request payloads.
- Provider support and exact behavior can differ by adapter and model.
- Useful for requesting automatic tool use, disabling tool use, requiring tool use, or selecting a specific tool when supported by the provider.

### `ToolName`

Normalized tool identifiers. Since v0.6.0.

- `WebSearch`: Built-in provider web-search tool. Since v0.6.0.
- `Custom(String)`: User-defined tool name. Since v0.6.0.
- `as_str()`: Returns the normalized display name.
- Implements `Display`, `AsRef<str>`, `From<String>`, `From<&String>`, `From<&str>`.
- Serialization:
  - `Custom("get_weather")` -> `"get_weather"`
  - `WebSearch` -> `{"WebSearch": null}`

### `ToolConfig`

Configuration variant for tools. Since v0.6.0.

- `WebSearch(WebSearchConfig)`: Typed config for the built-in web-search tool. Since v0.6.0.
- `Custom(serde_json::Value)`: Arbitrary JSON config for custom tools. Since v0.6.0.
- Serialization:
  - `Custom(json)` -> raw JSON value
  - `WebSearch(config)` -> `{"WebSearch": {...}}`

### `WebSearchConfig`

Typed configuration for the built-in web-search tool. Since v0.6.0.

- `max_uses: Option<u32>`: Maximum web-search uses when supported by the provider.
- `allowed_domains: Option<Vec<String>>`: Restrict search to specific domains when supported.
- `blocked_domains: Option<Vec<String>>`: Block specific domains when supported.

Use through `Tool::new_web_search().with_config(WebSearchConfig { ... })` or `ToolConfig::WebSearch(config)`.

### `ToolCall`

- `call_id: String`, `fn_name: String`, `fn_arguments: serde_json::Value`.
- `thought_signatures`: Leading thoughts associated with the call (captured during streaming). Updated in v0.6.0 for thought-signature tool handoff.
- `size()`: Approximate in-memory size.

### `ToolResponse`

- `call_id: String`, `content: String` (result as string, usually JSON).
- `ToolResponse::new(call_id, content)`: Constructor.
- `size()`: Approximate in-memory size.

## Responses & Streaming

### `StopReason`

Provider-agnostic normalized stop reason used by `ChatResponse` and streaming `StreamEnd`.

- Variants: `Completed(String)`, `MaxTokens(String)`, `ToolCall(String)`, `ContentFilter(String)`, `StopSequence(String)`, `Other(String)`.
- `From<String>` maps known provider strings such as `stop`, `length`, `tool_calls`, `content_filter`, and `stop_sequence` into normalized variants while preserving the raw provider string.
- `raw()`: Returns the original provider-specific string.
- `is_max_tokens()`: Returns true when generation was truncated by a token limit.
- Equality compares by variant only and ignores the raw provider string.

### `ChatResponse`

- `content`: `MessageContent`.
- `reasoning_content`: Extracted thoughts (if normalized). Updated in v0.6.0 to align with `ContentPart::ReasoningContent` round-tripping helpers.
- `model_iden`: Resolved `ModelIden` (may differ from requested after mapping).
- `provider_model_iden`: Provider-reported `ModelIden` (may differ from `model_iden`).
- `stop_reason`: `Option<StopReason>` with the normalized provider stop reason.
- `usage`: `Usage`.
- `captured_raw_body`: `Option<serde_json::Value>` (populated when `ChatOptions.capture_raw_body` is true).
- `response_id`: `Option<String>` for stateful OpenAI Responses continuation. Since v0.6.0.
- **Getters**: `first_text()`, `into_first_text()`, `texts()`, `into_texts()`, `tool_calls()`, `into_tool_calls()`, `stop_reason()`, `response_id()`, `usage()`, `model_iden()`, `provider_model_iden()`.

### `ChatStreamResponse`

- `stream: ChatStream`: The stream to iterate.
- `model_iden: ModelIden`: Model identifier for this request.

### `ChatStream`

Implements `Stream<Item = Result<ChatStreamEvent>>`.

### `ChatStreamEvent`

- `Start`: Emitted once at the start.
- `Chunk(StreamChunk)`: Assistant text content chunk. `StreamChunk { content: String }`.
- `ReasoningChunk(StreamChunk)`: Reasoning content chunk.
- `ThoughtSignatureChunk(StreamChunk)`: Thought signature chunk.
- `ToolCallChunk(ToolChunk)`: Tool-call chunk. `ToolChunk { tool_call: ToolCall }`.
- `End(StreamEnd)`: End of stream with captured data.

### `StreamEnd`

- `captured_usage`: `Option<Usage>`.
- `captured_stop_reason`: `Option<StopReason>`. Since v0.6.0.
- `captured_content`: `Option<MessageContent>` (text, tools, thoughts; ordering: ThoughtSignature -> Text -> ToolCall).
- `captured_reasoning_content`: Concatenated reasoning content when `ChatOptions.capture_reasoning_content` is enabled. Updated in v0.6.0 for assistant message handoff via `into_assistant_message_for_tool_use()`.
- `captured_response_id`: Response ID for stateful OpenAI Responses continuation. Since v0.6.0.
- **Getters**: `captured_first_text()`, `captured_into_first_text()`, `captured_texts()`, `into_texts()`, `captured_tool_calls()`, `captured_into_tool_calls()`, `captured_thought_signatures()`, `captured_into_thought_signatures()`, `captured_content()`, `captured_usage()`, `captured_stop_reason()`, `captured_response_id()`.
- `into_assistant_message_for_tool_use()`: Returns a `ChatMessage` ready for the next request in a tool-use flow, preserving thought-signature ordering and attaching reasoning via `with_reasoning_content(...)` when present. Since v0.6.0.

## Printer Utility

Module: `genai::chat::printer`.

- `print_chat_stream(chat_res, options)`: Writes streamed response to stdout, returns concatenated content as `String`.
- `PrintChatStreamOptions`:
  - `from_print_events(bool)`: When true, prints event markers and tool-call metadata.
- Has its own `printer::Error` type (wraps `tokio::io::Error` and `genai::Error`).

## Usage & Metadata

### `Usage`

- `prompt_tokens`: Total input tokens.
- `completion_tokens`: Total output tokens.
- `total_tokens`: Sum of input and output.
- `prompt_tokens_details`: `cache_creation_tokens`, `cached_tokens`, `audio_tokens`.
    - `cache_creation_details`: `ephemeral_5m_tokens`, `ephemeral_1h_tokens`.
- `completion_tokens_details`: `reasoning_tokens`, `audio_tokens`, `accepted_prediction_tokens`, `rejected_prediction_tokens`.
- `compact_details()`: Removes detail objects that contain only `None` fields.
- Updated in v0.6.0: prompt cache usage includes TTL-specific `cache_creation_details` when providers return 5-minute or 1-hour cache creation breakdowns.

Note: All token fields are `Option<i32>`. Zero values from providers are deserialized as `None`.

## Resolvers & Auth

Resolution order is `ModelMapper` -> `AuthResolver` -> adapter default endpoint -> `ServiceTargetResolver`. This matters for bound adapter clients and proxy/gateway configurations, because auth is resolved before the service target resolver gets the final override opportunity.

### `AuthData`

- `None`: No authentication data. Used for unauthenticated targets or resolver-controlled flows.
- `FromEnv(String)`: Env var name to lookup.
- `Key(String)`: The API key directly.
- `RequestOverride { url, headers }`: For unorthodox auth or endpoint overrides (e.g., Vertex AI, Bedrock).
- `MultiKeys(HashMap<String, String>)`: Multiple credential pieces (adapter-specific, not yet used).
- **Constructors**: `from_env(env_name)`, `from_single(value)`, `from_multi(data)`.
- `single_key_value()`: Resolves to a single key string (reads env if `FromEnv`).

### `AuthResolver`

- `from_resolver_fn(f)`: Sync resolver. `f: Fn(ModelIden) -> resolver::Result<Option<AuthData>>`.
- `from_resolver_async_fn(f)`: Async resolver. `f: Fn(ModelIden) -> Pin<Box<dyn Future<Output = resolver::Result<Option<AuthData>>> + Send>>`.
- If the resolver returns `None`, the adapter's default auth is used.
- Runs after model mapping and before the service target resolver. With a bound adapter client, bare model names have already been routed through the bound adapter before this resolver runs.

### `Endpoint`

Efficiently clonable endpoint holder.

- `Endpoint::from_static(url)`: From `&'static str`.
- `Endpoint::from_owned(url)`: From `impl Into<Arc<str>>`.
- `base_url()`: Returns `&str`.

### `ModelMapper`

Maps a resolved `ModelIden` to another before execution.

- `ModelMapper::from_mapper_fn(f)`: `f: Fn(ModelIden) -> resolver::Result<ModelIden>`.
- Runs before auth resolution. Use `ClientBuilder::with_adapter_kind(...)` when the desired behavior is simply to bind all bare model names to one adapter.

### `ServiceTargetResolver`

- `from_resolver_fn(f)`: Sync. `f: Fn(ServiceTarget) -> resolver::Result<ServiceTarget>`.
- `from_resolver_async_fn(f)`: Async. `f: Fn(ServiceTarget) -> Pin<Box<dyn Future<Output = resolver::Result<ServiceTarget>> + Send>>`.
- Maps `ServiceTarget` to a final call target, allowing full override of endpoint, auth, or model.
- Runs last. Even with a bound adapter client, this resolver can still mutate the final target, including endpoint, auth, and model identity.

### `Headers`

Single-value-per-name HTTP header map.

- `merge(overlay)`: Merge overlay into self (consumes overlay).
- `merge_with(&overlay)`: Merge from borrowed overlay.
- `applied_to(target)`: Apply self on top of target, consuming both.
- `iter()`, `iter_mut()`, `IntoIterator` implementations.
- `From<HashMap<String, String>>`, `From<(K, V)>`, `From<Vec<(K, V)>>`, `From<[(K, V); N]>`.

### `resolver::Error`

- `ApiKeyEnvNotFound { env_name }`: Environment variable not found.
- `ResolverAuthDataNotSingleValue`: AuthData is not a single value.
- `Custom(String)`: Custom error message. `From<String>`.

## AdapterKind

Enum identifying the AI provider adapter.

Variants (updated in v0.6.0): `openai`, `openai_resp`, `gemini`, `anthropic`, `fireworks`, `together`, `groq`, `aihubmix`, `mimo`, `moonshot`, `nebius`, `xai`, `deepseek`, `zai`, `bigmodel`, `aliyun`, `baidu`, `cohere`, `ollama`, `ollama_cloud`, `opencode_go`, `vertex`, `github_copilot`, `bedrock_api`, `bedrock_sigv4`, `open_router`.

  - Namespace matches adapter lowercase name (updated in v0.6.0 with namespaces such as `open_router::`, `bedrock_api::`, `bedrock_sigv4::`, `vertex::`, `github_copilot::`, `opencode_go::`, `baidu::`, `aliyun::`, `moonshot::`, `aihubmix::`, and `ollama_cloud::`).

- `as_str()`: Display name (e.g., `"OpenAI"`, `"xAi"`).
- `as_lower_str()`: Lowercase name (e.g., `"openai"`, `"xai"`).
- `from_lower_str(name)`: Parse from lowercase.
- `default_key_env_name()`: Returns `Option<&'static str>` (e.g., `"OPENAI_API_KEY"`, `None` for Ollama).
- `from_model(model_name)`: Infers adapter from model name string (see rules below).
- Implements `Display`, `Clone`, `Copy`, `Eq`, `Hash`, `Serialize`, `Deserialize`.

## Model Resolution Nuances

- **Auto-detection** (updated in v0.6.0, `AdapterKind::from_model`):
  - `gpt-*` (except `gpt-oss`), `o1*`, `o3*`, `o4*`, `chatgpt*`, `codex*`, `text-embedding*` -> `OpenAI` (or `OpenAIResp` for `gpt-5` and other models with `codex` or `pro` in name).
  - `gemini*` -> `Gemini`.
  - `claude*` -> `Anthropic`.
  - Contains `"fireworks"` -> `Fireworks`.
  - `mimo-*` -> `Mimo`.
  - `command*`, `embed-*` -> `Cohere`.
  - `deepseek-*` -> `DeepSeek`.
  - `moonshot-*` -> `Moonshot`.
  - `grok*` -> `Xai`.
  - `glm*` -> `Zai`.
  - Fallback -> `Ollama`.
- **Namespacing**: `namespace::model_name` (updated in v0.6.0, for example, `open_router::openai/gpt-4.1`, `together::meta-llama/...`, `nebius::Qwen/...`).

  - Namespace matches adapter lowercase name (updated in v0.6.0 to support `open_router::`, `bedrock_api::`, `bedrock_sigv4::`, `vertex::`, `github_copilot::`, `opencode_go::`, `baidu::`, `aliyun::`, `moonshot::`, `aihubmix::`, and `ollama_cloud::`).
  - Special: `zai_coding::` namespace maps to `Zai` adapter for subscription endpoint.
- **Bound Adapter Clients**:
  - Since v0.6.0, `ClientBuilder::with_adapter_kind(adapter_kind)` and `ClientConfig::with_adapter_kind(adapter_kind)` bind a client to one adapter.
  - Bare `ModelSpec::Name` values route through the bound adapter without using model-name inference.
  - Namespaced `ModelSpec::Name` values must match the bound adapter, otherwise `AdapterKindMismatch` is returned.
  - `ModelSpec::Iden` values must match the bound adapter, otherwise `AdapterKindMismatch` is returned.
  - `ModelSpec::Target` is already fully resolved and is not changed by bound-adapter routing.
  - `ServiceTargetResolver` still runs last and can mutate the final call target.
- **Namespace-only or recommended namespace providers**:
  - `groq::model_name` is required for direct Groq targeting (updated in v0.6.0).
  - `vertex::model_name` targets Google Vertex AI Model Garden (since v0.6.0).
  - `aliyun::model_name` targets Aliyun's OpenAI-compatible service (since v0.6.0).
  - `baidu::model_name` targets Baidu (since v0.6.0).
  - `aihubmix::model_name` targets AIHubMix (since v0.6.0).
  - `moonshot::model_name` targets Moonshot AI (since v0.6.0).
  - `ollama_cloud::model_name` targets Ollama Cloud (since v0.6.0).
  - `opencode_go::model_name` targets OpenCode Go (since v0.6.0).
  - `bedrock_api::model_name` targets AWS Bedrock Converse API with Bearer auth (since v0.6.0).
  - `bedrock_sigv4::model_name` targets AWS Bedrock Converse API with SigV4 auth (since v0.6.0).
  - `open_router::model_name` targets OpenRouter OpenAI-compatible gateway (since v0.6.0).
- **Ollama Fallback**: Unrecognized non-namespaced names default to `Ollama` adapter (localhost:11434).
- **Reasoning Normalization**: Automatic extraction for DeepSeek/Ollama `<think>` blocks when `normalize_reasoning_content` is enabled.

### Custom Adapter

The `genai_{n}::` namespace (e.g., `genai_1::my-model`) provides a built-in mechanism to add custom OpenAI-compatible
(and later, other protocol) endpoints without writing an adapter. Since v0.7.0-beta.

- **Endpoint**: Required base URL set via `GENAI_{n}_ENDPOINT`. Must include the path (e.g., `https://my-api.example.com/v1/`).
  If the URL does not end with `/`, one is appended automatically.
- **Auth**: Optional API key set via `GENAI_{n}_API_KEY`. If omitted, no authentication header is sent (useful for local unauthenticated endpoints).
- **Default protocol**: Currently defaults to the **OpenAI** Chat Completions protocol. The built-in custom adapter delegates
  to the OpenAI adapter, so the target service must be OpenAI-compatible.

Example:

```rust
use genai::Client;

let client = Client::default();
// Set environment variables:
//   GENAI_1_ENDPOINT=https://my-api.example.com/v1/
//   GENAI_1_API_KEY=sk-...
let chat_res = client.exec_chat("genai_1::some-model", chat_req, None).await?;
```

- **Number range**: The `n` in `genai_{n}` is a `u16` (0..65535). Multiple custom endpoints can be defined by using different numbers.
- **Advanced configuration**: For non-OpenAI protocols, custom resolvers, or fine-grained control, consider implementing a
  custom adapter or using the [`ServiceTargetResolver`](#servicetargetresolver) directly.



## Error Handling

- `genai::Error`: Main error enum. Variants:
  - `ChatReqHasNoMessages { model_iden }`: Request has no messages.
  - `LastChatMessageIsNotUser { model_iden, actual_role }`: Last message role is not User.
  - `MessageRoleNotSupported { model_iden, role }`: Role not supported for model.
  - `MessageContentTypeNotSupported { model_iden, cause }`: Content type not supported.
  - `JsonModeWithoutInstruction`: JSON mode without any instruction.
  - `VerbosityParsing { actual }`: Failed to parse verbosity.
  - `ReasoningParsingError { actual }`: Failed to parse reasoning effort.
  - `ServiceTierParsing { actual }`: Failed to parse service tier.
  - `PromptCacheRetentionParsing { actual }`: Failed to parse prompt cache retention.
  - `NoChatResponse { model_iden }`: No response from model.
  - `InvalidJsonResponseElement { info }`: Invalid JSON in response.
  - `RequiresApiKey { model_iden }`: API key required.
  - `NoAuthResolver { model_iden }`: No auth resolver found.
  - `NoAuthData { model_iden }`: No auth data available.
  - `ModelMapperFailed { model_iden, cause }`: Model mapping failed.
  - `WebAdapterCall { adapter_kind, webc_error }`: Web call failed (adapter level).
  - `WebModelCall { model_iden, webc_error }`: Web call failed (model level).
  - `ChatResponseGeneration { model_iden, request_payload, response_body, cause }`: Error generating ChatResponse.
  - `ChatResponse { model_iden, body }`: Error event in stream.
  - `StreamParse { model_iden, serde_error }`: Stream data parse failure.
  - `WebStream { model_iden, cause, error }`: Web stream error.
  - `HttpError { status, canonical_reason, body }`: HTTP error.
  - `Resolver { model_iden, resolver_error }`: Resolver error wrapper.
  - `AdapterNotSupported { adapter_kind, feature }`: Feature not supported by adapter.
  - `AdapterKindMismatch { bound, requested, model }`: A client bound to one adapter received a namespaced model or `ModelIden` targeting another adapter. Since v0.6.0.
  - `Internal(String)`: Internal error.
  - `JsonValueExt(JsonValueExtError)`: From `value_ext`.
  - `SerdeJson(serde_json::Error)`: From `serde_json`.
- `Result<T>`: Alias for `core::result::Result<T, genai::Error>`.
- `BoxError`: Type alias for `Box<dyn std::error::Error + Send + Sync>`.

### `webc::Error`

Public sub-error for web client operations.

- `ResponseFailedNotJson { content_type, body }`: Response is not JSON.
- `ResponseFailedInvalidJson { body, cause }`: Invalid JSON in response.
- `ResponseFailedStatus { status, body, headers }`: Non-success HTTP status.
- `JsonValueExt(JsonValueExtError)`: From `value_ext`.
- `Reqwest(reqwest::Error)`: From `reqwest`.

## Size Tracking

Many types implement `.size() -> usize` for approximate in-memory size tracking:
`Binary`, `ContentPart`, `MessageContent`, `ChatMessage`, `Tool`, `ToolCall`, `ToolResponse`.
