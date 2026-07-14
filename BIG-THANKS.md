# Big Thanks to

A huge thank you to all the contributors. The `genai` crate is so lucky, we only get 100% PR gold and 0% PR slop.

_If I forgot your PR, feel free to submit a PR_

## v0.7.0-beta.x

- [ropoctl](https://github.com/ropoctl)
  - [#254](https://github.com/jeremychone/rust-genai/pull/254) anthropic: pass custom content parts through
  - [#255](https://github.com/jeremychone/rust-genai/pull/255) anthropic: merge extra request body fields
  - [#257](https://github.com/jeremychone/rust-genai/pull/257) feat(gemini): forward JSON Schema raw via responseJsonSchema / parametersJsonSchema
- [coaoac](https://github.com/coaoac)
  - [#247](https://github.com/jeremychone/rust-genai/pull/247) feat(anthropic): support fine-grained tool streaming (eager_input_streaming)
  - [#253](https://github.com/jeremychone/rust-genai/pull/253) fix: ReasoningEffort::Zero (rename from ::None) disables reasoning on Anthropic (#251)
- [lambdabetaeta](https://github.com/lambdabetaeta)
  - [#249](https://github.com/jeremychone/rust-genai/pull/249) fix: reuse Client WebClient for model listing
  - [#250](https://github.com/jeremychone/rust-genai/pull/250) fix: support adaptive thinking for Claude Sonnet 5
- [Alb-O](https://github.com/Alb-O)
  - [#243](https://github.com/jeremychone/rust-genai/pull/243) derive PartialEq/Eq for Usage and nested structs
- [holovskyi](https://github.com/holovskyi) 
  - [#245](https://github.com/jeremychone/rust-genai/pull/245) fix(anthropic): propagate mid-stream error events
- [anagrius](https://github.com/anagrius)
  - [#244](https://github.com/jeremychone/rust-genai/pull/244) feat(otel): optional OpenTelemetry GenAI instrumentation (feature `otel`)
- [binyangzhu000-sudo](https://github.com/binyangzhu000-sudo)
  - [#259](https://github.com/jeremychone/rust-genai/pull/259) add Atlas Cloud OpenAI-compatible adapter
- [Jackkakaya](https://github.com/Jackkakaya)
  - [#258](https://github.com/jeremychone/rust-genai/pull/258) fix(anthropic): capture streaming cache tokens from `message_delta` fallback
- [J-F-Liu](https://github.com/J-F-Liu) 
  - [#262](https://github.com/jeremychone/rust-genai/pull/262) move messages after tools in JSON payloads for better prompt cache utilization

## v0.6.0..0.6.x

- [ropoctl](https://github.com/ropoctl)
  - [#237](https://github.com/jeremychone/rust-genai/pull/237) anthropic adaptive thinking for Opus 4.7+
  - [#236](https://github.com/jeremychone/rust-genai/pull/236) openai_resp tolerate response.completed without output field
- [coaoac](https://github.com/coaoac)
  - [#239](https://github.com/jeremychone/rust-genai/pull/239) selectable reqwest TLS backends with rustls default
- [kawayiYokam](https://github.com/kawayiYokami)
  - [#242](https://github.com/jeremychone/rust-genai/pull/242) fix: capture OpenAI stream usage tail

## v0.6.0

- [ropoctl](https://github.com/ropoctl)
  - [#227](https://github.com/jeremychone/rust-genai/pull/227) OpenAI Responses encrypted reasoning round-trip
  - [#220](https://github.com/jeremychone/rust-genai/pull/220) gemini yakbak tool_stream cassette and replay test
  - [#212](https://github.com/jeremychone/rust-genai/pull/212) client bind a Client to one adapter
  - [#211](https://github.com/jeremychone/rust-genai/pull/211) fix capture reasoning without effort
  - [#205](https://github.com/jeremychone/rust-genai/pull/205) use BTreeMap for deterministic tool-call ordering in openai_resp streamer
  - [#200](https://github.com/jeremychone/rust-genai/pull/200) add 'strict' option for OpenAI strict function calling
  - [#192](https://github.com/jeremychone/rust-genai/pull/192) gate `reasoning.encrypted_content` on `capture_reasoning_content`
  - [#191](https://github.com/jeremychone/rust-genai/pull/191) make `reasoning.summary` opt-in for `capture_reasoning_content`
  - [#190](https://github.com/jeremychone/rust-genai/pull/190) make Gemini `thinkingConfig/includeThoughts` opt-in for `capture_reasoning_content`
  - [#188](https://github.com/jeremychone/rust-genai/pull/188) add yakbak Gemini streaming replay test
  - [#182](https://github.com/jeremychone/rust-genai/pull/182) add yakbak HTTP record/replay integration test infrastructure
  - [#145](https://github.com/jeremychone/rust-genai/pull/145) ToolName and ToolConfig serde round-trip fix
- [chaizhenhua](https://github.com/chaizhenhua)
  - [#223](https://github.com/jeremychone/rust-genai/pull/223) normalize Gemini schema and tool calling
  - [#199](https://github.com/jeremychone/rust-genai/pull/199) fix(openai-streamer) emit ToolCallChunk with finish_reason
  - [#195](https://github.com/jeremychone/rust-genai/pull/195) Anthropic incremental ToolCallChunk streaming
  - [#187](https://github.com/jeremychone/rust-genai/pull/187) buffer incomplete UTF-8 sequences across stream chunks
  - [#186](https://github.com/jeremychone/rust-genai/pull/186) capture inline usage from OpenAI finish_reason stream chunks
  - [#185](https://github.com/jeremychone/rust-genai/pull/185) Anthropic null tool_call arguments serialization fix
  - [#173](https://github.com/jeremychone/rust-genai/pull/173) Gemini parallel streaming tool calls fix
  - [#162](https://github.com/jeremychone/rust-genai/pull/162) Gemini JSON Schema compatibility simplification
  - [#161](https://github.com/jeremychone/rust-genai/pull/161) Gemini usage-only stream tail fix
  - [#160](https://github.com/jeremychone/rust-genai/pull/160) expose provider stop_reason in chat responses
  - [#159](https://github.com/jeremychone/rust-genai/pull/159) OpenAI SSE error payload surfacing in streaming
- [xiao-e-yun](https://github.com/xiao-e-yun) 
  - [#230](https://github.com/jeremychone/rust-genai/pull/230) add OpenRouter adapter support
  - [#219](https://github.com/jeremychone/rust-genai/pull/219) refactor gemini adapter reasoning effort support
- [RouHim](https://github.com/RouHim) 
  - [#228](https://github.com/jeremychone/rust-genai/pull/228) add AIHubMix adapter support
  - [#214](https://github.com/jeremychone/rust-genai/pull/214) add OpenCodeGo adapter support
  - [#204](https://github.com/jeremychone/rust-genai/pull/204) refactor(ollama) extract shared functions
  - [#198](https://github.com/jeremychone/rust-genai/pull/198) add GitHub Copilot (GitHub Models API) adapter
- [fortunto2](https://github.com/fortunto2) 
  - [#177](https://github.com/jeremychone/rust-genai/pull/177) HTTP optimizations, gzip, TCP_NODELAY, and HTTP/2 tuning
  - [#168](https://github.com/jeremychone/rust-genai/pull/168) stateful sessions support
  - [#169](https://github.com/jeremychone/rust-genai/pull/169) OpenAIResp `instructions` system prompt support
  - [#167](https://github.com/jeremychone/rust-genai/pull/167) GPT-5 Responses API compatibility fixes
  - [#163](https://github.com/jeremychone/rust-genai/pull/163) Gemini function calling protocol fixes
- [clayrosenthal](https://github.com/clayrosenthal) - [#215](https://github.com/jeremychone/rust-genai/pull/215) add support for bedrock inference
- [narcilee7](https://github.com/narcilee7) - [#221](https://github.com/jeremychone/rust-genai/pull/221) add Moonshot AI adapter
- [zhangzhenhu](https://github.com/zhangzhenhu) - [#216](https://github.com/jeremychone/rust-genai/pull/216) add chat extra_body passthrough
- [BinaryMuse](https://github.com/BinaryMuse) 
  - [#156](https://github.com/jeremychone/rust-genai/pull/156) thread auth resolver through `all_model_names()`
  - [#154](https://github.com/jeremychone/rust-genai/pull/154) add `reasoning_content` content part
- [aldiyen](https://github.com/aldiyen) 
  - [#194](https://github.com/jeremychone/rust-genai/pull/194) add JSON schema support in Anthropic adapter
  - [#184](https://github.com/jeremychone/rust-genai/pull/184) add Google Vertex adapter with Gemini and Anthropic support
- [Borber](https://github.com/Borber) 
  - [#166](https://github.com/jeremychone/rust-genai/pull/166) OpenAI prompt cache options in ChatOptions
  - [#165](https://github.com/jeremychone/rust-genai/pull/165) add xhigh reasoning effort mapping
  - [#164](https://github.com/jeremychone/rust-genai/pull/164) keep delta content co-located with finish_reason in openai-streamer
- [vagmi](https://github.com/vagmi) 
  - [#232](https://github.com/jeremychone/rust-genai/pull/232) use Gemini returned `call_id` for function calling
  - [#218](https://github.com/jeremychone/rust-genai/pull/218) simplify Gemini streamer
  - [#141](https://github.com/jeremychone/rust-genai/pull/141) add streaming support for the OpenAI Responses API
  - [#125](https://github.com/jeremychone/rust-genai/pull/125) add separate reasoning content and thought signature for Anthropic messages API
  - [#121](https://github.com/jeremychone/rust-genai/pull/121) include thoughts and capture thoughts as reasoning content
- [vintcessun](https://github.com/vintcessun) 
  -  [#146](https://github.com/jeremychone/rust-genai/pull/146) Gemini image generation and binary response parsing
  - [#144](https://github.com/jeremychone/rust-genai/pull/144) Ollama native API support
  - [#142](https://github.com/jeremychone/rust-genai/pull/142) Gemini tool serialization camelCase fix
- [Himmelschmidt](https://github.com/Himmelschmidt) 
  - [#111](https://github.com/jeremychone/rust-genai/pull/111) Gemini `responseJsonSchema` support
  - [#103](https://github.com/jeremychone/rust-genai/pull/103) capture response body in ResponseFailedNotJson error
- [wangxuwei](https://github.com/wangxuwei) 
  - [#222](https://github.com/jeremychone/rust-genai/pull/222) add support for baidu provider
  - [#143](https://github.com/jeremychone/rust-genai/pull/143) add aliyun provider
- [Dridus](https://github.com/Dridus) - [#136](https://github.com/jeremychone/rust-genai/pull/136) fix `MessageContent::joined_texts` for multiple text parts
- [holovskyi](https://github.com/holovskyi) -  [#130](https://github.com/jeremychone/rust-genai/pull/130) Anthropic prompt caching fixes
- [BinaryMuse](https://github.com/BinaryMuse) - [#126](https://github.com/jeremychone/rust-genai/pull/126) add `ModelSpec` for additional model call details
- [anagrius](https://github.com/anagrius) - [#119](https://github.com/jeremychone/rust-genai/pull/119) use output_text for openai_resp assistant content
- [vlmutolo](https://github.com/vlmutolo) - [#115](https://github.com/jeremychone/rust-genai/pull/115) inject skip_thought_signature_validator into Gemini 3 tool call thoughtSignature
- [mengdehong](https://github.com/mengdehong) - [#108](https://github.com/jeremychone/rust-genai/pull/108) Ollama reasoning streaming fix
- [Akagi201](https://github.com/Akagi201) - [#105](https://github.com/jeremychone/rust-genai/pull/105) add MIMO model adapter

## v0.5.1

- [anagrius](https://github.com/anagrius) for [#119](https://github.com/jeremychone/rust-genai/pull/119) openai_resp assistant content fix
- [BinaryMuse](https://github.com/BinaryMuse) for [#117](https://github.com/jeremychone/rust-genai/pull/117) WebStream status check and [#116](https://github.com/jeremychone/rust-genai/pull/116) extra headers fix
- [vlmutolo](https://github.com/vlmutolo) for [#115](https://github.com/jeremychone/rust-genai/pull/115) Gemini 3 tool thoughtSignature fix

## v0.5.x

- [BinaryMuse](https://github.com/BinaryMuse) for [#114](https://github.com/jeremychone/rust-genai/pull/114) Anthropic ToolCalls streaming fix
- [Himmelschmidt](https://github.com/Himmelschmidt)
  - [#111](https://github.com/jeremychone/rust-genai/pull/111) Gemini `responseJsonSchema` support
  - [#103](https://github.com/jeremychone/rust-genai/pull/103) error body capture, and Gemini Thought signatures
- [malyavi-nochum](https://github.com/malyavi-nochum) for [#109](https://github.com/jeremychone/rust-genai/pull/109) Fireworks default streaming fix
- [mengdehong](https://github.com/mengdehong) for [#108](https://github.com/jeremychone/rust-genai/pull/108) Ollama reasoning streaming fix
- [Akagi201](https://github.com/Akagi201) for [#105](https://github.com/jeremychone/rust-genai/pull/105) MIMO model adapter

## v0.1.x .. v0.4.x

- [Vagmi Mudumbai](https://github.com/vagmi) for [#96](https://github.com/jeremychone/rust-genai/pull/96) openai audio_type
- [Himmelschmidt](https://github.com/Himmelschmidt) for [#98](https://github.com/jeremychone/rust-genai/pull/98) openai service_tier
- [Bart Carroll](https://github.com/bartCarroll) for [#91](https://github.com/jeremychone/rust-genai/pull/91) Fixed streaming tool calls for openai models
- [Rui Andrada](https://github.com/shingonoide) for [#95](https://github.com/jeremychone/rust-genai/pull/95) refactoring ZHIPU adapter to ZAI
- [Adrien](https://github.com/XciD) Extra headers in requests, seed for chat requests, and fixes (with [Julien Chaumond](https://github.com/julien-c) for extra headers)
- [Andrew Rademacher](https://github.com/AndrewRademacher) for PDF support, Anthropic streamer
- [Jesus Santander](https://github.com/jsantanders) Embedding support [PR #83](https://github.com/jeremychone/rust-genai/pull/83)
- [4t145](https://github.com/4t145) for raw body capture [PR #68](https://github.com/jeremychone/rust-genai/pull/68)
- [Vagmi Mudumbai](https://github.com/vagmi) exec_chat bug fix [PR #86](https://github.com/jeremychone/rust-genai/pull/86)
- [Maximilian Goisser](https://github.com/hobofan) Fix OpenAI adapter to use ServiceTarget
- [ClanceyLu](https://github.com/ClanceyLu) for tool use streaming support, web configuration support, and fixes
- [@SilasMarvin](https://github.com/SilasMarvin) for fixing content/tools issues with some Ollama models [PR #55](https://github.com/jeremychone/rust-genai/pull/55)
- [@una-spirito](https://github.com/luna-spirito) for Gemini `ReasoningEffort::Budget` support
- [@jBernavaPrah](https://github.com/jBernavaPrah) for adding tracing (it was long overdue). [PR #45](https://github.com/jeremychone/rust-genai/pull/45)
- [@GustavoWidman](https://github.com/GustavoWidman) for the initial Gemini tool/function support! [PR #41](https://github.com/jeremychone/rust-genai/pull/41)
- [@AdamStrojek](https://github.com/AdamStrojek) for initial image support [PR #36](https://github.com/jeremychone/rust-genai/pull/36)
- [@semtexzv](https://github.com/semtexzv) for `stop_sequences` Anthropic support [PR #34](https://github.com/jeremychone/rust-genai/pull/34)
- [@omarshehab221](https://github.com/omarshehab221) for de/serialize on structs [PR #19](https://github.com/jeremychone/rust-genai/pull/19)
- [@tusharmath](https://github.com/tusharmath) for making webc::Error [PR #12](https://github.com/jeremychone/rust-genai/pull/12)
- [@giangndm](https://github.com/giangndm) for making stream Send [PR #10](https://github.com/jeremychone/rust-genai/pull/10)
- [@stargazing-dino](https://github.com/stargazing-dino) for [PR #2](https://github.com/jeremychone/rust-genai/pull/2), implement Groq completions
