//! Macros for generating pass-through adapter implementations.

/// Generates a full `impl Adapter` block that delegates to a target protocol adapter.
///
/// # Parameters
///
/// - `name`: The adapter struct name (e.g., `MiniMaxAdapter`).
/// - `kind`: The `AdapterKind` variant for this adapter (used in unsupported-feature errors).
/// - `key_env`: The default API key environment variable name as `Option<&'static str>`.
/// - `endpoint`: The default endpoint URL as a string literal.
/// - `delegate`: The target adapter type to which all trait methods are forwarded.
/// - `unsupported`: (Optional) A list of features not supported. Currently only `embeddings` is recognized.
///
/// # Example
///
/// ```ignore
/// impl_pass_through_adapter!(
///     name: MiniMaxAdapter,
///     kind: AdapterKind::MiniMax,
///     key_env: Some("MINIMAX_API_KEY"),
///     endpoint: "https://api.minimax.io/anthropic/v1/",
///     delegate: crate::adapter::adapters::anthropic::AnthropicAdapter,
///     unsupported: [embeddings],
/// );
/// ```
///
/// NOTE: Eventually, this macro might be further simplified with paste
#[macro_export]
macro_rules! impl_pass_through_adapter {
	// ----- branch: no unsupported list -----
	(
        name: $name:ident,
        kind: $kind:expr,
        key_env: $key_env:expr,
        endpoint: $endpoint:expr,
        delegate: $delegate:ty $(,)?
    ) => {
		impl $crate::adapter::Adapter for $name {
			const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = $key_env;

			fn default_auth(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::AuthData {
				match Self::DEFAULT_API_KEY_ENV_NAME {
					Some(env_name) => $crate::resolver::AuthData::from_env(env_name),
					None => $crate::resolver::AuthData::None,
				}
			}

			fn default_endpoint(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::Endpoint {
				$crate::resolver::Endpoint::from_static($endpoint)
			}

			async fn all_model_names(
				kind: $crate::adapter::AdapterKind,
				endpoint: $crate::resolver::Endpoint,
				auth: $crate::resolver::AuthData,
				web_client: &$crate::webc::WebClient,
			) -> $crate::Result<Vec<String>> {
				<$delegate as $crate::adapter::Adapter>::all_model_names(kind, endpoint, auth, web_client).await
			}

			fn get_service_url(
				model_iden: &$crate::ModelIden,
				service_type: $crate::adapter::ServiceType,
				endpoint: $crate::resolver::Endpoint,
			) -> $crate::Result<String> {
				<$delegate as $crate::adapter::Adapter>::get_service_url(model_iden, service_type, endpoint)
			}

			fn to_web_request_data(
				service_target: $crate::ServiceTarget,
				service_type: $crate::adapter::ServiceType,
				chat_req: $crate::chat::ChatRequest,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				<$delegate as $crate::adapter::Adapter>::to_web_request_data(
					service_target,
					service_type,
					chat_req,
					options_set,
				)
			}

			fn to_chat_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_response(model_iden, web_response, options_set)
			}

			fn to_chat_stream(
				model_iden: $crate::ModelIden,
				reqwest_builder: ::reqwest::RequestBuilder,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatStreamResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_stream(model_iden, reqwest_builder, options_set)
			}

			fn to_embed_request_data(
				service_target: $crate::ServiceTarget,
				embed_req: $crate::embed::EmbedRequest,
				options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				<$delegate as $crate::adapter::Adapter>::to_embed_request_data(service_target, embed_req, options_set)
			}

			fn to_embed_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::embed::EmbedResponse> {
				<$delegate as $crate::adapter::Adapter>::to_embed_response(model_iden, web_response, options_set)
			}
		}
	};

	// ----- branch: unsupported list contains `embeddings` (exact) -----
	(
        name: $name:ident,
        kind: $kind:expr,
        key_env: $key_env:expr,
        endpoint: $endpoint:expr,
        delegate: $delegate:ty,
        unsupported: [embeddings $(,)?] $(,)?
    ) => {
		impl $crate::adapter::Adapter for $name {
			const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = $key_env;

			fn default_auth(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::AuthData {
				match Self::DEFAULT_API_KEY_ENV_NAME {
					Some(env_name) => $crate::resolver::AuthData::from_env(env_name),
					None => $crate::resolver::AuthData::None,
				}
			}

			fn default_endpoint(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::Endpoint {
				$crate::resolver::Endpoint::from_static($endpoint)
			}

			async fn all_model_names(
				kind: $crate::adapter::AdapterKind,
				endpoint: $crate::resolver::Endpoint,
				auth: $crate::resolver::AuthData,
				web_client: &$crate::webc::WebClient,
			) -> $crate::Result<Vec<String>> {
				<$delegate as $crate::adapter::Adapter>::all_model_names(kind, endpoint, auth, web_client).await
			}

			fn get_service_url(
				model_iden: &$crate::ModelIden,
				service_type: $crate::adapter::ServiceType,
				endpoint: $crate::resolver::Endpoint,
			) -> $crate::Result<String> {
				<$delegate as $crate::adapter::Adapter>::get_service_url(model_iden, service_type, endpoint)
			}

			fn to_web_request_data(
				service_target: $crate::ServiceTarget,
				service_type: $crate::adapter::ServiceType,
				chat_req: $crate::chat::ChatRequest,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				<$delegate as $crate::adapter::Adapter>::to_web_request_data(
					service_target,
					service_type,
					chat_req,
					options_set,
				)
			}

			fn to_chat_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_response(model_iden, web_response, options_set)
			}

			fn to_chat_stream(
				model_iden: $crate::ModelIden,
				reqwest_builder: ::reqwest::RequestBuilder,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatStreamResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_stream(model_iden, reqwest_builder, options_set)
			}

			fn to_embed_request_data(
				_service_target: $crate::ServiceTarget,
				_embed_req: $crate::embed::EmbedRequest,
				_options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				Err($crate::Error::AdapterNotSupported {
					adapter_kind: $kind,
					feature: "embeddings".to_string(),
				})
			}

			fn to_embed_response(
				_model_iden: $crate::ModelIden,
				_web_response: $crate::webc::WebResponse,
				_options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::embed::EmbedResponse> {
				Err($crate::Error::AdapterNotSupported {
					adapter_kind: $kind,
					feature: "embeddings".to_string(),
				})
			}
		}
	};
	// ----- branch: all_model_names (custom) + unsupported: [embeddings] -----
	(
        name: $name:ident,
        kind: $kind:expr,
        key_env: $key_env:expr,
        endpoint: $endpoint:expr,
        delegate: $delegate:ty,
        all_model_names: $all_fn:expr,
        unsupported: [embeddings $(,)?] $(,)?

    ) => {
		impl $crate::adapter::Adapter for $name {
			const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = $key_env;

			fn default_auth(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::AuthData {
				match Self::DEFAULT_API_KEY_ENV_NAME {
					Some(env_name) => $crate::resolver::AuthData::from_env(env_name),
					None => $crate::resolver::AuthData::None,
				}
			}

			fn default_endpoint(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::Endpoint {
				$crate::resolver::Endpoint::from_static($endpoint)
			}

			async fn all_model_names(
				kind: $crate::adapter::AdapterKind,
				endpoint: $crate::resolver::Endpoint,
				auth: $crate::resolver::AuthData,
				web_client: &$crate::webc::WebClient,
			) -> $crate::Result<Vec<String>> {
				let __all_fn = $all_fn;
				__all_fn(kind, endpoint, auth, web_client)
			}

			fn get_service_url(
				model_iden: &$crate::ModelIden,
				service_type: $crate::adapter::ServiceType,
				endpoint: $crate::resolver::Endpoint,
			) -> $crate::Result<String> {
				<$delegate as $crate::adapter::Adapter>::get_service_url(model_iden, service_type, endpoint)
			}

			fn to_web_request_data(
				service_target: $crate::ServiceTarget,
				service_type: $crate::adapter::ServiceType,
				chat_req: $crate::chat::ChatRequest,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				<$delegate as $crate::adapter::Adapter>::to_web_request_data(
					service_target,
					service_type,
					chat_req,
					options_set,
				)
			}

			fn to_chat_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_response(model_iden, web_response, options_set)
			}

			fn to_chat_stream(
				model_iden: $crate::ModelIden,
				reqwest_builder: ::reqwest::RequestBuilder,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatStreamResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_stream(model_iden, reqwest_builder, options_set)
			}

			fn to_embed_request_data(
				_service_target: $crate::ServiceTarget,
				_embed_req: $crate::embed::EmbedRequest,
				_options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				Err($crate::Error::AdapterNotSupported {
					adapter_kind: $kind,
					feature: "embeddings".to_string(),
				})
			}

			fn to_embed_response(
				_model_iden: $crate::ModelIden,
				_web_response: $crate::webc::WebResponse,
				_options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::embed::EmbedResponse> {
				Err($crate::Error::AdapterNotSupported {
					adapter_kind: $kind,
					feature: "embeddings".to_string(),
				})
			}
		}
	};

	// ----- branch: all_model_names (custom), no unsupported list -----
	(
        name: $name:ident,
        kind: $kind:expr,
        key_env: $key_env:expr,
        endpoint: $endpoint:expr,
        delegate: $delegate:ty,
        all_model_names: $all_fn:expr $(,)?

    ) => {
		impl $crate::adapter::Adapter for $name {
			const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = $key_env;

			fn default_auth(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::AuthData {
				match Self::DEFAULT_API_KEY_ENV_NAME {
					Some(env_name) => $crate::resolver::AuthData::from_env(env_name),
					None => $crate::resolver::AuthData::None,
				}
			}

			fn default_endpoint(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::Endpoint {
				$crate::resolver::Endpoint::from_static($endpoint)
			}

			async fn all_model_names(
				kind: $crate::adapter::AdapterKind,
				endpoint: $crate::resolver::Endpoint,
				auth: $crate::resolver::AuthData,
				web_client: &$crate::webc::WebClient,
			) -> $crate::Result<Vec<String>> {
				let __all_fn = $all_fn;
				__all_fn(kind, endpoint, auth, web_client)
			}

			fn get_service_url(
				model_iden: &$crate::ModelIden,
				service_type: $crate::adapter::ServiceType,
				endpoint: $crate::resolver::Endpoint,
			) -> $crate::Result<String> {
				<$delegate as $crate::adapter::Adapter>::get_service_url(model_iden, service_type, endpoint)
			}

			fn to_web_request_data(
				service_target: $crate::ServiceTarget,
				service_type: $crate::adapter::ServiceType,
				chat_req: $crate::chat::ChatRequest,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				<$delegate as $crate::adapter::Adapter>::to_web_request_data(
					service_target,
					service_type,
					chat_req,
					options_set,
				)
			}

			fn to_chat_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_response(model_iden, web_response, options_set)
			}

			fn to_chat_stream(
				model_iden: $crate::ModelIden,
				reqwest_builder: ::reqwest::RequestBuilder,
				options_set: $crate::chat::ChatOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::chat::ChatStreamResponse> {
				<$delegate as $crate::adapter::Adapter>::to_chat_stream(model_iden, reqwest_builder, options_set)
			}

			fn to_embed_request_data(
				service_target: $crate::ServiceTarget,
				embed_req: $crate::embed::EmbedRequest,
				options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				<$delegate as $crate::adapter::Adapter>::to_embed_request_data(service_target, embed_req, options_set)
			}

			fn to_embed_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::embed::EmbedResponse> {
				<$delegate as $crate::adapter::Adapter>::to_embed_response(model_iden, web_response, options_set)
			}
		}
	};
}
