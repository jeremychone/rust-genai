//! The genai chat module contains all of the constructs necessary
//! to make genai embedding requests with the `genai::Client`.

// region:    --- Modules

mod embed_options;
mod embed_request;
mod embed_response;

// -- Flatten
pub use embed_options::*;
pub use embed_request::*;
pub use embed_response::*;

// pub mod printer;

// endregion: --- Modules
