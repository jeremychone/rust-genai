//! API:          <https://api.githubcopilot.com>
//! Auth:         GitHub OAuth Device Flow (<https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow>)
//! Models:       <https://docs.github.com/en/copilot/reference/ai-models/supported-models>

// region:    --- Modules

mod adapter_impl;
mod copilot_auth;
mod copilot_callback;
pub(crate) mod copilot_types;
mod token_store;

pub use adapter_impl::*;
pub use copilot_auth::CopilotTokenManager;
pub use copilot_callback::{CopilotAuthCallback, PrintCopilotCallback};
pub use token_store::CopilotTokenStore;

// endregion: --- Modules
