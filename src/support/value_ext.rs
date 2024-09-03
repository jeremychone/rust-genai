use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::VecDeque;

pub trait ValueExt {
	/// Will return the value T for a given name/pointer path.
	/// - `name` - can be the direct name, or pointer path if starts with '/'
	fn x_get<T: DeserializeOwned>(&self, name_or_pointer: &str) -> Result<T>;

	/// Take the value at the name or pointer path, and replaces it with Null.
	/// - `name` - can be the direct name, or pointer path if starts with '/'
	fn x_take<T: DeserializeOwned>(&mut self, name_or_pointer: &str) -> Result<T>;

	/// Insert a new Value of type T at the specified name or pointer path.
	/// This method will create missing Value::Object as needed.
	/// - `name` - can be the direct name, or pointer path if starts with '/'
	fn x_insert<T: Serialize>(&mut self, name_or_pointer: &str, value: T) -> Result<()>;

	#[allow(unused)]
	fn x_walk<F>(&mut self, callback: F) -> bool
	where
		F: FnMut(&mut Map<String, Value>, &str) -> bool;

	/// Return the pretty_print string for this json value
	#[allow(unused)]
	fn x_pretty(&self) -> Result<String>;
}

impl ValueExt for Value {
	fn x_get<T: DeserializeOwned>(&self, name_or_pointer: &str) -> Result<T> {
		let value = if name_or_pointer.starts_with('/') {
			self.pointer(name_or_pointer)
				.ok_or_else(|| Error::PropertyNotFound(name_or_pointer.to_string()))?
		} else {
			self.get(name_or_pointer)
				.ok_or_else(|| Error::PropertyNotFound(name_or_pointer.to_string()))?
		};

		let value: T = serde_json::from_value(value.clone())?;
		Ok(value)
	}

	fn x_take<T: DeserializeOwned>(&mut self, name_or_pointer: &str) -> Result<T> {
		let value = if name_or_pointer.starts_with('/') {
			self.pointer_mut(name_or_pointer)
				.map(Value::take)
				.ok_or_else(|| Error::PropertyNotFound(name_or_pointer.to_string()))?
		} else {
			self.get_mut(name_or_pointer)
				.map(Value::take)
				.ok_or_else(|| Error::PropertyNotFound(name_or_pointer.to_string()))?
		};

		let value: T = serde_json::from_value(value)?;
		Ok(value)
	}

	fn x_insert<T: Serialize>(&mut self, name_or_pointer: &str, value: T) -> Result<()> {
		let new_value = serde_json::to_value(value)?;

		if !name_or_pointer.starts_with('/') {
			match self {
				Value::Object(map) => {
					map.insert(name_or_pointer.to_string(), new_value);
					Ok(())
				}
				_ => Err(Error::custom("Value is not an Object, cannot x_insert")),
			}
		} else {
			let parts: Vec<&str> = name_or_pointer.split('/').skip(1).collect();
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

	fn x_pretty(&self) -> Result<String> {
		let content = serde_json::to_string_pretty(self)?;
		Ok(content)
	}

	/// Walks through all properties of a JSON value tree and calls the callback function on each property.
	///
	/// - The callback signature is `(parent_map, property_name) -> bool`.
	///   - Return `false` from the callback to stop the traversal; return `true` to continue.
	///
	/// Returns:
	/// - `true` if the traversal completed to the end without being stopped early.
	/// - `false` if the traversal was stopped early because the callback returned `false`.
	fn x_walk<F>(&mut self, mut callback: F) -> bool
	where
		F: FnMut(&mut Map<String, Value>, &str) -> bool,
	{
		let mut queue = VecDeque::new();
		queue.push_back(self);

		while let Some(current) = queue.pop_front() {
			if let Value::Object(map) = current {
				// Call the callback for each property name in the current map
				for key in map.keys().cloned().collect::<Vec<_>>() {
					let res = callback(map, &key);
					if !res {
						return false;
					}
				}

				// Add all nested objects and arrays to the queue for further processing
				for value in map.values_mut() {
					if value.is_object() || value.is_array() {
						queue.push_back(value);
					}
				}
			} else if let Value::Array(arr) = current {
				// If current value is an array, add its elements to the queue
				for value in arr.iter_mut() {
					if value.is_object() || value.is_array() {
						queue.push_back(value);
					}
				}
			}
		}
		true
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

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Error = Box<dyn std::error::Error>;
	type Result<T> = core::result::Result<T, Error>; // For tests.

	use super::*;

	#[test]
	fn test_value_insert_ok() -> Result<()> {
		// -- Setup & Fixtures
		let mut value = json!({"tokens": 3});
		let fx_node_value = "hello";

		// -- Exec
		value.x_insert("/happy/word", fx_node_value)?;

		// -- Check
		let actual_value: String = value.x_get("/happy/word")?;
		assert_eq!(actual_value.as_str(), fx_node_value);

		Ok(())
	}

	#[test]
	fn test_value_walk_ok() -> Result<()> {
		// -- Setup & Fixtures
		let mut root_value = json!(
		{
				"tokens": 3,
				"schema": {
					"type": "object",
					"additionalProperties": false,
					"properties": {
						"all_models": {
							"type": "array",
							"items": {
								"type": "object",
								"additionalProperties": false,
								"properties": {
									"maker": { "type": "string" },
									"model_name": { "type": "string" }
								},
								"required": ["maker", "model_name"]
							}
						}
					},
					"required": ["all_models"]
				}
		});

		// -- Exec
		// Will remove "additionalProperties" (only the frist one, because return false)
		root_value.x_walk(|parent_map, property_name| {
			// --
			if property_name == "type" {
				let val = parent_map.get(property_name).and_then(|val| val.as_str());
				if let Some("object") = val {
					parent_map.remove("additionalProperties");
					return false; // will stop early
				}
			}
			true
		});

		// -- Check
		// the number of "additionalProperties" left
		let mut marker_count = 0;
		// Will remove "additionalProperties"
		root_value.x_walk(|_parent_map, property_name| {
			if property_name == "additionalProperties" {
				marker_count += 1;
			}
			true
		});
		assert_eq!(1, marker_count); // only 1 was removed, as callback returned false

		Ok(())
	}
}

// endregion: --- Tests
