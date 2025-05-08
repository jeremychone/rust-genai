use serde::{Deserialize, Deserializer};

pub fn zero_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
	D: Deserializer<'de>,
	T: Deserialize<'de> + PartialEq + Default,
{
	// Attempt to deserialize as the inner type T
	let value = Option::<T>::deserialize(deserializer)?;

	// If the value is Some(0) or the default for the type, return None
	match value {
		Some(val) if val == T::default() => Ok(None),
		other => Ok(other),
	}
}
