use crate::adapter::{AdapterDispatcher, AdapterKind};
use crate::chat::ChatOptions;
use crate::client::{ModelSpec, ServiceTarget};
use crate::embed::EmbedOptions;
use crate::resolver::{AuthData, AuthResolver, Endpoint, ModelMapper, ServiceTargetResolver};
use crate::{Error, ModelIden, Result, WebConfig};

/// Configuration for building and customizing a `Client`.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(super) auth_resolver: Option<AuthResolver>,
	pub(super) service_target_resolver: Option<ServiceTargetResolver>,
	pub(super) model_mapper: Option<ModelMapper>,
	pub(super) web_config: Option<WebConfig>,
	pub(super) chat_options: Option<ChatOptions>,
	pub(super) embed_options: Option<EmbedOptions>,
	pub(super) adapter_kind: Option<AdapterKind>,
}

/// Chainable setters related to the ClientConfig.
impl ClientConfig {
	/// Sets the AuthResolver.
	///
	/// Called before `service_target_resolver`; if set, it will receive this value.
	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		self.auth_resolver = Some(auth_resolver);
		self
	}

	/// Sets the ModelMapper.
	///
	/// Called before `service_target_resolver`; if set, it will receive this value.
	pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self {
		self.model_mapper = Some(model_mapper);
		self
	}

	/// Sets the ServiceTargetResolver.
	///
	/// Final step before execution; allows full control over the resolved endpoint, auth, and model identifier.
	pub fn with_service_target_resolver(mut self, service_target_resolver: ServiceTargetResolver) -> Self {
		self.service_target_resolver = Some(service_target_resolver);
		self
	}

	/// Sets default ChatOptions for chat requests.
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		self.chat_options = Some(options);
		self
	}

	/// Sets default EmbedOptions for embed requests.
	pub fn with_embed_options(mut self, options: EmbedOptions) -> Self {
		self.embed_options = Some(options);
		self
	}

	/// Sets the HTTP client configuration (reqwest).
	pub fn with_web_config(mut self, web_config: WebConfig) -> Self {
		self.web_config = Some(web_config);
		self
	}

	/// Binds this Client to a single [`AdapterKind`].
	///
	/// A Client that has configured an [`AuthResolver`] or [`ServiceTargetResolver`]
	/// scoped to a specific adapter is *already* physically single-provider: both
	/// resolvers gate on `adapter_kind`, so any call that routes through a
	/// different adapter silently drops auth and the configured endpoint. Setting
	/// `adapter_kind` on the Client makes that constraint explicit and drives
	/// routing directly, bypassing the [`AdapterKind::from_model`] name-sniffing
	/// heuristic (which falls back to Ollama for unrecognized names).
	///
	/// When set:
	/// - [`ModelSpec::Name`] routes through this adapter. A `::` namespace prefix
	///   or other embedded adapter reference is a usage error (see
	///   [`Error::AdapterKindMismatch`]).
	/// - [`ModelSpec::Iden`] must carry the same adapter; otherwise returns
	///   [`Error::AdapterKindMismatch`] instead of silently producing a
	///   misconfigured request.
	/// - [`ModelSpec::Target`] is passed through unchanged (callers handing in a
	///   fully-resolved target have already opted out of Client-level routing).
	///
	/// Leave unset for the classic multi-provider shape (route per-call by model
	/// name).
	pub fn with_adapter_kind(mut self, adapter_kind: AdapterKind) -> Self {
		self.adapter_kind = Some(adapter_kind);
		self
	}

	/// Returns the WebConfig, if set.
	pub fn web_config(&self) -> Option<&WebConfig> {
		self.web_config.as_ref()
	}
}

/// Getters for the fields of ClientConfig (as references).
impl ClientConfig {
	/// Returns the AuthResolver, if set.
	pub fn auth_resolver(&self) -> Option<&AuthResolver> {
		self.auth_resolver.as_ref()
	}

