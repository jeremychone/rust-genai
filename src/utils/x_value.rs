use serde::de::DeserializeOwned;
use serde_json::Value;

pub trait XValue {
	fn x_get<T: DeserializeOwned>(&self, name: &str) -> Result<T>;
	fn x_take<T: DeserializeOwned>(&mut self, name: &str) -> Result<T>;
}

impl XValue for Value {
	fn x_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
		let value = if path.starts_with('/') {
			self.pointer(path).ok_or_else(|| Error::PropertyNotFound(path.to_string()))?
		} else {
			self.get(path).ok_or_else(|| Error::PropertyNotFound(path.to_string()))?
		};

		let value: T = serde_json::from_value(value.clone())?;
		Ok(value)
	}

	fn x_take<T: DeserializeOwned>(&mut self, path: &str) -> Result<T> {
		let value = if path.starts_with('/') {
			self.pointer_mut(path)
				.map(Value::take)
				.ok_or_else(|| Error::PropertyNotFound(path.to_string()))?
		} else {
			self.get_mut(path)
				.map(Value::take)
				.ok_or_else(|| Error::PropertyNotFound(path.to_string()))?
		};

		let value: T = serde_json::from_value(value)?;
		Ok(value)
	}
}

// region:    --- Error
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, derive_more::From)]
pub enum Error {
	PropertyNotFound(String),
	#[from]
	SerdeJson(serde_json::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate

// endregion: --- Error
