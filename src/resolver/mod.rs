// Resolvers are hooks that library users can set to customize part of the library's default behavior.
// A good example for now is the AuthResolver, which provides the authentication data (e.g., api_key).
//
// Eventually, the library will have more resolvers.

// region:    --- Modules

mod adapter_kind_resolver;
mod auth_resolver;
mod error;

pub use adapter_kind_resolver::*;
pub use auth_resolver::*;
pub use error::{Error, Result};

// endregion: --- Modules
