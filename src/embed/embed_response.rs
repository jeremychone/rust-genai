//! This module contains all the types related to a Chat Response (except ChatStream, which has its own file).

use serde::{Deserialize, Serialize};

use crate::ModelIden;
use serde_with::{serde_as, skip_serializing_none};

// region:    --- EmbedResponse

/// The Embed response when performing a direct `Client::`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
	/// The eventual content of the chat response
	pub embeddings: Vec<EmbeddingObject>,

	/// The Model Identifier (AdapterKind/ModelName) used for this request.
	/// > NOTE: This might be different from the request model if changed by the ModelMapper
	pub model_iden: ModelIden,

	/// The eventual usage of the chat response
	pub usage: MetaUsage,
}

/// A single embedding object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingObject {
	pub index: Option<i32>,
	pub embedding: Vec<f64>,
}

// Getters
impl EmbedResponse {
	/// Returns the first embedding object of the list as `&EmbeddingObject` if it exists.
	/// Otherwise, returns None
	pub fn first_embedding(&self) -> Option<&EmbeddingObject> {
		self.embeddings.iter().next()
	}

	/// Consumes the EmbedResponse and returns the first embedding object of the list
	/// Otherwise, returns None
	pub fn into_first_embedding(self) -> Option<EmbeddingObject> {
		self.embeddings.into_iter().next()
	}

	pub fn embeddings(&self) -> Vec<&EmbeddingObject> {
		self.embeddings.iter().collect()
	}

	pub fn into_embeddings(self) -> Vec<EmbeddingObject> {
		self.embeddings
	}
}

// endregion: --- EmbedResponse

// region:    --- MetaUsage

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MetaUsage {
	/// The input tokens (replaces input_tokens)
	pub prompt_tokens: Option<i32>,

	/// The total number of tokens if returned by the API call.
	/// This will either be the total_tokens if returned, or the sum of prompt/completion if not specified in the response.
	pub total_tokens: Option<i32>,
}

// endregion: --- MetaUsage
