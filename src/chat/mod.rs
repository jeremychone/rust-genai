//! The genai chat module contains all of the constructs necessary
//! to make genai requests with the `genai::Client`.


// region:    --- Modules

mod chat_message;
mod chat_options;
mod chat_req_response_format;
mod chat_request;
mod chat_response;
mod chat_stream;
mod message_content;
mod tool;

// -- Flatten
pub use chat_message::*;
pub use chat_options::*;
pub use chat_req_response_format::*;
pub use chat_request::*;
pub use chat_response::*;
pub use chat_stream::*;
pub use message_content::*;
pub use tool::*;

pub mod printer;

// endregion: --- Modules
