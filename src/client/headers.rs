use derive_more::From;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a list of headers to be applied to a request.
/// These can be applied on top of each other to produce the final list.
///
/// NOTE: Currently, this Headers construct supports only one value per header name,
///       which, in the context of genai, is the most intuitive default behavior.
///       This allows for natural authentication and other header overrides.
#[derive(Debug, Default, Clone, From, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Headers {
	inner: HashMap<String, String>,
}

impl Headers {
	/// Merge this ExtraHeaders in place with the overlay ExtraHeaders
	/// This take the ownership of the overlay
	/// Use [`extend_with`] for reference overllay
	pub fn merge(&mut self, overlay: impl Into<Headers>) {
		let overlay = overlay.into();

		// Insert or replace each single-value override:
		for (k, v) in overlay.inner {
			self.inner.insert(k, v);
		}
	}

	/// Extend this ExtraHeaders in place with the overlay
	/// This takes a reference to the overlay, and does not consume it.
	pub fn merge_with(&mut self, overlay: &Headers) {
		// Insert or replace each single-value override:
		for (k, v) in &overlay.inner {
			self.inner.insert(k.clone(), v.clone());
		}
	}

	/// Apply this header on top of a target ExtraHeaders.
	/// Consuming both, and returning the augmented target
	pub fn applied_to(self, target: impl Into<Headers>) -> Headers {
		let mut target = target.into();
		for (k, v) in self.inner {
			target.inner.insert(k, v);
		}
		target
	}
}

// region:    --- Froms
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
	/// Returns an iterator over the headers.
	pub fn iter(&self) -> Iter<'_, String, String> {
		self.inner.iter()
	}

	/// Returns a mutable iterator over the headers.
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
