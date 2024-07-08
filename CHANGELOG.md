`.` minor | `-` Fix | `+` Addition | `^` improvement | `!` Change | `*` Refactor

## 2024-07-08 - `0.1.2`

- `+` `ChatRequestOptions` - added `temperature`, `max_tokens`, `top_k` for all adapters (see readme for property mapping). 
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