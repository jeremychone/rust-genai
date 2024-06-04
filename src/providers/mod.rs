// region:    --- Modules

// External providers
mod ext;
mod support;

// Flatten external providers (they are prefixed with ext to allow their crate name)
pub use ext::*;

// -- Here the native providers
pub mod anthropic;

// endregion: --- Modules
