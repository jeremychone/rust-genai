# OpenTelemetry GenAI instrumentation (`otel` feature)

genai can instrument its operations following the
[OpenTelemetry GenAI semantic conventions](https://github.com/open-telemetry/semantic-conventions-genai)
(status: Development). The feature is **off by default**.

```toml
[dependencies]
genai = { version = "0.7", features = ["otel"] }
```

## How it works (tracing bridge)

genai emits plain [`tracing`](https://docs.rs/tracing) spans whose field names
follow the `gen_ai.*` conventions, plus the bridge fields `otel.kind`,
`otel.name`, and `otel.status_code`. It adds **no new dependencies** and never
owns an OpenTelemetry pipeline.

To export them as OpenTelemetry spans, wire
[`tracing-opentelemetry`](https://docs.rs/tracing-opentelemetry) onto your
`tracing-subscriber` registry in the application:

```rust,ignore
use tracing_subscriber::prelude::*;

let tracer = /* your OpenTelemetry tracer (OTLP, stdout, ...) */;
tracing_subscriber::registry()
    .with(tracing_opentelemetry::layer().with_tracer(tracer))
    .init();
```

genai records to whatever subscriber is installed. See
[`examples/c12-otel.rs`](../examples/c12-otel.rs) for a runnable example.

## What is instrumented

| Operation | Auto | Span name | Kind |
|---|---|---|---|
| `Client::exec_chat` | ✅ | `chat {model}` | client |
| `Client::exec_chat_stream` | ✅ | `chat {model}` | client |
| `Client::exec_embed` / `embed` / `embed_batch` | ✅ | `embeddings {model}` | client |

The `chat`/`embeddings` spans cover the full operation (including streaming, to
stream end) and record the request parameters, server address/port, response id
and model, finish reasons, token usage, and — on failure — `error.type` with
`otel.status_code = error`. Streaming spans also record
`gen_ai.response.time_to_first_chunk`.

### Recorded attributes (chat)

- `gen_ai.operation.name`, `gen_ai.provider.name`, `gen_ai.request.model`,
  `gen_ai.request.stream`
- `gen_ai.request.{temperature, max_tokens, top_p, seed, stop_sequences}` (when set)
- `gen_ai.output.type` (when an output format is requested)
- `server.address`, `server.port`
- `gen_ai.response.{id, model, finish_reasons}`
- `gen_ai.usage.{input_tokens, output_tokens, cache_read.input_tokens,
  cache_creation.input_tokens, reasoning.output_tokens}`
- `gen_ai.response.time_to_first_chunk` (streaming)
- `error.type` (on failure)

### Provider names

genai's `AdapterKind` is mapped to the spec's `gen_ai.provider.name` well-known
values where one exists (`openai`, `anthropic`, `gcp.gemini`, `gcp.vertex_ai`,
`aws.bedrock`, `groq`, `cohere`, `deepseek`, `x_ai`, `moonshot_ai`); other
adapters fall back to their lower-case id (the spec permits custom values). See
`genai::otel::provider_name`.

## Content capture (opt-in, off by default)

Prompt/response content is **not** recorded unless explicitly enabled, because
it is likely to contain sensitive / PII data. Enable it with the standard env
var (matching the OpenTelemetry GenAI instrumentations):

```sh
OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT=true
```

When enabled, these attributes are recorded as JSON strings following the spec
message schemas:

- `gen_ai.system_instructions`
- `gen_ai.input.messages`
- `gen_ai.output.messages`
- `gen_ai.tool.definitions`

## Opt-in helpers (agent / workflow / tool / evaluation)

genai is a provider client — it has no agent, workflow, planning, tool
execution, or evaluation machinery, so those signals are **not** emitted
automatically. For applications building such flows on top of genai, the
`genai::otel` module exposes builders that produce spec-compliant spans/events:

```rust,ignore
use genai::otel::agent;

// `execute_tool` span from a genai ToolCall surfaced in a chat response.
for tool_call in chat_res.tool_calls() {
    let span = agent::execute_tool_span(tool_call);
    let _guard = span.enter();
    // ... run the tool ...
    agent::ExecuteToolSpan::record_result(&span, &result_json);
}

// Agent / workflow / plan spans.
let _span = agent::AgentSpan::invoke_agent().with_name("Greeter").start();
let _span = agent::invoke_workflow_span("daily-report");
let _span = agent::plan_span(Some("Greeter"));
```

```rust,ignore
use genai::otel::events::EvalResult;

EvalResult::new("Relevance")
    .with_score_value(4.0)
    .with_score_label("relevant")
    .with_response_id(response_id)
    .emit();
```

## Limitations

- **Spans only.** The spec's metric instruments (`gen_ai.client.token.usage`,
  `gen_ai.client.operation.duration`, ...) are not emitted; the equivalent data
  is available as span attributes, and backends can derive metrics from spans.
- **Array attributes as JSON strings.** `tracing` has no array field type, so
  `gen_ai.response.finish_reasons`, `gen_ai.request.stop_sequences`, and the
  message-content attributes are encoded as JSON strings. The spec permits
  JSON-string encoding on spans.
- **No cost attribute.** The GenAI semantic conventions define token usage but
  no monetary cost attribute; cost is left to downstream tooling.
- **Events as point-in-time spans.** Spec "events" (e.g.
  `gen_ai.evaluation.result`) are emitted as zero-duration spans carrying the
  event's attributes — the faithful representation in a tracing → OpenTelemetry
  bridge.
- The GenAI semantic conventions are at **Development** status; attribute names
  may still change. They are centralized in `genai::otel::conventions`.
