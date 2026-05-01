# OpenCode Go Adapter Specification

## Architecture

- **AdapterKind**: `OpenCodeGo`
- **Namespace**: `opencode_go::` (namespace-only access, no prefix-based auto-detection)
- **Default env var**: `OPENCODE_GO_API_KEY`
- **Endpoint**: `https://opencode.ai/zen/go/v1/`

The adapter routes requests to either the OpenAI-compatible endpoint (`/v1/chat/completions`) or the Anthropic-compatible endpoint (`/v1/messages`) depending on the model. 12 models use the OpenAI protocol, 2 MiniMax models use the Anthropic protocol.

## Supported Models (14 total)

### OpenAI Protocol (`/v1/chat/completions`, 12 models)
| Model | Auth Header |
|-------|-------------|
| `glm-5` | `Authorization: Bearer` |
| `glm-5.1` | `Authorization: Bearer` |
| `kimi-k2.5` | `Authorization: Bearer` |
| `kimi-k2.6` | `Authorization: Bearer` |
| `deepseek-v4-pro` | `Authorization: Bearer` |
| `deepseek-v4-flash` | `Authorization: Bearer` |
| `qwen3.5-plus` | `Authorization: Bearer` |
| `qwen3.6-plus` | `Authorization: Bearer` |
| `mimo-v2-pro` | `Authorization: Bearer` |
| `mimo-v2-omni` | `Authorization: Bearer` |
| `mimo-v2.5-pro` | `Authorization: Bearer` |
| `mimo-v2.5` | `Authorization: Bearer` |

### Anthropic Protocol (`/v1/messages`, 2 MiniMax models)
| Model | Auth Header |
|-------|-------------|
| `minimax-m2.5` | `x-api-key` |
| `minimax-m2.7` | `x-api-key` |

## Auth Strategy

- Same API key value for both endpoints, only the header name differs.
- **OpenAI path** (`/v1/chat/completions`): `Authorization: Bearer <key>`
- **Anthropic (MiniMax) path** (`/v1/messages`): `x-api-key: <key>`

Key auth findings from protocol validation:
- `Authorization: Bearer` is REJECTED with 401 on the MiniMax/Anthropic endpoint.
- `anthropic-version` header is NOT required for MiniMax requests.
- `max_tokens` is NOT required for MiniMax requests.

## Response Handling

- **OpenAI models**: Delegate to `OpenAIAdapter::to_chat_response()` / `to_chat_stream()`. Standard OpenAI-compatible format with `reasoning_content` field.
- **MiniMax models**: Delegate to `AnthropicAdapter::to_chat_response()` / `to_chat_stream()`. Uses standard Anthropic SSE format with `thinking` content blocks containing `signature`.

## Model Listing

Models are fetched dynamically via `GET /v1/models` (OpenAI-compatible endpoint, Bearer auth). All 14 models appear in the listing. A hardcoded fallback list is kept as a safety net.

## Payload Construction (MiniMax)

- Use `AnthropicAdapter::into_anthropic_request_parts()` to convert the request.
- Build payload manually (following the Vertex adapter pattern).
- Include: `model`, `messages`, `stream` flag.
- Optional: `system`, `tools`, `temperature`, `top_p`, `stop_sequences`.
- Do NOT include `anthropic-version` in body or as header.
- Do NOT require `max_tokens` (but include it if provided in options).

---

## Protocol Validation Report (2026-05-01)

### Test 1: Non-MiniMax Basic (glm-5, OpenAI endpoint)
**Status**: ✅ PASS (HTTP 200)
**Endpoint**: `POST /v1/chat/completions`
**Auth**: `Authorization: Bearer <key>` ✅
**Response**: Standard OpenAI-compatible format with `reasoning_content` field
**Findings**: 
- Auth with `Authorization: Bearer` works for OpenAI path
- Response includes `reasoning_content` (thinking/reasoning) field
- Model returned as `"frank/GLM-5.1"` (internal routing name, not the request ID)

### Test 2: MiniMax Anthropic Endpoint — Auth Requirements
**Status**: CRITICAL FINDING
**Endpoint**: `POST /v1/messages`

| Test | Auth Header | `anthropic-version` | `max_tokens` | Result |
|------|------------|---------------------|--------------|--------|
| 2a | `Authorization: Bearer` | Yes | Yes | ❌ 401 "Missing API key" |
| 2b | `Authorization: Bearer` | No | Yes | ❌ 401 "Missing API key" |
| 2c | `x-api-key` | Yes | Yes | ✅ 200 OK |
| 2d | `x-api-key` | No | Yes | ✅ 200 OK |
| 2e | `x-api-key` (stream) | No | Yes | ✅ 200 OK |
| 2f | `x-api-key` | No | No | ✅ 200 OK |

**Findings**:
1. ✅ **MiniMax requires `x-api-key` header** — `Authorization: Bearer` is REJECTED with 401
2. ❌ **`anthropic-version` header is NOT required** — works with and without
3. ❌ **`max_tokens` is NOT required** — works with and without
4. ✅ **Response format**: Anthropic-like with `thinking` content blocks containing `signature`

### Test 3: MiniMax Streaming SSE Format
**Status**: ✅ PASS — Standard Anthropic SSE format
**Events observed**:
```
event: message_start
event: ping
event: content_block_start  (thinking block)
event: content_block_delta   (thinking text)
event: content_block_delta   (signature)
event: content_block_stop
event: content_block_stop    (second block)
event: message_delta         (stop_reason, usage)
event: message_stop
event: ping                  (with cost)
```
**Compatibility**: Compatible with `AnthropicStreamer`

### Test 4: Model Listing Endpoint
**Status**: ✅ PASS (HTTP 200)
**Endpoint**: `GET /v1/models`
**Auth**: `Authorization: Bearer <key>` ✅
**All 14 models returned**:
```
[ "minimax-m2.7", "minimax-m2.5", "kimi-k2.6", "kimi-k2.5",
  "glm-5.1", "glm-5", "deepseek-v4-pro", "deepseek-v4-flash",
  "qwen3.6-plus", "qwen3.5-plus", "mimo-v2-pro", "mimo-v2-omni",
  "mimo-v2.5-pro", "mimo-v2.5" ]
```
**Findings**: 
- MiniMax models DO appear in listing
- Response is OpenAI-compatible `{"data": [{"id": "...", ...}]}` format
- No hardcoded fallback needed (but keep as safety net)