	/// Returns the ServiceTargetResolver, if set.
	pub fn service_target_resolver(&self) -> Option<&ServiceTargetResolver> {
		self.service_target_resolver.as_ref()
	}

	/// Returns the ModelMapper, if set.
	pub fn model_mapper(&self) -> Option<&ModelMapper> {
		self.model_mapper.as_ref()
	}

	/// Returns the default ChatOptions, if set.
	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}

	/// Returns the default EmbedOptions, if set.
	pub fn embed_options(&self) -> Option<&EmbedOptions> {
		self.embed_options.as_ref()
	}

	/// Returns the bound [`AdapterKind`], if set via [`Self::with_adapter_kind`].
	pub fn adapter_kind(&self) -> Option<AdapterKind> {
		self.adapter_kind
	}
}

/// Resolvers
impl ClientConfig {
	/// Resolves auth and endpoint for the given adapter kind.
	///
	/// Used by `Client::all_model_names()` where no specific model name is available.
	pub(crate) async fn resolve_adapter_config(&self, adapter_kind: AdapterKind) -> Result<(AuthData, Endpoint)> {
		let model = ModelIden::new(adapter_kind, "");
		let auth = self.run_auth_resolver(model).await?;
		let endpoint = AdapterDispatcher::default_endpoint(adapter_kind);
		Ok((auth, endpoint))
	}

	/// Resolves a ServiceTarget for the given model.
	///
	/// Applies the ModelMapper (if any), resolves auth (via AuthResolver or adapter default),
	/// selects the adapter's default endpoint, then applies the ServiceTargetResolver (if any).
	///
	/// Errors with Error::Resolver if any resolver step fails.
	pub async fn resolve_service_target(&self, model: ModelIden) -> Result<ServiceTarget> {
		// -- Resolve the Model first
		let model = self.run_model_mapper(model.clone())?;

		// -- Get the auth
		let auth = self.run_auth_resolver(model.clone()).await?;

		// -- Get the default endpoint
		// For now, just get the default endpoint; the `resolve_target` will allow overriding it.
		let endpoint = AdapterDispatcher::default_endpoint(model.adapter_kind);

		// -- Create the default service target
		let service_target = ServiceTarget {
			model: model.clone(),
			auth,
			endpoint,
		};

		// -- Resolve the service target
		let service_target = self.run_service_target_resolver(service_target).await?;

		Ok(service_target)
	}

	/// Resolves a [`ModelIden`] to a [`ModelIden`] via the [`ModelMapper`] (if any).
	fn run_model_mapper(&self, model: ModelIden) -> Result<ModelIden> {
		match self.model_mapper() {
			Some(model_mapper) => model_mapper.map_model(model.clone()),
			None => Ok(model.clone()),
		}
		.map_err(|resolver_error| Error::Resolver {
			model_iden: model.clone(),
			resolver_error,
		})
	}

	/// Resolves a [`ModelIden`] to an [`AuthData`] via the [`AuthResolver`] (if any).
	async fn run_auth_resolver(&self, model: ModelIden) -> Result<AuthData> {
		match self.auth_resolver() {
			Some(auth_resolver) => {
				let auth_data = auth_resolver
					.resolve(model.clone())
					.await
					.map_err(|err| Error::Resolver {
						model_iden: model.clone(),
						resolver_error: err,
					})?
					// default the resolver resolves to nothing
					.unwrap_or_else(|| AdapterDispatcher::default_auth(model.adapter_kind));

				Ok(auth_data)
			}
			None => Ok(AdapterDispatcher::default_auth(model.adapter_kind)),
		}
	}

	/// Resolves a [`ServiceTarget`] via the [`ServiceTargetResolver`] (if any).
	async fn run_service_target_resolver(&self, service_target: ServiceTarget) -> Result<ServiceTarget> {
		let model = service_target.model.clone();

		match self.service_target_resolver() {
			Some(service_target_resolver) => {
				service_target_resolver
					.resolve(service_target)
					.await
					.map_err(|resolver_error| Error::Resolver {
						model_iden: model,
						resolver_error,
					})
			}
			None => Ok(service_target),
		}
	}

