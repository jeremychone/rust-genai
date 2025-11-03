//! ZAI API Documentation
//! API Documentation:     <https://api.z.ai>
//! Model Names:           GLM series models
//! Pricing:               <https://api.z.ai/pricing>
//!
//! ## Dual Endpoint Support
//!
//! ZAI supports two different API endpoints using the ServiceTargetResolver pattern:
//!
//! ### Regular API (Credit-based)
//! - Endpoint: `https://api.z.ai/api/paas/v4/`
//! - Models: `glm-4.6`, `glm-4.5`, etc.
//! - Usage: Standard API calls billed per token
//!
//! ### Coding Plan (Subscription-based)  
//! - Endpoint: `https://api.z.ai/api/coding/paas/v4/`
//! - Models: `coding::glm-4.6`, `coding:glm-4.5`, etc.
//! - Usage: Fixed monthly subscription for coding tasks
//!
//! ## Usage with ServiceTargetResolver
//!
//! ```rust
//! use genai::resolver::{Endpoint, ServiceTargetResolver};
//! use genai::{Client, AdapterKind, ModelIden};
//!
//! let target_resolver = ServiceTargetResolver::from_resolver_fn(
//!     |service_target| -> Result<ServiceTarget, _> {
//!         let model_name = service_target.model.model_name.to_string();
//!         
//!         // Route to appropriate endpoint based on model naming
//!         let endpoint_url = if model_name.starts_with("coding::") {
//!             "https://api.z.ai/api/coding/paas/v4/"
//!         } else {
//!             "https://api.z.ai/api/paas/v4/"
//!         };
//!         
//!         let final_endpoint = Endpoint::from_static(endpoint_url);
//!         let final_model = ModelIden::new(AdapterKind::Zai, clean_model_name);
//!         
//!         Ok(ServiceTarget { endpoint: final_endpoint, model: final_model })
//!     }
//! );
//!
//! let client = Client::builder().with_service_target_resolver(target_resolver).build();
//!
//! // Use regular API
//! let response = client.exec_chat("glm-4.6", chat_request, None).await?;
//!
//! // Use coding plan
//! let response = client.exec_chat("coding::glm-4.6", chat_request, None).await?;
//! ```
//!
//! See `examples/c07-zai-dual-endpoints.rs` for a complete working example.

// region:    --- Modules

mod adapter_impl;

pub use adapter_impl::*;

// endregion: --- Modules
