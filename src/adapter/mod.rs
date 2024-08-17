//! The Adapter layer allows adapting the client request/response to various AI Providers.
//! Right now, it uses a static dispatch pattern with the `Adapter` trait and `AdapterDispatcher` implementation.
//! Adapter implementations are organized by adapter type under the `adapters` submodule.
//!
//! Notes:
//! - All `Adapter` trait methods take the `AdapterKind` as an argument, and for now, the `Adapter` trait functions
//!   are all static (i.e., no `&self`). This reduces state management and enforces all states to be passed as arguments.
//! - Only `AdapterKind` from `AdapterConfig` is publicly exported.

// region:    --- Modules

mod support;

mod adapter_kind;
mod adapter_types;
mod adapters;
mod dispatcher;

// -- Flatten (private, crate, public)
use adapters::*;

pub(crate) use adapter_types::*;
pub(crate) use dispatcher::*;

pub use adapter_kind::*;

// -- Crate modules
pub(crate) mod inter_stream;

// endregion: --- Modules