	/// Resolves a [`ModelSpec`] to a [`ServiceTarget`].
	///
	/// The resolution behavior depends on the variant and on whether the
	/// Client has been bound to an adapter via [`Self::with_adapter_kind`]:
	///
	/// - [`ModelSpec::Name`]: if a bound adapter is set, routes the bare
	///   name through it (a `::` namespace prefix in the name is rejected as
	///   [`Error::AdapterKindMismatch`] when it would resolve to a different
	///   adapter). Otherwise infers adapter from the name.
	///
	/// - [`ModelSpec::Iden`]: if a bound adapter is set and differs from the
	///   Iden's adapter, returns [`Error::AdapterKindMismatch`]. Otherwise
	///   proceeds with the Iden as given.
	///
	/// - [`ModelSpec::Target`]: returns the target directly, running only
	///   the service target resolver. A fully-resolved target has already
	///   opted out of Client-level routing.
	pub async fn resolve_model_spec(&self, spec: ModelSpec) -> Result<ServiceTarget> {
		match spec {
			ModelSpec::Name(name) => {
				let resolved = AdapterKind::from_model(&name)?;
				let adapter_kind = match self.adapter_kind {
					Some(bound) => {
						// If the name carries an explicit `::` namespace that
						// resolves to a different adapter, that's a
						// misconfiguration: the bound Client's resolvers
						// won't fire for it. Reject loudly.
						if resolved != bound && AdapterKind::from_model_namespace(&name).is_some() {
							return Err(Error::AdapterKindMismatch {
								bound,
								requested: resolved,
								model: name.to_string(),
							});
						}
						bound
					}
					None => resolved,
				};
				let model = ModelIden::new(adapter_kind, name);
				self.resolve_service_target(model).await
			}
			ModelSpec::Iden(model) => {
				if let Some(bound) = self.adapter_kind
					&& model.adapter_kind != bound
				{
					return Err(Error::AdapterKindMismatch {
						bound,
						requested: model.adapter_kind,
						model: model.model_name.to_string(),
					});
				}
				self.resolve_service_target(model).await
			}
			ModelSpec::Target(target) => self.run_service_target_resolver(target).await,
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use crate::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};

	/// Build a ClientConfig bound to the given adapter, with an
	/// auth/service-target resolver pair gated on that same adapter —
	/// mirroring the real-world configuration shape this feature is meant
	/// to model. The service_target_resolver records the adapter it
	/// received so tests can assert routing landed where expected.
	fn bound_config(adapter_kind: AdapterKind, observed_endpoint: &'static str) -> ClientConfig {
		ClientConfig::default()
			.with_adapter_kind(adapter_kind)
			.with_auth_resolver(AuthResolver::from_resolver_fn(
				move |model_iden: ModelIden| -> std::result::Result<Option<AuthData>, crate::resolver::Error> {
					if model_iden.adapter_kind == adapter_kind {
						Ok(Some(AuthData::from_single("test-key")))
					} else {
						// Simulate the real-world gating: mismatched adapter
						// gets no auth — which is exactly the silent failure
						// that motivated this feature.
						Ok(None)
					}
				},
			))
			.with_service_target_resolver(ServiceTargetResolver::from_resolver_fn(
				move |mut service_target: crate::ServiceTarget| -> std::result::Result<crate::ServiceTarget, crate::resolver::Error> {
					if service_target.model.adapter_kind == adapter_kind {
						service_target.endpoint = Endpoint::from_static(observed_endpoint);
					}
					Ok(service_target)
				},
			))
	}

	#[tokio::test]
	async fn bound_client_routes_bare_name_through_bound_adapter() {
		// `mini-max-m2.7` hits no prefix matcher — without a bound
		// adapter it would fall through to Ollama. With `with_adapter_kind`,
		// it must route through OpenAI instead.
		let config = bound_config(AdapterKind::OpenAI, "https://custom.example/v1");
		let target = config
			.resolve_model_spec(ModelSpec::Name("mini-max-m2.7".into()))
			.await
			.expect("bound name should resolve");
		assert_eq!(target.model.adapter_kind, AdapterKind::OpenAI);
		assert_eq!(target.endpoint.base_url(), "https://custom.example/v1");
	}

	#[tokio::test]
	async fn bound_client_rejects_mismatched_namespace() {
		// Bound to OpenAI, but caller passes an explicit `anthropic::...`
		// namespace. The resolvers would silently drop (no auth, default
		// endpoint). Return AdapterKindMismatch instead.
		let config = bound_config(AdapterKind::OpenAI, "https://custom.example/v1");
		let err = config
			.resolve_model_spec(ModelSpec::Name("anthropic::claude-3-5-sonnet".into()))
			.await
			.expect_err("mismatched namespace should error");
		match err {
			Error::AdapterKindMismatch { bound, requested, .. } => {
				assert_eq!(bound, AdapterKind::OpenAI);
				assert_eq!(requested, AdapterKind::Anthropic);
			}
			other => panic!("expected AdapterKindMismatch, got {other:?}"),
		}
	}

	#[tokio::test]
	async fn bound_client_accepts_matching_namespace() {
		// Redundant but harmless: `openai::gpt-4` against a Client bound
		// to OpenAI. The namespace matches, so the call proceeds normally.
		let config = bound_config(AdapterKind::OpenAI, "https://custom.example/v1");
		let target = config
			.resolve_model_spec(ModelSpec::Name("openai::gpt-4".into()))
			.await
			.expect("matching namespace should resolve");
		assert_eq!(target.model.adapter_kind, AdapterKind::OpenAI);
	}

	#[tokio::test]
	async fn bound_client_rejects_mismatched_iden() {
		// ModelSpec::Iden carries its own AdapterKind. If it disagrees
		// with the bound one, same silent-drop failure mode — reject.
		let config = bound_config(AdapterKind::OpenAI, "https://custom.example/v1");
		let iden = ModelIden::new(AdapterKind::Gemini, "gemini-1.5-pro");
		let err = config
			.resolve_model_spec(ModelSpec::Iden(iden))
			.await
			.expect_err("mismatched iden should error");
		assert!(matches!(
			err,
			Error::AdapterKindMismatch {
				bound: AdapterKind::OpenAI,
				requested: AdapterKind::Gemini,
				..
			}
		));
	}

	#[tokio::test]
	async fn unbound_client_preserves_inference() {
		// No `with_adapter_kind` set → classic behavior. `gpt-4` should
		// infer to OpenAI via the prefix matcher, and the service-target
		// resolver (gated on OpenAI) should fire.
		let config = bound_config(AdapterKind::OpenAI, "https://custom.example/v1");
		// Strip the binding to simulate an unbound Client that still has
		// resolvers attached (closest real-world unbound config).
		let config = ClientConfig {
			adapter_kind: None,
			..config
		};
		let target = config
			.resolve_model_spec(ModelSpec::Name("gpt-4".into()))
			.await
			.expect("unbound name should resolve via inference");
		assert_eq!(target.model.adapter_kind, AdapterKind::OpenAI);
	}

	#[test]
	fn bound_client_exposes_adapter_kind_via_getter() {
		// `Client::adapter_kind()` is the introspection getter a
		// caller uses to read the bound provider back off a built
		// Client (without having to carry the AdapterKind alongside
		// it). Set path returns Some, unset path returns None.
		let bound = crate::Client::builder().with_adapter_kind(AdapterKind::OpenAI).build();
		assert_eq!(bound.adapter_kind(), Some(AdapterKind::OpenAI));

		let unbound = crate::Client::default();
		assert_eq!(unbound.adapter_kind(), None);
	}
}

// endregion: --- Tests
