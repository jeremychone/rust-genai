//! Some support utilities for the tests
//! Note: Must be imported in each test file

#![allow(unused)] // For test support

// region:    --- Modules

mod asserts;
mod data;
mod helpers;
mod seeders;

pub use asserts::*;
pub use helpers::*;
pub use seeders::*;

pub mod common_tests;

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// endregion: --- Modules
