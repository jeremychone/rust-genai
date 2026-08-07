`.` minor | `-` Fix | `+` Addition | `^` improvement | `!` Change | `>` Refactor

## v0.7.0-beta.x (see [genai versions](https://crates.io/crates/genai/versions))

- `!` API CHANGE - `Error::HttpError` adds a `headers: Box<HeaderMap>` field carrying the response headers of failed streaming HTTP calls (e.g., `retry-after`, `retry-after-ms`, `x-should-retry`), matching the non-streaming `webc::Error::ResponseFailedStatus`, so downstream retry layers can honor provider-requested retry delays. Exhaustive matchers/constructors of `Error::HttpError` must add the `headers` field (or match with `..`).
- `!` API CHANGE - `Tool` adds the public `custom_format: Option<Value>` field for provider-native freeform custom-tool formats. Downstream `Tool` struct literals must add `custom_format: None`, or preferably migrate to `Tool::new(...)` and builder methods. `Tool::with_custom_format(...)` is the new builder API.
- `!` API CHANGE - `ToolResponse` adds the public `parts: Option<Vec<Binary>>` field for binary tool-result attachments (e.g., screenshots produced by agentic tools). Downstream `ToolResponse` struct literals must add `parts: None`, or preferably migrate to `ToolResponse::new(...)` and the new `ToolResponse::with_parts(...)` / `ToolResponse::append_binary(...)` builders. Image parts serialize natively where the wire supports them (Anthropic `tool_result`, Bedrock Converse `toolResult`, OpenAI Responses `function_call_output`) and ride in a follow-up user message elsewhere (OpenAI Chat Completions-compatible providers, Gemini, Ollama). Non-image parts are skipped with a warning, and text-only tool responses keep their exact previous serialization on every adapter.
- `!` BEHAVIOR CHANGE - A `ContentPart::ToolResponse` embedded in an Assistant-role message now fails serialization with the existing `Error::MessageContentTypeNotSupported` (no new error variant) on every adapter: OpenAI Chat Completions and Responses, Anthropic, Bedrock Converse, Gemini, and Ollama native — including the delegating providers that reuse those serializers (Vertex Claude models, opencode_go, minimax, github_copilot, ollama_cloud, ...). Previously Anthropic, Bedrock Converse, and Gemini silently dropped the embedded response, Ollama garbled it into the assistant `content`, and the OpenAI serializers silently dropped it. No provider wire represents a tool result authored by the assistant, so the shape has no faithful translation; the error's cause points at the supported Tool-role message shape (e.g., `ChatMessage::from(ToolResponse)`). The crate-wide rule is now: supported shapes are translated faithfully, unsupported shapes fail loudly, nothing vanishes.
- `+` `Client` adds per-request exec hooks, following the resolver idiom (dedicated types with sync/async function variants, installed via the `ClientBuilder`, stored in the `ClientConfig`). `ClientBuilder::with_payload_interceptor(...)` / `with_payload_interceptor_fn(...)` set a `PayloadInterceptor` called on each chat exec call (streaming and non-streaming) with the target `ModelIden` and the serialized provider payload (`serde_json::Value`) before the HTTP request is built — returning `Some(value)` replaces the payload, `None` keeps it unchanged. `ClientBuilder::with_response_observer(...)` / `with_response_observer_fn(...)` set a `ResponseObserver` called with the `ModelIden`, response `StatusCode`, and `HeaderMap` as soon as the HTTP response arrives and before its body/stream is consumed — including on 4xx/5xx responses (on the streaming path, the send is lazy, so the observer fires during the first stream poll, before the status check and the `Error::HttpError` construction). Chat exec paths only (`exec_chat` / `exec_chat_stream`); embeddings and model listing are not hooked.
- `+` New Providers:
  - AtlasCloud - default env: `ATLASCLOUD_API_KEY`, Adapter: OpenAI, endpoint: `https://api.atlascloud.ai/v1/` (activated on the `atlascloud::` namespace) (PR #259)
  - Qwen Cloud - default env: `QWEN_CLOUD_API_KEY`, Adapter: OpenAI, endpoint: `https://dashscope-intl.aliyuncs.com/compatible-mode/v1/` (activated on the `qwen_cloud::` namespace)
  - Kimi - default env: `KIMI_API_KEY`, Adapter: OpenAI, endpoint: `https://api.moonshot.ai/v1/` (activated on the `kimi::` namespace or `kimi` model prefix, moonshot.ai)
- Anthropic:
  - `+` Serialize `ToolResponse.parts` image attachments as base64 `image` blocks inside the `tool_result` content array (after the text block). Text-only tool responses keep the legacy plain-string `content`. Non-image parts are skipped with a warning, since Anthropic `tool_result` content only accepts text and image blocks.
  - `^` Support URL image sources in user messages and tool results: `BinarySource::Url` images now serialize natively as `{"type": "image", "source": {"type": "url", "url": ...}}` blocks instead of being silently omitted with a warning (the Anthropic Messages API natively supports URL image sources). Base64 image serialization is unchanged. Delegating providers that reuse the Anthropic serializers (minimax, the `baidu-coding-anthropic` namespace, Vertex Claude models, opencode_go minimax models) inherit this automatically; whether a given gateway accepts URL sources is provider-side.
  - `+` Expose streaming SSE ping messages as provider-neutral `ChatStreamEvent::Heartbeat` events, allowing callers to distinguish a live long-running stream from a stall. (PR #271)
  - `+` Add prompt caching on tools via `Tool::with_cache_control`, and make request-level `ChatOptions::with_cache_control` automatically apply a cache breakpoint to the static (tools+system) prefix, which was previously ignored. `Ephemeral24h` is documented as clamped to Anthropic's max `1h` TTL.
  - `+` Support the `extra_body` `ChatOptions` field, merging extra request body fields. ([#255](https://github.com/jeremychone/rust-genai/pull/255))
  - `-` Capture streaming cache tokens from the `message_delta` fallback. (PR #258)
  - `-` Fix: reuse Client WebClient for model listing. ([#249](https://github.com/jeremychone/rust-genai/pull/249))
  - `+` Sanitize JSON Schema for structured responses and strict tools. (PR #263)
  - `!` `ReasoningEffort::None` is renamed to `ReasoningEffort::Zero`, avoiding confusion with `Option::None`. `#[serde(alias = "None")]` keeps old JSON deserializable. The canonical keyword is now `"zero"` (was `"none"`), `as_keyword()` and `Display` emit `"zero"`, and `from_keyword()` still accepts `"none"` as a backward-compatible alias. (PR #253, #251)
    - `Zero` now positively disables reasoning, whereas it previously triggered adaptive thinking.
    - Sonnet 5 sends `thinking: {"type": "disabled"}`, because thinking is on by default.
    - Other models omit `thinking` and `output_config.effort`.
    - Fable and Mythos omit `thinking`, because it is always on and cannot be explicitly disabled.
    - The Anthropic `-zero` model suffix is canonical, while `-none` remains a backward-compatible alias. Both map to `Zero` and are stripped.
- OpenAI:
  - `-` Chat Completions and Responses: a `ToolResponse` embedded in a User-role message is now serialized instead of silently dropped (text and all). This user-embedded shape (the Anthropic-style form where tool results ride as user content blocks) is extracted into proper `role:"tool"` messages / `function_call_output` items (`custom_tool_call_output` for custom tool calls) emitted before the carrying user message, with images folded into that same user message (`image_url` / `input_image` blocks, no label); a user message left empty by the extraction is omitted. Text/placeholder and custom-output rules match Tool-role serialization, and `call_id`s are serialized as-is (provider-side validation, as elsewhere). The Ollama native serializer (shared by `ollama_cloud`) gets the same extraction — it previously garbled the shape (the response text was inserted as the user `content`, where sibling text parts overwrote it, and image parts were lost): the user-embedded `ToolResponse` now becomes a `role:"tool"` message before the carrying user message, its images ride the existing labeled follow-up user image message (native base64 `images` array), and the same empty-user-message omission applies; in the same stroke, the Ollama Tool-role path now emits one `role:"tool"` message per `ToolResponse` when a Tool-role message carries several, in part order (previously each response's text overwrote the previous, keeping only the last), with their images still batched into that single labeled follow-up user image message. A `ToolResponse` embedded in an Assistant-role message fails loudly instead — see the crate-wide BEHAVIOR CHANGE entry above.
  - `+` Chat Completions: `ToolResponse.parts` images ride in a follow-up `user` message (`image_url` blocks), batched across a run of consecutive tool messages; the `tool` message keeps its text, or the `"(see attached image)"` placeholder when the result is image-only. Applies to all OpenAI-compatible providers sharing this serializer.
  - `+` Responses: `ToolResponse.parts` images serialize natively as `input_image` items in the `function_call_output` `output` array (after the `input_text` item). Custom tool-call outputs stay raw strings (with the `"(see attached image)"` / `"(no tool output)"` placeholder rules); their images ride in a follow-up `user` message input item, batched across a run of consecutive tool messages.
  - `+` Support OpenAI Responses freeform custom tools with grammar-constrained raw-string input. Custom tools serialize as `type: "custom"`, custom tool-call input streams incrementally, and round-trips as `custom_tool_call` / `custom_tool_call_output` items. (PR #266)
  - `^` Capture `cache_write_tokens` from prompt-cache usage and normalize it to `Usage.prompt_tokens_details.cache_creation_tokens` for Chat Completions and Responses API payloads.
  - `^` GPT-5.6 and later now use cache opt-in only. Here is how to opt in: (see [PR #260](https://github.com/jeremychone/rust-genai/pull/260))
    - Set a request-level cache intent with `ChatOptions::with_cache_control(...)` or provide a `prompt_cache_key`. OpenAI then uses `prompt_cache_options.mode = "implicit"` and manages cache placement without an invented content breakpoint.
    - Set message-level cache control to use explicit mode and place one cache breakpoint on the last eligible content block. Chat Completions supports text, image, audio, file, and refusal blocks. Responses supports input text, input image, and input file blocks.
    - Otherwise, when there is no request-level cache intent, `prompt_cache_key`, message-level cache control, or tool-level cache control, the mode is set to `"explicit"` with no breakpoint. Nothing is implicitly cached.
    - Tool-level cache control is ignored by OpenAI because the supported protocols do not provide a valid tool-definition breakpoint representation. It does not change the cache mode, fail serialization, or emit an unsupported breakpoint field.
    - Message-level cache placement is best effort. When a controlled message has no eligible content block, OpenAI omits the breakpoint and continues request serialization without failing.
    - This policy applies only to native OpenAI Chat Completions and Responses requests for GPT-5.6 and later. Older OpenAI models retain legacy cache-retention behavior, and OpenAI-compatible adapters do not receive these OpenAI-specific fields.
    - Existing normalized usage continues to expose cache reads through `cached_tokens` and cache writes through `cache_creation_tokens`.
  - `+` Sanitize JSON Schema for structured responses and strict tools. (PR #263)
  - `!` Apply the `ReasoningEffort::None` to `ReasoningEffort::Zero` rename mechanically, while preserving provider-specific keyword mappings.
- Gemini:
  - `+` `ToolResponse.parts` images ride in a follow-up `user` turn (`inline_data` / `file_data`) emitted after the merged `functionResponse` turn; the `functionResponse` keeps its text, or the `"(see attached image)"` placeholder when the result is image-only. Also applies to Vertex (Google publisher).
  - `^` Forward JSON Schema raw via `responseJsonSchema` and `parametersJsonSchema`. (PR #257)
  - `!` Map `ReasoningEffort::Zero` to a budget of `0`, which might be rejected by the provider on some models.
  - `-` Protect known model names such as `deepseek-r1-zero` from reasoning suffix stripping by using a whitelist in `from_model_name()`.
- Bedrock:
  - `+` `ToolResponse.parts` images serialize natively as `image` blocks inside the Converse `toolResult` content array (after the text block).
  - `!` Apply the `ReasoningEffort::None` to `ReasoningEffort::Zero` rename mechanically, with behavior unchanged.
- Ollama:
  - `+` `ToolResponse.parts` images (base64 only) ride in a follow-up `user` message via the native `images` array; the `tool` message keeps its text, or the `"(see attached image)"` placeholder when the result is image-only.
- Cross-provider adapters:
  - `^` Move messages after tools in JSON payloads for better prompt cache utilization. (PR #262)
- OpenTelemetry:
  - `-` Fix `otel` feature compilation, by covering the `CacheBreakpointNoEligibleContent` error variant in the `error.type` derivation (broken since v0.7.0-beta.18).
  - `+` Add optional OpenTelemetry GenAI semantic-convention instrumentation behind the new `otel` feature, off by default, using a pure `tracing` bridge with no extra dependencies.
    - Auto-instruments `exec_chat`, `exec_chat_stream`, and `exec_embed` as `gen_ai.*` spans, including operation, provider, request params, server address/port, usage tokens, finish reasons, response id/model, streaming time-to-first-chunk, and `error.type`. Prompt and response content capture is opt-in via `OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT`. Adds opt-in `genai::otel` helpers for agent, workflow, and tool spans, plus the evaluation-result event. Export by wiring `tracing-opentelemetry` in the application. See `docs/otel.md` and `examples/c12-otel.rs`.

## 2026-06-06 [v0.6.5](https://github.com/jeremychone/rust-genai/compare/v0.6.4...v0.6.5)

- `-` fix: capture OpenAI stream usage tail (PR #242)

## 2026-06-01 [v0.6.4](https://github.com/jeremychone/rust-genai/compare/v0.6.3...v0.6.4)

- `+` NEW PROVIDER: MiniMax ([https://platform.minimax.io/](platform.minimax.io/)) with `MiniMax` or `minimax` prefixes, or `minimax::` namespace.
- `^` build - reqwest TLS is now selectable via cargo features: `rustls-tls` (default; rustls + aws-lc-rs + OS trust store) and `native-tls`. 
  - reqwest switches to `default-features = false` with all previously-implicit features pinned explicitly; the default (`rustls-tls`) uses the same crypto backend as before, so HTTPS keeps working out of the box. BYO crypto provider / custom CA / mTLS via `ClientBuilder::with_reqwest`. Enabling both backends at once fails fast with a `compile_error!` instead of silently picking one.
- `>` Refactored `Antrophic` adapter with `antropic_shared.rs` (internal)

## 2026-05-29 - [v0.6.3](https://github.com/jeremychone/rust-genai/compare/v0.6.2...v0.6.3)

- `^` use adaptive thinking for opus 4.7 (and above)

## 2026-05-27 - [v0.6.2](https://github.com/jeremychone/rust-genai/compare/v0.6.1...v0.6.2)

- `-` fix - openai_resp: tolerate response.completed events without `output` field (PR #236)
- `^` adapter/openai - Fix ReasoningEffort::Max mapping to 'max' keyword

## 2026-05-24 - [v0.6.1](https://github.com/jeremychone/rust-genai/compare/v0.6.0...v0.6.1)

- `^` openai shared adapter - remove OpenAI only guard on reasoning suffix
  - now all openai compatible adapters get the reasoning suffix resolved

## 2026-05-22 - [v0.6.0](https://github.com/jeremychone/rust-genai/compare/v0.5.3...v0.6.0)

- API changes:
  - `+` API NEW - `all_model_names(adapter_kind, provider_config)` - added `ProviderConfig` for model listing (endpoint/auth overrides)
  - `!` API CHANGE - `all_model_names()` - now live (with AuthResolver support)
  - `+` API NEW - New `ModelSpec` to define custom endpoint, model, ..
  - `+` API NEW - add openai resp stateful sessions — `previous_response_id`, `store`, `response_id` (PR #168)
  - `+` API NEW - Add `ContentPart::ReasoningContent` support
  - `+` API NEW - expose provider `stop_reason` in chat responses
  - `+` API NEW - add typed and normalized built-in tools, `ToolName`, `ToolConfig`, `WebSearch`, and related tool support
  - `+` API NEW - WebSearch builtin tool spport for Anthropic, OpenAI, Gemini
  - `^` API NEW - chat-level prompt cache `CacheControl` with openai prompt_cache_key Support
  - `^` API NEW - Add support for `ReasoningEffort::Max` (Anthropic) and `ReasoningEffort::XHigh` (OpenAI)
  - `^` openai - now support prompt_cache_key in `ChatOptions` (and `prompt_cache_retention` via `CacheControl`)
  - `!` openai_resp - gate `reasoning.encrypted_content` on `capture_reasoning_content`
  - `!` openai_resp - make `reasoning.summary` opt-in for `capture_reasoning_content`
  - `!` gemini - make `thinkingConfig/includeThoughts` opt-in for `capture_reasoning_content`
  - `!` groq - providers must be addressed via namespaced model (`groq::_model_name`)
  - `>` AuthData - add `None` variant
- New Providers:
    - AWS Bedrock (`bedrock_api` and `bedrock_sigv4` adapters)
    - `open_router`
    - `vertex` (with Gemini and Anthropic support)
    - `github_copilot` (GitHub Models API)
    - `opencode_go`
    - `baidu`
    - `aliyun`
    - `moonshot`
    - `aihubmix`
    - `ollama_cloud` (Ollama Cloud)
- Other additions & enhancements:
  - `!` zai - now use `zai_coding` for the plan based (not `coding` anymore)
  - `^` gemini - use provider-returned `call_id` for tool calls (PR #232)
  - `+` anthropic - add JSON schema support
  - `^` perf - enable HTTP optimizations, gzip, `TCP_NODELAY`, and HTTP/2 tuning
  - `^` ollama - implement native API support (BIG)
  - `^` openai - route GPT-5 models through the OpenAI Responses API
  - `^` openai - add request-level prompt cache support and use `instructions` for Responses API system prompts
  - `^` anthropic - add support for adaptive thinking
  - `^` anthropic - emit incremental `ToolCallChunk` events during streaming
- Others:
  - `^` docs - comprehensive update of LLM API reference, README, and migration guide for v0.6.0
  - `^` doc - sync llm api reference, spec rules, and tool spec
  - `+` tests - add yakbak Gemini streaming replay test
  - `+` tests - add yakbak HTTP record/replay integration test infrastructure
  - `>` ModelName - add `Static` and `Shared` inner support
  - `>` adapter - update `Adapter` trait with `DEFAULT_API_KEY_ENV_NAME` and update implementations
  - `>` openai - relayout adapter implementation and shared code
  - `>` examples - rename examples
  - `-` openai_resp - fix buffering of incomplete UTF-8 sequences across stream chunks
  - `-` openai - capture inline usage from `finish_reason` stream chunks
  - `-` anthropic - guard against null `tool_call` arguments in request serialization
  - `-` anthropic - implement missing prompt caching fixes, cache token capture and normalization, TTL support, and per-part cache control support
  - `-` gemini - support parallel tool calls in streaming adapter
  - `-` openai - fix streamer to emit delta content from `finish_reason` message
  - `-` gemini - fix JSON schema compatibility and usage-only stream tail handling
  - `-` openai - surface SSE error payloads in streaming
  - `-` openai - fix recursive issue on tool handling
  - `-` gemini - fix tool serialization to use `functionDeclarations` camelCase

## 2026-01-31 - [v0.5.3](https://github.com/jeremychone/rust-genai/compare/v0.5.2...v0.5.3)

- `^` error - add request payload / response body when to chat response fail
- `>` refactor captured_raw_body into client .exec_chat (prep for #137)
- `.` tracing - add traced to web-client for ai response (#132)
- `-` Fix incorrect empty output from MessageContent::joined_texts for ≥ 2 text parts (fixes #135) (#136) Co-authored-by: Ross MacLeod <rmm+github@z.odi.ac>
- `.` ChatRole - Add PartialEq / Eq (#131)

## 2026-01-27 - [v0.5.2](https://github.com/jeremychone/rust-genai/compare/v0.5.1...v0.5.2)

- `-` Does not capture body when json parse fail  (#128)
- `^` Anthropic - Add separate reasoning content and thought signature for anthropic messages api (#125)
- `-` fix - Ollama tool calls are silently swallowed in OpenAI adapter (streaming) (#124)
- `^` test - ollama - add tool tests
- `^` gemini - Include thoughts and capture thoughts are reasoning content (#121)

## 2026-01-17 - [v0.5.1](https://github.com/jeremychone/rust-genai/compare/v0.5.0...v0.5.1)

`!` `Error::WebStream` - added error field to preserve original error
`^` gemini - allow empty tool `thoughtSignature` for Gemini 3 (#115)
`-` webc - check HTTP status in `WebStream` before processing byte stream (#117)
`-` client - ensure extra headers are applied in `exec_chat` and `exec_chat_stream` (#116)
`-` openai_resp - fix assistant message content to use `output_text` (#119)

## 2026-01-09 - [v0.5.0](https://github.com/jeremychone/rust-genai/compare/v0.4.4...v0.5.0)

- `!` zai - change namespace strategy with (zai:: for default, and zai-codding:: for subscription, same Adapter)
- `+` New Adapter: bigmodel - add back bigmodel.cn and BigModel adapter (only via namespace)
- `+` MessageContent - Add from ContentPart and Binary
- `+` New Adatper: : Add MIMO model adapter (#105)
- `+` gemini adapter - impl thought signature - ThoughtSignature api update
- `^` anthropic - implemented new output_config.effort for opus-4-5 (matching ReasonningEffort)
- `^` gemini - for gemini-3, convert ReasoningEffort Low/High to the appropriate gemini thinkingLevel LOW/HIGH, fall back on budget if not gemini 3 or other effort
- `^` reasoning - add RasoningEffort::None
- `^` dependency - update to reqwest 0.13
- `^` MessageContent - add .binaries() and .into_binaries()
- `^` .size - implement .size in ContentPart and MessageContent
- `^` ContentPart - Binary from file (as base64)
- `^` binary - add constructors (from_base64, from_url, from_file)
- `-` pr-anthropic-tool-fix - #pr 114 - Anthropic ToolCalls with no parameters are not parsed correctly while streaming
- `-` Fix Gemini adapter to use responseJsonSchema (PR #111)
- `-` Fix Ollama reasoning streaming (Skip empty reasoning chunks in streaming)
- `-` Fix Fireworks default depending on streaming (#109)
- `-` Capture response body in ResponseFailedNotJson error (#103)
- `>` anthropic - Refactor streamer to use webc::EventSourceStream
- `>` adapter_openai - switched to custom webc::EventSourceStream based on WebStream
- `>` webc - remove 'reqwest-eventsource' dependency, all based in same WebStream (EventsourceStream wrapper)
- `>` ModelName - add namespace_is(..), namespace(), namespace_and_name()
- `>` binary - refactor openai to use into_url for the base64 url
- `>` content_part - refactor binary into own file

## 2025-11-14 - [v0.4.4](https://github.com/jeremychone/rust-genai/compare/v0.4.3...v0.4.4)

- `+` openai - adding support for gpt-5-pro (must be mapped to OpenaiResp adapter)
- `+` Add support for openai audio_type content part for voice agent support. ([PR #96](https://github.com/jeremychone/rust-genai/pull/96) thanks to [Vagmi Mudumbai](https://github.com/vagmi))
- `+` Add support for OpenAI `service_tier` parameter. ([PR #98](https://github.com/jeremychone/rust-genai/pull/98) thanks to [Himmelschmidt](https://github.com/Himmelschmidt))


## 2025-10-25 - [v0.4.3](https://github.com/jeremychone/rust-genai/compare/v0.4.2...v0.4.3)

- `!` Refactor ZHIPU adapter to ZAI with namespace-based endpoint routing (#95)
- `-` openai - stream tool - Fix streaming too issue (#91)
- `.` added ModelName partial eq implementations for string types (#94)
- `.` anthropic - update model name for haiku 4.5

## 2025-10-12 - [v0.4.2](https://github.com/jeremychone/rust-genai/compare/v0.4.1...v0.4.2)

- `.` test - make the common_test_chat_stop_sequences_ok more resilient
- `^` Anthropic - preserve the content order as it appears in the JSON array (#89)
- `.` Gemini - when no response, return error with finishReason and usageMetadata

## 2025-09-30 - [v0.4.1](https://github.com/jeremychone/rust-genai/compare/v0.4.0...v0.4.1)

- `^` anthropic - add reasoning support

## 2025-09-28 - [v0.4.0](https://github.com/jeremychone/rust-genai/compare/v0.3.5...v0.4.0)

Some **API Changes** - See [migration-v_0_3_to_0_4](doc/migration/migration-v_0_3_to_0_4.md)

- `+` openai - gpt-5-codex - Add support for gpt-5-codex via Responses OpenAIResp adapter 
- `!` StreamEnd - now have conent: Option<MessageContent> (but same public api)
- `!` ChatResponse - now .content: MessageContent (rather than Vec<MessageContent>). Same public interface.
- `^` chat_stream - Relax Unpin bound in from_inter_stream
- `^` chat/content_part - trim() in MIME checks, add into_binary(), and struct spacing; update work plan/history
- `!` ChatRequest::append/with_...(vec) functions  now take iterators
- `-` ChatRequests::combine_systems - keeping one empty line in between
- `-` ChatOptions - fix minimal reasoning effort (was matching to low)
- `+` openai - add verbosity parameter
- `!` ContentPart - from/new binary now have name last (since optional)
- `!` ContentPart - Now use ContentPart::Binary(Binary)
- `!` MessageContent - New structure (flatten multi-part)
- `.` gemini - update gemini-2.5-flash-lite model name
- `^` openai - add support for the -minimal suffix
- `.` test - openai - add gpt-5-mini when possible
- `.` openai - exclude the 'gpt-oss..' model to allow ollama fallback
- `>` test - refactoring to TestResult for better test error display
- `!` API CHAINGE - Now ContentPart::Binary
- `+` Added support for PDF (and file in general) for openai, anthropic, and gemini
- `^` fireworks - set the default max_tokens to 256k (won't fail if model lower)
- `.` openai streamer - check that tool_calls is not null to enter tool_calls (for together streaming)
- `.` openai streamer - add teogether for capture usage
- `*` openai streamer - make the content extraction more resilient (do not error or cannot x_take)
- `+` adapter - add together.ai adapter
- `+` adapter - add fireworks adapter
- `+` Add embeddings support (#83)
- `*` note about AuthData::RequestOverride being only for exec_chat_stream
- `.` c06-target-resolver - update model
- `!` Headers - change the API to set / override headers
- `.` groq - add models
- `-` gemini - fix -zero,low,... suffix issue
- `^` Anthropic - Add support for tool calls and thinking in the Anthropic streamer. (#80)
- `*` Add support for tool calls and thinking in the anthropic streamer.
- `.` ModelName / ... - add Hash, Eq, PartialEq
- `+` ChatOptions - Add extra headers to requests (#78)
- `+` Implement zhipu (ChatGLM) completions  (#76)
- `+` feat: add new model names to ZhipuAdapter (#77)
- `.` webc::error ResponseFailedStatus now takes headers: Box HeaderMap
- `-` fix: handle null tool calls in OpenAIRequestParts parser (#74)
- `-` fix: push empty content for sglang impl
- `-` fix: OpenAIAdapter to avoid pushing empty content after reasoning extraction (#72)
- `-` Fix OpenAIAdapter to avoid pushing empty content after reasoning extraction
- `-` trim strings too
- `!` MessageContent - Now use text() and into_text() (from text_as_str, text_into_string)
- `>` refactor capture_raw_body (from #68) - move it to ChatOptions to be able to override it by chat request
- `-` feat: support optionally capture raw response body (#68)
- `-` Extend OpenAI adapter to preserve request params (#71)
- `+` feat: support embeded tools like gemini's google search (#67)
- `-` fix: improve reasoning content extraction in OpenAIAdapter (#69)
- `-` Updated the reasoning content extraction logic to handle cases where reasoning may be present in multiple locations within the response. Enhanced error handling for the extraction process to ensure robustness.
- `-` feat: add web configuration support (#66)
- `-` Introduced web_config.rs for web-specific configurations, and updated related modules to integrate web configuration capabilities into the client builder.
- `-` gemini - fix streaming multi-content
- `!` api change - StreamEnd - Now text and tool calls content part of Vec
- `^` Anthropic - add support for ChatResponse multi content (not the stream yet)
- `^` openai - add support for ChatResponse multi content (not the stream yet)
- `!` API CHANGE - ChatResponse.tool_calls now return Vec ToolCall
- `^` ChatResponse - now implements texts(), into_texts(), first...
- `!` api change - ChatResponse.content now have content Vec MessageContent
- `-` gemini - fix wrong tool_response.content json parsing (#59)
- `-` gemini - fix: fixed partial message parsing in Gemini stream (#63)
- `.` gemini - now use x-goog-api-key header for auth
- `.` tests - serial for anthropic
- `!` nebius - remove the model name match for Adapter selection (needs to use namespace)
- `+` Model Namespace Support
- `+` add Nebius adapter (#56)
- `+` Add Tool Use Streaming Support (#58)

## 2025-05-26 - [v0.3.5](https://github.com/jeremychone/rust-genai/compare/v0.3.4...v0.3.5)

- `^` OpenAI Adapter - Update OpenAI adapter to check for tool calls if the LLM returns an empty content response ([PR #55](https://github.com/jeremychone/rust-genai/pull/55))

## 2025-05-24 - [v0.3.4](https://github.com/jeremychone/rust-genai/compare/v0.3.3...v0.3.4)

- `^` Anthropic - update the default max_tokens for the models, including claude-*-4
- `-` Anthropic - fix the way prompt_tokens and cache_..._tokens are computed to match the normalized OpenAI Way

## 2025-05-20 - [v0.3.3](https://github.com/jeremychone/rust-genai/compare/v0.3.2...v0.3.3)

- `-` gemini - fix cache computation (cachedContentTokenCount is included in promptTokenCount, as openai)
- `^` xai - Fix/normalize xAI grok-3-beta API issue that does not compute completion_tokens the OpenAI way when reasonning_tokens
- `.` test - updated xai test to use grok-3-mini-beta (and grok-3-beta for streaming)

## 2025-05-14 - [v0.3.2](https://github.com/jeremychone/rust-genai/compare/v0.3.1...v0.3.2)

- `^` error - implement proper display for error variants

## 2025-05-10 - [v0.3.1](https://github.com/jeremychone/rust-genai/compare/v0.3.0...v0.3.1)

- `^` gemini - usage - add capture/normalize cached tokens usage (need futher validation, but should work)
- `-` xai - fix streaming usage capture (now same as openai), and list models

## 2025-05-08 - [v0.3.0](https://github.com/jeremychone/rust-genai/compare/v0.2.4...v0.3.0)

- `+` gemini - reasoning effort - thinking budget - Added `ReasoningEffort::Budget(num)` variant. 
  -  Minor API Update - Now `ReasoningEffort` has a new vairant `Budget`
- `+` gemini - reasoning effort - added `-zero`, `-low`, `-medium`, and `-high` suffixes, and also mapped the other variants to the correct budget when present.
- `^` ModelIden - added `from_name(new_name`) and `from_option_name(Option name)`
- `!` ModelIden - Minor API Change - deprecation of `with_name_or_clone` (use `from_optional_name`)

## 2025-05-07 - [v0.2.4](https://github.com/jeremychone/rust-genai/compare/v0.2.3...v0.2.4)

- `^` openai usage - (change) Now details properties None when 0, and usage.compact_details() to set details to None when empty.
- `.` gemini - remove wrongly assign accepted_prediction_tokens

## 2025-04-26 - [v0.2.3](https://github.com/jeremychone/rust-genai/compare/v0.2.2...v0.2.3)

- `-` gemini - fix computation of completion_tokens/reasoning_tokens to match OpenAI API way

## 2025-04-19 - [v0.2.2](https://github.com/jeremychone/rust-genai/compare/v0.2.1...v0.2.2)

- `^` gemini 2.5* - Added support for completion_tokens_details.reasoning_tokens
- `.` xai - update model list with grok-3-..

## 2025-04-16 - [v0.2.1](https://github.com/jeremychone/rust-genai/compare/v0.2.0...v0.2.1)

- `-` fix openai adapter to accept `o4-mini`, my matching all model names starting `o1`, `o3` and `o4` to OpenAI Adapter. 

## 2025-04-16 - [v0.2.0](https://github.com/jeremychone/rust-genai/compare/v0.2.0-rc.5...v0.2.0)

- `.` Update version to `0.2.0`

## 2025-04-06 - [v0.2.0-rc.5](https://github.com/jeremychone/rust-genai/compare/v0.2.0-rc.2...v0.2.0-rc.5)

- `!` **API-CHANGE** - Now `client.resolve_service_target(model)` is ASYNC, so, `client.resolve_service_target(model).await`
- `^` `AuthResolver` - Now allow async resolver function/closure (Fn) as well as sync ones
- `^` `ServiceTargetResolver` - Now allow async resolver function/closure (Fn) as well as sync ones
- Now `edition = 2024`

## 2025-03-29 - [v0.2.0-rc.2](https://github.com/jeremychone/rust-genai/compare/v0.2.0-rc.1...v0.2.0-rc.2)

- `+` Add `ChatResponse.provider_model_iden` – This will be the model returned by the provider, or a clone of the one sent if the provider does not return it or if it doesn't match.

## 2025-03-09 - [v0.2.0-rc.1](https://github.com/jeremychone/rust-genai/compare/v0.1.23...v0.2.0-rc.1)

- `+` Anthropic - Support for `cache_control` at the message level
- **API-CHANGES**
  - `chat::MetaUsage` has been renamed to `chat::Usage`
  - `Usage.input_tokens` to `Usage.prompt_tokens` 
  - `Usage.prompt_tokens` to `Usage.completion_tokens`
  - `ChatMessage` now takes an additional property, `options: MessageOptions` with and optional `cache_control` (`CacheControl::Ephemeral`)
  	- This is for the now supported Anthropic caching scheme (which can save 90% on input tokens).
  	- Should be relative transparent when use `ChatMessage::user...` and such. 
  	- Unused on OpenAI APIs/Adapters as it supports it transparently.
  	- Google/Gemini caching is not supported at this point, as it is a totally different scheme (requiring a separate request).

## 2025-02-25 - [v0.1.23](https://github.com/jeremychone/rust-genai/compare/v0.1.22...v0.1.23)

- `-` Anthropic - ensure `claude-3-7-sonnet-latest` uses the 8k max_tokens (revert the logic, only '3-opus' and '3-haiku' get the 4k max_tokens)
  - NOTE: I wish Anthropic max_tokens were optional, and they would take the max by default.

## 2025-02-22 - [v0.1.22](https://github.com/jeremychone/rust-genai/compare/v0.1.21...v0.1.22)

- `+` Tool - Add support Gemini for tool calls and responses (thanks to - [@GustavoWidman](https://github.com/GustavoWidman) - [PR #41](https://github.com/jeremychone/rust-genai/pull/41))
- `*` reqwest - Use rustls-tls now (can add feature later if needed) 
- `.` tokio - narrow tokio features 


## 2025-02-04 - [v0.1.21](https://github.com/jeremychone/rust-genai/compare/v0.1.20...v0.1.21)

- `-` usage - make the details properties public

## 2025-02-03 - [v0.1.20](https://github.com/jeremychone/rust-genai/compare/v0.1.19...v0.1.20)

- `+` `reasoning_content` normalization
  - `deepseek-reasoner` (DeepSeekR1) from response `reasoning_content`
  - For #Ollama/@GroqInc with `ChatOptions` `normalize_reasoning_content: true`, `reasoning_content` will be populated from the `<string>` content.

- `^` `deepseek-reasoner` (DeepSeekR1) support for stream reasoning content.
  - With `ChatOptions` `capture_reasoning_content` to capture/concatenate reasoning chunk stream events.

- `+` **o3mini** with `reasoning_effort` low/medium/high, and `o3-mini-low` (medium/high) model aliases with corresponding reasoning effort.

- `!` API CHANGE (minor) - normalize to `usage.prompt_tokens` `usage.completion_tokens`
  - `usage.prompt_tokens` replaces `usage.input_tokens` and `usage.completion_tokens` replaces `usage.output_tokens`
  - Both `.input_tokens` and `.output_tokens` are still present in `MetaUsage` (though they do not get serialized to JSON)

- `+` Added support for `usage.prompt_tokens_details` and `usage.completion_tokens_details`


## 2025-01-27 - [v0.1.19](https://github.com/jeremychone/rust-genai/compare/v0.1.18...v0.1.19)

- `^` groq - add deepseek-r1-distill-llama-70b to default models

## 2025-01-21 - [v0.1.18](https://github.com/jeremychone/rust-genai/compare/v0.1.17...v0.1.18)

- `^` ollama - add deepseek support (by making deepseek.com model names fixed for now)
  - for now `deepseek-chat`, `deepseek-reasoning`
- `.` groq - Update groq model names
- `.` fix links to c03 examples (#37)
- `-` Fix AdapterKind::as_lower_str for deepseek

## 2025-01-06 - [v0.1.17](https://github.com/jeremychone/rust-genai/compare/v0.1.16...v0.1.17)

- `+` AI Provider - Added DeepSeek

## 2025-01-02 - [v0.1.16](https://github.com/jeremychone/rust-genai/compare/v0.1.15...v0.1.16)

- `.` MessageContent::text_into_string/str return None when Parts (to avoid leak)
- `^` Image support - Add Test, Image update, API Update (constructors, ImageSource variants with data)
- `+` Image Support - Initial (Thanks to [@AdamStrojek](https://github.com/AdamStrojek))
  - For OpenAI, Gemini, Anthropic. (Only OpenAI supports URL images, others require base64)

## 2024-12-08 - [v0.1.15](https://github.com/jeremychone/rust-genai/compare/v0.1.14...v0.1.15)

- `+` add back AdapterKind::default_key_env_name

## 2024-12-08 - `0.1.14`

- `+` adapter - xAI adapter
- `+` **ServiceTargetResolver** added (support for **custom endpoint**) (checkout [examples/c06-starget-resolver.rs](examples/c06-target-resolver.rs))
- `.` ollama - now use openai v1 api to list models
- `.` test - add test for Client::all_model_names
- `*` major internal refactor

## 2024-12-07 - `0.1.13`

- `.` ollama - removed workaround for multi-system lack of support (for old ollama)
- `+` add stop_sequences support cohere
- `+` stop_sequences - for openai, ollama, groq, gemini, cochere
- `+` stop_sequences - for anthropic (thanks [@semtexzv](https://github.com/semtexzv))

## 2024-11-18 - `0.1.12`

- `.` minor update on llms model names
- `^` ChatRole - impl Display
- `^` ChatReqeuust - added from_messages, and append_messages

## 2024-11-04 - `0.1.11`

- `^` anthropic - updated the default max_token to the max for given the model (i.e. 3-5 will be 8k)
- `+` tool - First pass at adding Function Calling for OpenAI and Anthropic (rel #24)
  - **NOTE**: The tool is still work in progress, but this should be a good first start. 
- `.` update version to 0.1.11-WIP

## 2024-10-05 - `0.1.10`

(minor release)

- `^` ChatRequest - add `ChatReqeust::from_user(...)`
- `.` openai - add o1-preview, o1-mini to openai list
- `.` update groq models (llama 3.2)
- `.` Added .github with Github Bug Report template (#26)
- `.` minor readme update to avoid browser issue to scroll down to video section


## 2024-09-18 - `0.1.9`

- `^` AdapterKind - expose default_key_env_name
- `.` openai - add 'o1-' model prefix to point to OpenAI Adapter
- `.` comments proofing (using genai with custom devai script)
- `.` #23 - add documentation
- `.` fix printer comment
- `.` updated to v0.1.9-wip

## 2024-09-06 - `0.1.8`

- `.` printer - now uses printer::Error (rather than box dyn) (rel #21)
- `+` **NEW** - **structured output** - for gemini & OpenAI
  - Behind the scene:
    - <a style="display: inline-block;transform: translateY(4px);"  href="https://www.youtube.com/watch?v=GdFsqLJ1_pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube-Video?style=flat&logo=youtube&color=%23ff0000"></a> Adding **Gemini** Structured Output (vid-0060)
    - <a style="display: inline-block;transform: translateY(4px);"  href="https://www.youtube.com/watch?v=FpoNbQMhAH8&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube-Video?style=flat&logo=youtube&color=%23ff0000"></a> Adding **OpenAI** Structured Output (vid-0059)
- `!` **soft deprecation (for now)** use `ChatResponseFormat::JsonMode` (was `ChatOptions::json_mode` flag) 
- `*` Make most public types `De/Serializable`
- `.` openai - fix chatgpt prefix. Update current model lists
- `.` add json test for anthropic
- `.` makes `webc::Error` public (relates to: #12)

## 2024-08-14 - `0.1.7`

- `+` Added ModelMapper scheme (client_builder::with_model_mapper_fn)
  - <a style="display: inline-block;transform: translateY(4px);"  href="https://www.youtube.com/watch?v=5Enfcwrl7pE&list=PL7r-PXl6ZPcBcLsBdBABOFUuLziNyigqj"><img alt="Static Badge" src="https://img.shields.io/badge/YouTube-Video?style=flat&logo=youtube&color=%23ff0000"></a> - genai ModelMapper code demo (v0.1.7)
- `!` **API CHANGE** Removed `AdapterKindResolver` (should use ModelMapper) (see [examples/c03-mapper.rs](examples/c03-mapper.rs))
- `!` **API CHANGE** Renamed `ModelInfo` to `ModelIden`
- `!` **API CHANGE** `AuthResolver` - Refactor AuthResolver Scheme/API (see [examples/c02-auth.rs](examples/c02-auth.rs))
- `!` **API CHANGE** completely remove `AdapterConfig` (see `AuthResolver`)
- `.` test groq - switch to llama3-groq-8b-8192-tool-use-preview for testing to have the test_chat_json work as expected
- `^` chore: make stream is send
- `.` test - `ChatOptions` - add tests for temperature
- `.` A typo in adapters for OpenAI makes the temperature chat option unusable.
- `.` unit test - first value_ext insert

## 2024-07-26 - `0.1.6`

- `+` ChatOption Add json mode for openai type models
- `.` groq - added the Llama 3.1 previews, and grog-..-tool-use.. to the groq model list names
- `!` now `chat::printer::print_chat_stream` (was `utils::print_chat_stream`)
- `!` Now `ChatOptions` (was `ChatRequestOptions`) ! Remove `client_builder.with_default_chat_request_options` (available with `client_builder.with_chat_options`)
- `.` readme - add youtube videos doc

## 2024-07-21 - `0.1.5`

- `!` **API CHANGE** now ClientBuilder::insert_adapter_config (was with_adapter_config)
- `.` code clean

## 2024-07-19 - `0.1.4`

- `!` **API CHANGE** - refactor Error 
  - With new `ModelInfo` 
  - Back to `genai::Error` (`adapter::Error` was wrongly exposing internal responsibility)
- `.` update tests and examples from 'gpt-3.5-turbo' to 'gpt-4o-mini'
- `-` Fix naming `ClientConfig::with_adapter_kind_resolver` (was wrongly `...auth_resolver`)
- `*` refactor code layout, internal Adapter calls to use ModelInfo 
- `+` Add ModelName and ModelInfo types for better efficient request/error context 
- `!` **API CHANGE** - now `Client::resolve_model_info(model)` (was `Client::resolve_adapter_kind(mode)`)
- `^` `ChatRequest` - add `ChatRequest::from_system`
- `.` updated provider supported list

## 2024-07-18 - `0.1.3`

- `^` **openai** - added `gpt-4o-mini` and switched all openai examples/tests to it
- `!` **API CHANGE** - New `MessageContent` type for `ChatMessage.content`, `ChatResponse.content`, and `StreamEnd.captured_content` (only ::Text variant for now).
  - This is in preparation for multimodal support
- `!` **API CHANGE** - (should be minor, as `Into` implemented) - `ChatMessage` now takes `MessageContent` with only `::Text(String)` variant for now.
- `!` **API CHANGE** - Error refactor - added `genai::adapter::Error` and `genai::resolver::Error`, and updated `genai::Error` with appropriate `Froms`
- `+` **Added token usage** for ALL adapters/providers - `ChatResponse.usage` and `ChatRequestOption` `.capture_usage`/`.capture_content` (for streaming) support for all Adapters (see note in Readme for Ollama for streaming)
- `!` **API CHANGE**: `ClientConfig::with_chat_request_options` (was `with_default_chat_request_options`)
- `!` **API CHANGE**: `PrintChatStreamOptions::from_print_events` (was `from_stream_events`)
- `^` `AdapterKind` - added `as_str` and `as_lower_str`
- `^` `ChatRequest` - added `.iter_systems()` and `.combine_systems()` (includes eventual `chat_req.system` as part of the system messages)
- `!` **API CHANGE**: `Client::all_model_names(..)` (was `Client::list_models(..)`)
- `^` **groq** - add gemma2-9b-it to the list of Groq models
- `!` **API CHANGE**: `genai::Client` (was `genai::client::Client`, same for `ClientBuilder` `ClientConfig`)
- `-` **groq** - remove groq whisper model from list_models as it is not a chat completion model
- `^` **ollama** - implement live list_models for ollama
- `!` Makes AdapterDispatcher crate only (should be internal only)

## 2024-07-08 - `0.1.2`

- `+` `ChatRequestOptions` - added `temperature`, `max_tokens`, `top_p` for all adapters (see readme for property mapping). 
- `!` `SyncAdapterKindResolverFn` - Change signature to return Result<Option<AdapterKind>> (rather than Result<AdapterKind>)
- `.` made public `client.resolve_adapter_kind(model)`
- `+` implement groq completions

## 2024-06-12 - `0.1.1`

- `-` gemini - proper stream message error handling

## 2024-06-11 - `0.1.0`

- `.` print_chat_stream - minor refactor to ensure flush

## 2024-06-10 - `0.0.14`

- `-` ollama - improve Ollama Adapter to support multi system messages
- `-` gemini - fix adapter to set "systemInstruction" (Supported in v1beta)

## 2024-06-10 - `0.0.13`

- `+` Added AdapterKindResolver
- `-` Adapter::list_models api impl and change
- `^` chat_printer - added PrintChatStreamOptions with print_events
