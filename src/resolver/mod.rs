//! Resolvers are hooks that library users can set to customize aspects of the library's default behavior.
//! A good example for now is the AuthResolver, which provides the authentication data (e.g., api_key).
//!
//! Eventually, the library will have more resolvers.

// region:    --- Modules

mod auth_resolver;
mod error;
mod model_mapper;

pub use auth_resolver::*;
pub use error::{Error, Result};
pub use model_mapper::*;

// endregion: --- Modules
