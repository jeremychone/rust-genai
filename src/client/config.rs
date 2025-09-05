use crate::adapter::AdapterDispatcher;
use crate::chat::ChatOptions;
use crate::client::ServiceTarget;
use crate::embed::EmbedOptions;
use crate::resolver::{AuthResolver, ModelMapper, ServiceTargetResolver};
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
}

/// Resolvers
impl ClientConfig {
	/// Resolves a ServiceTarget for the given model.
	///
	/// Applies the ModelMapper (if any), resolves auth (via AuthResolver or adapter default),
	/// selects the adapter's default endpoint, then applies the ServiceTargetResolver (if any).
	///
	/// Errors with Error::Resolver if any resolver step fails.
	pub async fn resolve_service_target(&self, model: ModelIden) -> Result<ServiceTarget> {
		// -- Resolve the Model first
		let model = match self.model_mapper() {
			Some(model_mapper) => model_mapper.map_model(model.clone()),
			None => Ok(model.clone()),
		}
		.map_err(|resolver_error| Error::Resolver {
			model_iden: model.clone(),
			resolver_error,
		})?;

		// -- Get the auth
		let auth = if let Some(auth) = self.auth_resolver() {
			// resolve async which may be async
			auth.resolve(model.clone())
				.await
				.map_err(|err| Error::Resolver {
					model_iden: model.clone(),
					resolver_error: err,
				})?
				// default the resolver resolves to nothing
				.unwrap_or_else(|| AdapterDispatcher::default_auth(model.adapter_kind))
		} else {
			AdapterDispatcher::default_auth(model.adapter_kind)
		};

		// -- Get the default endpoint
		// For now, just get the default endpoint; the `resolve_target` will allow overriding it.
		let endpoint = AdapterDispatcher::default_endpoint(model.adapter_kind);

		// -- Resolve the service_target
		let service_target = ServiceTarget {
			model: model.clone(),
			auth,
			endpoint,
		};
		let service_target = match self.service_target_resolver() {
			Some(service_target_resolver) => {
				service_target_resolver
					.resolve(service_target)
					.await
					.map_err(|resolver_error| Error::Resolver {
						model_iden: model,
						resolver_error,
					})?
			}
			None => service_target,
		};

		Ok(service_target)
	}
}
