`.` minor | `-` Fix | `+` Addition | `^` improvement | `!` Change | `*` Refactor

> **IMPORTANT:** `0.1.x` will still have some breaking changes in patches.
> - Make sure to **lock** your version, e.g., `genai = "=0.1.7"`.
> - Version `0.2.x` will follow semver more strictly.
> - API changes will be denoted as "`!` - **API CHANGE** ...."

## 2024-08-14 - `0.1.7`

- `+` Added ModelMapper scheme (client_builder::with_model_mapper_fn)
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

- `!` - **API CHANGE** now ClientBuilder::insert_adapter_config (was with_adapter_config)
- `.` - code clean

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