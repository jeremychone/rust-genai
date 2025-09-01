/// Note: MessageContent is use for the ChatRequest as well as the ChatResponse
use crate::chat::{ContentPart, ToolCall, ToolResponse};
use serde::{Deserialize, Serialize};

/// MessageContent for the ChatRequest and ChatResponse
///
/// This is a list of ContentPart that can be Text, Binary, ToolCall, or ToolResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageContent {
	/// The content parts of this message content
	parts: Vec<ContentPart>,
}

/// Constructors
impl MessageContent {
	/// Create a new MessageContent with a single Text part
	pub fn from_text(content: impl Into<String>) -> Self {
		Self {
			parts: vec![ContentPart::Text(content.into())],
		}
	}

	/// Create a new MessageContent from provided content parts
	pub fn from_parts(parts: impl Into<Vec<ContentPart>>) -> Self {
		Self { parts: parts.into() }
	}

	/// Create a new MessageContent from provided tool calls
	pub fn from_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
		Self {
			parts: tool_calls.into_iter().map(ContentPart::ToolCall).collect(),
		}
	}
}

/// Fluid Setters/Builders
impl MessageContent {
	/// Push a single ContentPart (Consuming Builder style)
	pub fn append(mut self, part: impl Into<ContentPart>) -> Self {
		self.parts.push(part.into());
		self
	}

	/// Push a single ContentPart
	pub fn push(&mut self, part: impl Into<ContentPart>) {
		self.parts.push(part.into());
	}

	pub fn extended<I>(mut self, iter: I) -> Self
	where
		I: IntoIterator<Item = ContentPart>,
	{
		self.parts.extend(iter);
		self
	}
}

impl Extend<ContentPart> for MessageContent {
	fn extend<T: IntoIterator<Item = ContentPart>>(&mut self, iter: T) {
		self.parts.extend(iter);
	}
}

// region:    --- Iterator Support

use std::iter::FromIterator;
use std::slice::{Iter, IterMut};

impl IntoIterator for MessageContent {
	type Item = ContentPart;
	type IntoIter = std::vec::IntoIter<ContentPart>;
	fn into_iter(self) -> Self::IntoIter {
		self.parts.into_iter()
	}
}

impl<'a> IntoIterator for &'a MessageContent {
	type Item = &'a ContentPart;
	type IntoIter = Iter<'a, ContentPart>;
	fn into_iter(self) -> Self::IntoIter {
		self.parts.iter()
	}
}

impl<'a> IntoIterator for &'a mut MessageContent {
	type Item = &'a mut ContentPart;
	type IntoIter = IterMut<'a, ContentPart>;
	fn into_iter(self) -> Self::IntoIter {
		self.parts.iter_mut()
	}
}

// collect() support
impl FromIterator<ContentPart> for MessageContent {
	fn from_iter<T: IntoIterator<Item = ContentPart>>(iter: T) -> Self {
		Self {
			parts: iter.into_iter().collect(),
		}
	}
}

// endregion: --- Iterator Support

/// Getters
impl MessageContent {
	pub fn parts(&self) -> &Vec<ContentPart> {
		&self.parts
	}

	pub fn into_parts(self) -> Vec<ContentPart> {
		self.parts
	}

	pub fn texts(&self) -> Vec<&str> {
		self.parts.iter().filter_map(|p| p.as_text()).collect()
	}

	pub fn into_texts(self) -> Vec<String> {
		self.parts.into_iter().filter_map(|p| p.into_text()).collect()
	}

	/// Returns references to tool calls only if all parts are ToolCall.
	pub fn tool_calls(&self) -> Vec<&ToolCall> {
		self.parts
			.iter()
			.filter_map(|p| match p {
				ContentPart::ToolCall(tc) => Some(tc),
				_ => None,
			})
			.collect()
	}

	/// Consumes and returns tool calls only if all parts are ToolCall.
	pub fn into_tool_calls(self) -> Vec<ToolCall> {
		self.parts
			.into_iter()
			.filter_map(|p| match p {
				ContentPart::ToolCall(tc) => Some(tc),
				_ => None,
			})
			.collect()
	}

	pub fn tool_responses(&self) -> Vec<&ToolResponse> {
		self.parts
			.iter()
			.filter_map(|p| match p {
				ContentPart::ToolResponse(tr) => Some(tr),
				_ => None,
			})
			.collect()
	}

