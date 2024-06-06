//! Public common types

use crate::adapter::AdapterConfig;
use crate::client::ClientConfig;

pub struct ConfigSet<'a> {
	client_config: &'a ClientConfig,
	adapter_config: &'a AdapterConfig,
}

impl<'a> ConfigSet<'a> {
	pub fn new(client_config: &'a ClientConfig, adapter_config: &'a AdapterConfig) -> ConfigSet<'a> {
		ConfigSet {
			client_config,
			adapter_config,
		}
	}

	pub fn client_config(&self) -> &ClientConfig {
		self.client_config
	}

	pub fn adapter_config(&self) -> &AdapterConfig {
		self.adapter_config
	}
}
