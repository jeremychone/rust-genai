//! Re-export OAuthConfig from resolver module for backward compatibility.
//!
//! OAuthConfig has been moved to the resolver module to allow it to be
//! used in OAuthCredentials without circular dependencies.

#[allow(unused_imports)]
pub use crate::resolver::{OAuthConfig, CLAUDE_CODE_SYSTEM_TEXT};