	pub fn into_tool_responses(self) -> Vec<ToolResponse> {
		self.parts
			.into_iter()
			.filter_map(|p| match p {
				ContentPart::ToolResponse(tr) => Some(tr),
				_ => None,
			})
			.collect()
	}

	pub fn is_empty(&self) -> bool {
		self.parts.is_empty()
	}

	pub fn len(&self) -> usize {
		self.parts.len()
	}

	/// Returns true if there is at least one text part
	/// and all text parts are empty or whitespace.
	pub fn is_text_empty(&self) -> bool {
		if self.parts.is_empty() {
			return true;
		}
		self.parts
			.iter()
			.all(|p| matches!(p, ContentPart::Text(t) if t.trim().is_empty()))
	}
}

/// Convenient Getters
impl MessageContent {
	/// Returns the MessageContent as &str only if it contains exactly one Text part.
	/// Otherwise, it returns None.
	///
	/// NOTE: When multi-part content is present, this will return None and won't concatenate the text parts.
	pub fn first_text(&self) -> Option<&str> {
		let first_text_part = self.parts.iter().find(|p| p.is_text())?;
		first_text_part.as_text()
	}

	/// Consumes the MessageContent and returns it as String only if it contains exactly one Text part.
	/// Otherwise, it returns None.
	///
	/// NOTE: When multi-part content is present, this will return None and won't concatenate the text parts.
	pub fn into_first_text(self) -> Option<String> {
		let first_text_part = self.parts.into_iter().find(|p| p.is_text())?;
		first_text_part.into_text()
	}

	/// Joined the text, and join with empty line "\n" (will add extra "\n" if previous text does not end with "\n")
	pub fn joined_texts(&self) -> Option<String> {
		let texts = self.texts();
		if texts.is_empty() {
			return None;
		}

		if texts.len() == 1 {
			return texts.first().map(|s| s.to_string());
		}

		let mut combined = String::new();
		for text in texts {
			if combined.ends_with('\n') {
				combined.push('\n');
			} else if !combined.is_empty() {
				combined.push_str("\n\n");
			}
			// Do not add any empty line if previous content is empty

			combined.push_str(text);
		}
		Some(combined)
	}

	pub fn into_joined_texts(self) -> Option<String> {
		let texts = self.into_texts();
		if texts.is_empty() {
			return None;
		}

		if texts.len() == 1 {
			return texts.into_iter().next();
		}

		let mut combined = String::new();
		for text in texts {
			if combined.ends_with('\n') {
				combined.push('\n');
			} else if !combined.is_empty() {
				combined.push_str("\n\n");
			}
			// Do not add any empty line if previous content is empty

			combined.push_str(&text);
		}
		Some(combined)
	}
}

/// is_.., contains_..
impl MessageContent {
	pub fn is_text_only(&self) -> bool {
		self.parts.iter().all(|p| p.is_text())
	}

	pub fn contains_text(&self) -> bool {
		self.parts.iter().any(|p| p.is_text())
	}

	pub fn contains_tool_call(&self) -> bool {
		self.parts.iter().any(|p| p.is_tool_call())
	}

	pub fn contains_tool_response(&self) -> bool {
		self.parts.iter().any(|p| p.is_tool_response())
	}
}

// region:    --- Froms

impl From<&str> for MessageContent {
	fn from(s: &str) -> Self {
		Self {
			parts: vec![ContentPart::Text(s.to_string())],
		}
	}
}

impl From<&String> for MessageContent {
	fn from(s: &String) -> Self {
		Self {
			parts: vec![ContentPart::Text(s.clone())],
		}
	}
}

impl From<String> for MessageContent {
	fn from(s: String) -> Self {
		Self {
			parts: vec![ContentPart::Text(s)],
		}
	}
}

impl From<Vec<ToolCall>> for MessageContent {
	fn from(tool_calls: Vec<ToolCall>) -> Self {
		Self {
			parts: tool_calls.into_iter().map(ContentPart::ToolCall).collect(),
		}
	}
}

impl From<ToolResponse> for MessageContent {
	fn from(tool_response: ToolResponse) -> Self {
		Self {
			parts: vec![ContentPart::ToolResponse(tool_response)],
		}
	}
}

impl From<Vec<ContentPart>> for MessageContent {
	fn from(parts: Vec<ContentPart>) -> Self {
		Self { parts }
	}
}

// endregion: --- Froms
