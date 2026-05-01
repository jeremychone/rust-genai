//! AWS Bedrock Converse adapters.
//!
//! Two adapters share this module:
//!
//! * `BedrockApiAdapter` — always available. Uses Bedrock's simple
//!   `Authorization: Bearer $BEDROCK_API_KEY` auth. Adds no new dependencies; reuses the shared
//!   reqwest `WebClient` and a hand-rolled event-stream parser.
//!
//! * `BedrockSigv4Adapter` — opt-in via the `bedrock-sigv4` Cargo feature. Full AWS credential
//!   chain + SigV4 request signing via `aws-config` and `aws-sigv4`. Best for workloads that
//!   already use AWS credentials (env, profile, SSO, IMDS, AssumeRole). Pulls in the AWS SDK
//!   smithy/hyper dep tree.
//!
//! Both adapters use the Converse API which normalizes chat requests across Bedrock's
//! publishers (Anthropic, Amazon Nova, Meta Llama, Mistral, Cohere, AI21).
//!
//! Usage: namespace model names with `bedrock_api::` (default) or `bedrock_sigv4::`, e.g.
//!   `bedrock_api::anthropic.claude-sonnet-4-5-20250929-v1:0`
//!   `bedrock_sigv4::amazon.nova-pro-v1:0`
//!
//! API Documentation:
//!   - Converse:        https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_Converse.html
//!   - ConverseStream:  https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_ConverseStream.html

mod adapter_api;
mod converse;
mod shared;
mod streamer;

#[cfg(feature = "bedrock-sigv4")]
mod adapter_sigv4;
#[cfg(feature = "bedrock-sigv4")]
mod sigv4;

pub use adapter_api::*;

#[cfg(feature = "bedrock-sigv4")]
pub use adapter_sigv4::*;
