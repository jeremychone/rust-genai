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
	// ----- Unified top-level rule -----
	(
		name: $name:ident,
		kind: $kind:expr,
		key_env: $key_env:expr,
		endpoint: $endpoint:expr,
		delegate: $delegate:ty
		$(, unsupported: [ $($unsupported:ident),* $(,)? ])?
		$(,)?
	) => {
		impl $crate::adapter::Adapter for $name {
			const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = $key_env;

			$crate::impl_pass_through_adapter!(@common_methods delegate = $delegate, endpoint = $endpoint);

			async fn all_model_names(
				kind: $crate::adapter::AdapterKind,
				endpoint: $crate::resolver::Endpoint,
				auth: $crate::resolver::AuthData,
				web_client: &$crate::webc::WebClient,
			) -> $crate::Result<Vec<String>> {
				<$delegate as $crate::adapter::Adapter>::all_model_names(kind, endpoint, auth, web_client).await
			}

			fn to_embed_request_data(
				service_target: $crate::ServiceTarget,
				embed_req: $crate::embed::EmbedRequest,
				options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::adapter::WebRequestData> {
				$crate::impl_pass_through_adapter!(
					@embed_req_body
					kind = $kind,
					delegate = $delegate,
					st = service_target,
					er = embed_req,
					os = options_set
					$(, unsupported = [ $($unsupported)* ])?
				)
			}

			fn to_embed_response(
				model_iden: $crate::ModelIden,
				web_response: $crate::webc::WebResponse,
				options_set: $crate::embed::EmbedOptionsSet<'_, '_>,
			) -> $crate::Result<$crate::embed::EmbedResponse> {
				$crate::impl_pass_through_adapter!(
					@embed_res_body
					kind = $kind,
					delegate = $delegate,
					mi = model_iden,
					wr = web_response,
					os = options_set
					$(, unsupported = [ $($unsupported)* ])?
				)
			}
		}
	};

	// ----- Internal helper: shared trait-method bodies (never vary across call sites) -----
	(@common_methods delegate = $delegate:ty, endpoint = $endpoint:expr) => {
		fn default_auth(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::AuthData {
			match Self::DEFAULT_API_KEY_ENV_NAME {
				Some(env_name) => $crate::resolver::AuthData::from_env(env_name),
				None => $crate::resolver::AuthData::None,
			}
		}

		fn default_endpoint(_kind: $crate::adapter::AdapterKind) -> $crate::resolver::Endpoint {
			$crate::resolver::Endpoint::from_static($endpoint)
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
	};

	// ----- Internal helper: to_embed_request_data, unsupported: [embeddings] -----
	(@embed_req_body kind=$k:expr, delegate=$del:ty, st=$st:ident, er=$er:ident, os=$os:ident, unsupported=[embeddings]) => {
		{
			let _ = (&$st, &$er, &$os);
			Err($crate::Error::AdapterNotSupported {
				adapter_kind: $k,
				feature: "embeddings".to_string(),
			})
		}
	};

	// ----- Internal helper: to_embed_request_data, default (delegate) -----
	(@embed_req_body kind=$k:expr, delegate=$del:ty, st=$st:ident, er=$er:ident, os=$os:ident $(,)?) => {
		<$del as $crate::adapter::Adapter>::to_embed_request_data($st, $er, $os)
	};

	// ----- Internal helper: to_embed_response, unsupported: [embeddings] -----
	(@embed_res_body kind=$k:expr, delegate=$del:ty, mi=$mi:ident, wr=$wr:ident, os=$os:ident, unsupported=[embeddings]) => {
		{
			let _ = (&$mi, &$wr, &$os);
			Err($crate::Error::AdapterNotSupported {
				adapter_kind: $k,
				feature: "embeddings".to_string(),
			})
		}
	};

	// ----- Internal helper: to_embed_response, default (delegate) -----
	(@embed_res_body kind=$k:expr, delegate=$del:ty, mi=$mi:ident, wr=$wr:ident, os=$os:ident $(,)?) => {
		<$del as $crate::adapter::Adapter>::to_embed_response($mi, $wr, $os)
	};
}
