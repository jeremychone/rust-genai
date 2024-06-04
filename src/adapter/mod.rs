// The Adapter layer allows adapting the client request/response to various AI Providers
// Right now, it uses a static dispatch pattern with the `Adapter` trait and `AdapterDispatcher` implementation.
// Adapter implementations are under the `adapters` submodule organized by adapter type
//
// Note: All `Adapter` trait methods take the AdapterKind as an argument, and for now, the Adapter trait functions
//       are all static (i.e. no `&self`). This reduces state management and enforces all states to be passed as arguments.

// region:    --- Modules

mod adapter_types;
mod adapters;
mod dispatcher;
mod support;

pub use adapter_types::*;
pub use adapters::*;
pub use dispatcher::*;
pub use support::*;

// endregion: --- Modules
