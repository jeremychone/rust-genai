`.` minor | `-` Fix | `+` Addition | `^` improvement | `!` Change | `*` Refactor

## Upcoming - 0.4.0-alpha...

- MORE TO COME...
- `!` API CHANGE, `ContentPart::Binary` now (was `ContentPart::Image`), enabling PDF support  
- `!` API CHANGE, `StreamEnd`, text and tool calls content are now part of Vec  
- `!` API CHANGE, `ChatResponse::tool_calls(&self) -> Vec<&ToolCall>` (instead of `Option<Vec..>`)  
- `!` API CHANGE, `ChatResponse.content` now contains `Vec<MessageContent>`  
- `!` API CHANGE, `MessageContent` now uses `message_content.text()` and `message_content.into_text()` (replacing `text_as_str`, `text_into_string`)  
- `+` API NEW, **Custom HTTP headers** in `ChatOptions` (#78)  
- `+` API NEW, added `ChatResponse.captured_raw_body` (opt-in via `chat_options.with_capture_raw_body(true)`) (PR #68 + refactor)  
- `+` Added Gemini built-in support (PR #67)  
- `+` API NEW, added web configuration support (#66) (manage proxy, timeout, etc.)  
- `+` API NEW, Model **Namespace Support** to force AdapterKind, e.g., `openai::codex-...`  
- `+` NEW ADAPTERS, fireworks.ai and together.ai, with namespaces; fireworks models detected by `fireworks` in model name  
- `+` Added streaming support for tool calls  
- `+` **New Adapters**, Zhipu (ChatGLM) (#76), Nebius  
- `^` API NEW, `ChatResponse` now implements `texts()`, `into_texts()`, `first()`  
- `^` Anthropic, added support for `ChatResponse` multi-content  
- `^` openai, added support for `ChatResponse` multi-content  
- `^` gemini, now uses `x-goog-api-key` header for auth  
- `-` gemini, fixed built-in and user tools issue from #67  
- `-` openai, fixed ServiceTarget issue where custom URL and auth were not used (PR #71)  
- `-` Fixed, improved reasoning content extraction in OpenAIAdapter (#69)  
- `-` gemini, fixed streaming multi-content  
- `-` gemini, fixed incorrect `tool_response.content` JSON parsing (#59)  
- `-` gemini, fixed partial message parsing in Gemini stream (#63)  
- `.` openai, added `codex` prefix for Adapter match  
- `.` gemini model names, updated  
- `.` groq, updated Groq Llama models to `llama-3.1-8b-instant`, `llama-3.3-70b-versatile` (per deprecation notice)

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