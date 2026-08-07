## webc

### Goal

The `webc` module provides a low-level, internal web client layer utilizing `reqwest`. Its primary role is to abstract standard HTTP requests (GET/POST) and manage complex streaming responses required by various AI providers, especially those that do not fully conform to the Server-Sent Events (SSE) standard (`text/event-stream`). It handles standard JSON requests/responses and custom stream parsing.

### Public Module API

The `webc` module is primarily an internal component, only exposing its dedicated error type publicly.

- `pub use error::Error;`
    - `Error`: An enum representing all possible errors originating from the web communication layer (e.g., failed status codes, JSON parsing errors, reqwest errors, stream clone errors).

(All other types like `WebClient`, `WebResponse`, `WebStream`, and `Result` are exported as `pub(crate)` for internal library use.)

### Module Parts

The module consists of three main internal components:

- `error.rs`: Defines the `Error` enum and the module-scoped `Result<T>` type alias. It captures network/HTTP related failures and external errors like `reqwest::Error` and `value_ext::JsonValueExtError`.

- `web_client.rs`: Contains the `WebClient` struct, a thin wrapper around `reqwest::Client`. It provides methods (`do_get`, `do_post`) for non-streaming standard HTTP communication, which assumes the response body is JSON and is parsed into `serde_json::Value`. `do_post_with_observer` is the observed variant used by the chat exec path: it fires the (optional) crate-internal `BoundResponseObserver` on the response head (status + headers) right after `send()` resolves, before the body is consumed and before the status check — `do_post` simply delegates with no observer. It also defines `WebResponse`, which encapsulates the HTTP status and parsed JSON body.

- `web_stream.rs`: Implements `WebStream`, a custom `futures::Stream` implementation designed for handling non-SSE streaming protocols used by some AI providers (e.g., Cohere, Gemini). It defines `StreamMode` to specify how stream chunks should be parsed (either by a fixed delimiter or specialized handling for "Pretty JSON Array" formats). `WebStream` (and the `EventSourceStream` wrapping it) can carry an optional `BoundResponseObserver` (set via `with_response_observer`, threaded down from `Adapter::to_chat_stream`): since the HTTP send is lazy (performed on the first poll), the observer is awaited inside the send future as soon as the `reqwest::Response` head is in hand — before the success/failure status check, so it also fires on 4xx/5xx responses.

### Key Design Considerations

- **Internal Focus:** The module is designed strictly for internal use (`pub(crate)`) except for the public error type. This shields the rest of the library from direct `reqwest` dependency details.

- **Custom Streaming:** `WebStream` exists specifically to manage streaming protocols that deviate from the standard SSE format, providing message splitting based on `StreamMode`. This ensures compatibility with providers like Cohere (delimiter-based) and Gemini (JSON array chunking).

- **Generic JSON Response Handling:** `WebResponse` abstracts successful non-streaming responses by immediately parsing the body into `serde_json::Value`. This allows adapter modules to deserialize into their specific structures subsequently.

- **Error Richness:** The `Error::ResponseFailedStatus` variant includes the `StatusCode`, full `body`, and `HeaderMap` to provide comprehensive debugging information upon API failure. On the streaming path, `WebStream` mirrors this: when the lazy request send resolves to a non-success status, it surfaces a crate-level `genai::Error::HttpError` carrying the status, canonical reason, body, and response `HeaderMap` (captured while the `reqwest::Response` is still in hand), so retry-relevant headers such as `retry-after`, `retry-after-ms`, and `x-should-retry` remain visible to downstream retry layers.

- **Response Observation:** The per-request `ResponseObserver` exec hook (defined in the `client` module) reaches this layer as a `BoundResponseObserver` (observer + `ModelIden`). It fires exactly once per request on the response head — in `do_post_with_observer` for non-streaming calls, and inside `WebStream`'s send future (or the Bedrock byte-stream equivalent) for streaming calls — always before body consumption and before the status check, so failing responses are observed too (in addition to yielding the headers-carrying error described above). When no observer is configured, the code paths are unchanged.

- **Async Implementation:** All network operations rely on `tokio` and `reqwest`, ensuring non-blocking execution throughout the I/O layer. `WebStream` leverages `futures::Stream` traits for integration with standard Rust async infrastructure.
