// region:    --- Modules

mod error;
mod web_client;
// for when not `text/event-stream`
mod web_stream;

pub use self::error::{Error, Result};
pub use web_client::*;
pub use web_stream::*;

// endregion: --- Modules
