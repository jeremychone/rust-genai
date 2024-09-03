use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum ChatResponseFormat {
	JsonMode,
}
