## client

### Goal

The `client` module provides the core entry point (`Client`) for interacting with various Generative AI providers. It encapsulates configuration (`ClientConfig`, `WebConfig`), a builder pattern (`ClientBuilder`), request execution (`exec_chat`, `exec_embed`), and service resolution logic (e.g., determining endpoints and authentication).

### Public Module API

The `client` module exposes the following public types:

- **`Client`**: The main interface for executing AI requests (chat, embedding, streaming, model listing).
  - `Client::builder()`: Starts the configuration process.
  - `Client::default()`: Creates a client with default configuration.
  - Core execution methods: `exec_chat`, `exec_chat_stream`, `exec_embed`, `embed`, `embed_batch`.
  - Resolution/Discovery methods: `all_model_names`, `resolve_service_target`.

- **`ClientBuilder`**: Provides a fluent interface for constructing a `Client`. Used to set `ClientConfig`, default `ChatOptions`, `EmbedOptions`, custom resolvers (`AuthResolver`, `ServiceTargetResolver`, `ModelMapper`), and per-request exec hooks (`PayloadInterceptor`, `ResponseObserver`).

- **`ClientConfig`**: Holds the resolved and default configurations used by the `Client`, including resolver functions, exec hooks, and default options.

- **`PayloadInterceptor`**: Per-request exec hook (resolver idiom: enum with sync `InterceptorFn` and async `InterceptorAsyncFn` variants, created via `from_interceptor_fn` / `from_interceptor_async_fn`). Called on each chat exec call (streaming and non-streaming) with the target `ModelIden` and the serialized provider payload (`serde_json::Value`), after `to_web_request_data` and before the HTTP request is built. Returning `Some(value)` replaces the payload sent over the wire; `None` keeps it unchanged (the payload is cloned once per request only when an interceptor is set).

- **`ResponseObserver`**: Per-request exec hook (same idiom, `from_observer_fn` / `from_observer_async_fn`). Called on each chat exec call with the target `ModelIden`, the response `StatusCode`, and the response `HeaderMap` as soon as the HTTP response arrives and before its body/stream is consumed — including on 4xx/5xx responses. Chat exec paths only (embeddings and model listing are not hooked).

- **`Headers`**: A simple map wrapper (`HashMap<String, String>`) for managing HTTP headers in requests.

- **`ServiceTarget`**: A struct containing the final resolved components needed to execute a request: `Endpoint`, `AuthData`, and `ModelIden`.

- **`WebConfig`**: Configuration options specifically for building the underlying `reqwest::Client` (e.g., timeouts, proxies, default headers).

### Module Parts

The module is composed of several files that implement the layered client architecture:

- `builder.rs`: Implements `ClientBuilder`, handling the creation and configuration flow. It initializes or updates the nested `ClientConfig` and optionally an internal `WebClient`.

- `client_types.rs`: Defines the main `Client` struct and `ClientInner` (which holds `WebClient` and `ClientConfig` behind an `Arc`).

- `config.rs`: Defines `ClientConfig` and the core `resolve_service_target` logic, which orchestrates calls to `ModelMapper`, `AuthResolver`, and `ServiceTargetResolver` before falling back to adapter defaults.

- `client_impl.rs`: Contains the main implementation of the public API methods on `Client`, such as `exec_chat` and `exec_embed`. These methods perform service resolution and delegate to `AdapterDispatcher` for request creation and response parsing. The chat exec paths also apply the exec hooks: the `PayloadInterceptor` runs between `to_web_request_data` and the request-builder construction (which makes the `exec_chat_stream` setup an async block), and the `ResponseObserver` is bound to the request's `ModelIden` as a crate-internal `BoundResponseObserver` that is handed to `WebClient::do_post_with_observer` (non-streaming) or threaded through `Adapter::to_chat_stream` into the web stream (streaming).

- `exec_hooks.rs`: Defines the per-request exec hooks `PayloadInterceptor` and `ResponseObserver` (with their sync/async function traits and `Into*` conversion traits, mirroring `AuthResolver`), plus the crate-internal `BoundResponseObserver` pairing an observer with the in-flight request's `ModelIden` so the web layer can fire it without knowing about model resolution.

- `headers.rs`: Implements the `Headers` utility for managing key-value HTTP header maps.

- `service_target.rs`: Defines the `ServiceTarget` structure for resolved endpoints, authentication, and model identifiers.

- `web_config.rs`: Defines `WebConfig` and its logic for applying settings to a `reqwest::ClientBuilder`.

### Key Design Considerations

- **Client Immutability and Sharing**: The `Client` holds its internal state (`ClientInner` with `WebClient` and `ClientConfig`) wrapped in an `Arc`. This design ensures that the client is thread-safe and cheaply cloneable, aligning with common client patterns in asynchronous Rust applications.

- **Config Layering and Resolution**: The client architecture employs a sophisticated resolution process managed by `ClientConfig::resolve_service_target`.
  - It first applies a `ModelMapper` to potentially translate the input model identifier.
  - It then consults the `AuthResolver` for authentication data. If the resolver is absent or returns `None`, it defaults to the adapter's standard authentication mechanism (e.g., API key headers).
  - It determines the adapter's default endpoint.
  - Finally, it applies the optional `ServiceTargetResolver`, allowing users to override the endpoint, auth, or model for complex scenarios (e.g., custom proxies or routing).

- **WebClient Abstraction**: The core HTTP client logic is delegated to the `WebClient` (from the `webc` module), which handles low-level request execution and streaming setup. This separation keeps the `client` module focused on business logic and AI provider orchestration.

- **Builder Pattern for Configuration**: `ClientBuilder` enforces configuration before client creation, simplifying object construction and ensuring necessary dependencies are set up correctly.

- **Headers Simplification**: The `Headers` struct abstracts HTTP header management, ensuring that subsequent merges or overrides result in a single, final header value, which is typical for API key authorization overrides.

- **Exec Hooks (Per-Request Observability/Interception)**: `PayloadInterceptor` and `ResponseObserver` give downstream runtimes request auditing and payload-shaping without changing the serialized `ChatOptions` (which stays hooks-free since it is Serialize/Deserialize). On the streaming path, the HTTP send is lazy (performed on the first stream poll inside `WebStream`), so the observer is carried into the stream and fires when the send resolves — before the status check, so it also fires on failing responses (alongside the headers-carrying `Error::HttpError`). The synthetic `Event::Open` of the SSE stream is emitted before any HTTP activity and is deliberately not tied to the observer.
