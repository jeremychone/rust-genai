// region:    --- Modules

mod chat;
mod gen;
mod tool;

// -- Flatten
pub use chat::*;
pub use gen::*;
pub use tool::*;

// endregion: --- Modules

#[derive(Debug)]
pub enum OutFormat {
	Text,
	Json,
}
