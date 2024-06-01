use crate::{OutFormat, Result};
use futures::Stream;
use std::pin::Pin;

// region:    --- GenReq

#[derive(Debug)]
pub struct GenReq {
	pub prompt: String,
	pub inst: Option<String>,
	pub out_format: Option<OutFormat>,
}

impl From<&str> for GenReq {
	fn from(val: &str) -> Self {
		Self {
			prompt: val.to_string(),
			inst: None,
			out_format: None,
		}
	}
}

impl From<String> for GenReq {
	fn from(val: String) -> Self {
		Self {
			prompt: val,
			inst: None,
			out_format: None,
		}
	}
}

impl From<&String> for GenReq {
	fn from(val: &String) -> Self {
		Self {
			prompt: val.to_string(),
			inst: None,
			out_format: None,
		}
	}
}

// endregion: --- GenReq

// region:    --- GenRes

#[derive(Debug, Clone)]
pub struct GenRes {
	pub response: String,
}

// endregion: --- GenRes

// region:    --- GenResStream

pub type GenResStream = Pin<Box<dyn Stream<Item = Result<GenResChunks>>>>;

pub type GenResChunks = Vec<GenResChunk>;

pub struct GenResChunk {
	pub response: String,
}

// endregion: --- GenResStream
