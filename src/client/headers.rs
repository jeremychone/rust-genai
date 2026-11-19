use derive_more::From;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A map of HTTP headers (single value per name).
/// Headers can be layered; later values override earlier ones.
///
/// Note: This type keeps a single value per header name, which is the most
/// intuitive behavior for genai. It enables straightforward auth and other
/// overrides.
#[derive(Debug, Default, Clone, From, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Headers {
	inner: HashMap<String, String>,
}

impl Headers {
	/// Merge headers from overlay into self, consuming overlay.
	/// Later values override existing ones.
	/// Use [`Headers::merge_with`] for a borrowed overlay.
	pub fn merge(&mut self, overlay: impl Into<Headers>) {
		let overlay = overlay.into();

		// Insert or replace each single-value override:
		for (k, v) in overlay.inner {
			self.inner.insert(k, v);
		}
	}

	/// Merge headers from overlay into self without consuming it.
	/// Later values override existing ones.
	pub fn merge_with(&mut self, overlay: &Headers) {
		// Insert or replace each single-value override:
		for (k, v) in &overlay.inner {
			self.inner.insert(k.clone(), v.clone());
		}
	}

	/// Returns the value for `name`, if present.
	/// Lookup is ASCII case-insensitive, since HTTP header names are.
	pub fn get(&self, name: &str) -> Option<&str> {
		self.inner
			.get(name)
			.or_else(|| self.inner.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v))
			.map(String::as_str)
	}

	/// Apply self on top of target, consuming both, and return the result.
	/// Values in self override those in target.
	pub fn applied_to(self, target: impl Into<Headers>) -> Headers {
		let mut target = target.into();
		for (k, v) in self.inner {
			target.inner.insert(k, v);
		}
		target
	}
}

// region:    --- Froms

impl Headers {
	/// Builds `Headers` from a reqwest `HeaderMap` (e.g., HTTP response headers).
	/// Names are lowercase. Repeated headers are joined with `", "` (one value per name),
	/// and values that are not valid UTF-8 are converted lossily.
	pub(crate) fn from_header_map(header_map: &reqwest::header::HeaderMap) -> Self {
		let mut inner: HashMap<String, String> = HashMap::new();
		for (name, value) in header_map {
			let value = String::from_utf8_lossy(value.as_bytes());
			inner
				.entry(name.as_str().to_string())
				.and_modify(|existing| {
					existing.push_str(", ");
					existing.push_str(&value);
				})
				.or_insert_with(|| value.into_owned());
		}
		Headers { inner }
	}
}
impl<K, V> From<(K, V)> for Headers
where
	K: Into<String>,
	V: Into<String>,
{
	fn from((k, v): (K, V)) -> Self {
		let mut inner = HashMap::new();
		inner.insert(k.into(), v.into());
		Headers { inner }
	}
}

impl<K, V> From<Vec<(K, V)>> for Headers
where
	K: Into<String>,
	V: Into<String>,
{
	fn from(vec: Vec<(K, V)>) -> Self {
		let inner = vec.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
		Headers { inner }
	}
}

impl<K, V, const N: usize> From<[(K, V); N]> for Headers
where
	K: Into<String>,
	V: Into<String>,
{
	fn from(arr: [(K, V); N]) -> Self {
		let inner = arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
		Headers { inner }
	}
}

// endregion: --- Froms

// region:    --- Iterators

use std::collections::hash_map::{IntoIter, Iter, IterMut};

impl Headers {
	/// Returns an iterator over (name, value) pairs.
	pub fn iter(&self) -> Iter<'_, String, String> {
		self.inner.iter()
	}

	/// Returns a mutable iterator over (name, value) pairs.
	pub fn iter_mut(&mut self) -> IterMut<'_, String, String> {
		self.inner.iter_mut()
	}
}

// IntoIterator for owned Headers
impl IntoIterator for Headers {
	type Item = (String, String);
	type IntoIter = IntoIter<String, String>;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.into_iter()
	}
}

// IntoIterator for &Headers
impl<'a> IntoIterator for &'a Headers {
	type Item = (&'a String, &'a String);
	type IntoIter = Iter<'a, String, String>;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter()
	}
}

// IntoIterator for &mut Headers
impl<'a> IntoIterator for &'a mut Headers {
	type Item = (&'a String, &'a mut String);
	type IntoIter = IterMut<'a, String, String>;

	fn into_iter(self) -> Self::IntoIter {
		self.inner.iter_mut()
	}
}

// endregion: --- Iterators

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;
	use reqwest::header::{HeaderMap, HeaderValue};

	#[test]
	fn test_headers_get_case_insensitive() -> Result<()> {
		// -- Setup & Fixtures
		let headers = Headers::from([("X-Request-Id", "req-1")]);

		// -- Exec & Check
		assert_eq!(headers.get("X-Request-Id"), Some("req-1"));
		assert_eq!(headers.get("x-request-id"), Some("req-1"));
		assert_eq!(headers.get("x-other"), None);

		Ok(())
	}

	#[test]
	fn test_headers_from_header_map_joins_repeated_values() -> Result<()> {
		// -- Setup & Fixtures
		let mut header_map = HeaderMap::new();
		header_map.insert("x-request-id", HeaderValue::from_static("req-1"));
		header_map.append("x-multi", HeaderValue::from_static("a"));
		header_map.append("x-multi", HeaderValue::from_static("b"));

		// -- Exec
		let headers = Headers::from_header_map(&header_map);

		// -- Check
		assert_eq!(headers.get("x-request-id"), Some("req-1"));
		assert_eq!(headers.get("x-multi"), Some("a, b"));

		Ok(())
	}
}

// endregion: --- Tests
