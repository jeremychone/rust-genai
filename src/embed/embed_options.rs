//! ChatOptions allows customization of a chat request.
//! - It can be provided at the `client::exec_chat(..)` level as an argument,
//! - or set in the client config `client_config.with_chat_options(..)` to be used as the default for all requests
//!
//! Note 1: In the future, we will probably allow setting the client
//! Note 2: Extracting it from the `ChatRequest` object allows for better reusability of each component.

use serde::{Deserialize, Serialize};

/// Embed Options that are considered for any `Client::exec_embed...` calls.
///
/// A fallback `EmbedOptions` can also be set at the `Client` during the client builder phase
/// ``
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbedOptions {
	/// Will be used for this request if the Adapter/provider supports it.
	pub dimensions: Option<u32>,
}

/// Chainable Setters
impl EmbedOptions {
	/// Set the `temperature` for this request.
	pub fn with_dimensions(mut self, value: u32) -> Self {
		self.dimensions = Some(value);
		self
	}
}

// region:    --- EmbedOptionsSet

/// This is an internal crate struct to resolve the EmbedOptions value in a cascading manner.
/// First, it attempts to get the value at the embed level (EmbedOptions from the exec_embed...(...) argument).
/// If a value for the property is not found, it looks at the client default one.
#[derive(Default, Clone)]
pub(crate) struct EmbedOptionsSet<'a, 'b> {
	client: Option<&'a EmbedOptions>,
	embed: Option<&'b EmbedOptions>,
}

impl<'a, 'b> EmbedOptionsSet<'a, 'b> {
	pub fn with_client_options(mut self, options: Option<&'a EmbedOptions>) -> Self {
		self.client = options;
		self
	}
	pub fn with_embed_options(mut self, options: Option<&'b EmbedOptions>) -> Self {
		self.embed = options;
		self
	}
}

impl EmbedOptionsSet<'_, '_> {
	pub fn dimensions(&self) -> Option<u32> {
		self.embed
			.and_then(|chat| chat.dimensions)
			.or_else(|| self.client.and_then(|client| client.dimensions))
	}
}

// endregion: --- EmbedOptionsSet
