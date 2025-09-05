//! Tooling primitives for chat function-calling.
//!
//! This module exposes the core traits and data structures to define tools,
//! represent tool invocations emitted by models, and carry tool execution results.
//! The concrete types are implemented in submodules and re-exported here for
//! ergonomic importing.

// region:    --- Modules

mod tool_base;
mod tool_call;
mod tool_response;

/// Base traits and specifications for declaring tools.
pub use tool_base::*;

/// Types representing a tool invocation emitted by a model.
pub use tool_call::*;

/// Types for returning results produced by a tool.
pub use tool_response::*;

// endregion: --- Modules

