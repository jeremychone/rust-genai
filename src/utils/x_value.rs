use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

#[allow(unused)]
pub trait XValue {
	fn x_get<T: DeserializeOwned>(&self, name: &str) -> Result<T>;
	fn x_take<T: DeserializeOwned>(&mut self, name: &str) -> Result<T>;
	fn x_deep_insert<T: Serialize>(&mut self, name: &str, value: T) -> Result<()>;
	fn x_insert<T: Serialize>(&mut self, name: &str, value: T) -> Result<()>;
	fn x_pretty(&self) -> Result<String>;
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

	fn x_deep_insert<T: Serialize>(&mut self, name: &str, value: T) -> Result<()> {
		let new_value = serde_json::to_value(value)?;

		if !name.starts_with('/') {
			match self {
				Value::Object(map) => {
					map.insert(name.to_string(), new_value);
					Ok(())
				}
				_ => Err(Error::custom("Value is not an Object, cannot x_insert")),
			}
		} else {
			let parts: Vec<&str> = name.split('/').skip(1).collect();
			let mut current = self;

			// -- Add the eventual missing parents
			for &part in &parts[..parts.len() - 1] {
				match current {
					Value::Object(map) => {
						current = map.entry(part).or_insert_with(|| json!({}));
					}
					_ => return Err(Error::custom("Path does not point to an Object")),
				}
			}

			// -- Set the value at the last element
			if let Some(&last_part) = parts.last() {
				match current {
					Value::Object(map) => {
						map.insert(last_part.to_string(), new_value);
						Ok(())
					}
					_ => Err(Error::custom("Path does not point to an Object")),
				}
			} else {
				Err(Error::custom("Invalid path"))
			}
		}
	}

	fn x_insert<T: Serialize>(&mut self, path: &str, value: T) -> Result<()> {
		let value = serde_json::to_value(value)?;
		let (name, container) = if path.starts_with('/') {
			let name = path
				.rsplitn(2, '/')
				.last()
				.ok_or_else(|| Error::custom("json pointer not valid"))?;
			let container = self
				.pointer_mut(path)
				.ok_or_else(|| Error::custom("json value not found at pointer"))?;
			(name, container)
		} else {
			(path, self)
		};

		let container = container
			.as_object_mut()
			.ok_or_else(|| Error::custom("value is not a object"))?;

		container.insert(name.to_string(), value);

		Ok(())
	}

	fn x_pretty(&self) -> Result<String> {
		let content = serde_json::to_string_pretty(self)?;
		Ok(content)
	}
}

// region:    --- Error
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, derive_more::From)]
pub enum Error {
	Custom(String),

	PropertyNotFound(String),
	#[from]
	SerdeJson(serde_json::Error),
}

impl Error {
	fn custom(val: impl std::fmt::Display) -> Self {
		Self::Custom(val.to_string())
	}
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
