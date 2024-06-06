use std::fmt;

#[derive(Debug, Default)]
pub struct ClientConfig {}

/// Convenient Constructors
/// Note: Those constructor(s) will call `default()` and sent the given property
///       They are just for convenience, the builder setter function can be used.
impl ClientConfig {}

#[derive(Debug, Default)]
pub enum ApiKeyStrategy {
	#[default]
	FallBackToDefaultEnv,
	Another,
}

// region:    --- EndPoint

#[derive(Debug)]
pub struct EndPoint {
	pub host: Option<String>,
	pub port: Option<u16>,
}

impl From<(String, u16)> for EndPoint {
	fn from((host, port): (String, u16)) -> Self {
		Self {
			host: Some(host),
			port: Some(port),
		}
	}
}

impl From<(&str, u16)> for EndPoint {
	fn from((host, port): (&str, u16)) -> Self {
		Self {
			host: Some(host.to_string()),
			port: Some(port),
		}
	}
}

// endregion: --- EndPoint
