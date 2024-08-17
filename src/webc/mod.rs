// region:    --- Modules

mod error;
mod web_client;
// for when not `text/event-stream`
mod web_stream;

pub(crate) use error::Result;
pub(crate) use web_client::*;
pub(crate) use web_stream::*;

// only public for external use
pub use error::Error;

// endregion: --- Modules
