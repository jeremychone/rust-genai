//! This module contains all the types related to a Embed Request (Single or Batch).

use serde::{Deserialize, Serialize};

// region:    --- SingleEmbedRequest

/// The Chat request when performing a direct `Client::`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SingleEmbedRequest {
	/// The messages of the request.
	pub document: String,
}

/// Constructors
impl SingleEmbedRequest {
	/// Create a new SingleEmbedRequest with the given document.
	pub fn new(document: impl Into<String>) -> Self {
		Self {
			document: document.into(),
		}
	}
}

// endregion: --- SingleEmbedRequest

// region:    --- BatchEmbedRequest

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatchEmbedRequest {
	/// The messages of the request.
	pub documents: Vec<String>,
}

/// Constructors
impl BatchEmbedRequest {
	/// Create a new SingleEmbedRequest with the given document.
	pub fn new(documents: Vec<impl Into<String>>) -> Self {
		Self {
			documents: documents.into_iter().map(|s| s.into()).collect(),
		}
	}
}

/// Chainable Setters
impl BatchEmbedRequest {
	/// Set the documents of the request.
	pub fn with_documents(mut self, documents: Vec<impl Into<String>>) -> Self {
		self.documents = documents.into_iter().map(|s| s.into()).collect();
		self
	}

	/// Append a document to the request.
	pub fn append_document(mut self, document: impl Into<String>) -> Self {
		self.documents.push(document.into());
		self
	}
}

// endregion: --- BatchEmbedRequest
