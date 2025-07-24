//! The genai embed module contains all of the constructs necessary
//! to make embedding requests with the `genai::Client`.

// region:    --- Modules

mod embed_request;
mod embed_response;
mod embed_options;

// -- Flatten
pub use embed_request::*;
pub use embed_response::*;
pub use embed_options::*;

// endregion: --- Modules
