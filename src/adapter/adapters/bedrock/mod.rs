//! AWS Bedrock Adapter Implementation
//!
//! API Documentation: https://docs.aws.amazon.com/bedrock/latest/APIReference/
//! Converse API: https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_Converse.html
//! ConverseStream API: https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_ConverseStream.html
//!
//! Authentication:
//! Uses Bearer token authentication with AWS Bedrock API keys.
//! See: https://docs.aws.amazon.com/bedrock/latest/userguide/api-keys-use.html
//!
//! Supported Models:
//! - anthropic.claude-3-5-sonnet-20241022-v2:0
//! - anthropic.claude-3-5-haiku-20241022-v1:0
//! - anthropic.claude-3-opus-20240229-v1:0
//! - anthropic.claude-3-sonnet-20240229-v1:0
//! - anthropic.claude-3-haiku-20240307-v1:0
//! - meta.llama3-70b-instruct-v1:0
//! - meta.llama3-8b-instruct-v1:0
//! - amazon.titan-text-express-v1
//! - amazon.titan-text-lite-v1
//! - mistral.mistral-7b-instruct-v0:2
//! - mistral.mixtral-8x7b-instruct-v0:1
//! - cohere.command-r-plus-v1:0
//! - cohere.command-r-v1:0
//!
//! Environment Variables:
//! - AWS_BEARER_TOKEN_BEDROCK: AWS Bedrock API key (Bearer token)
//! - AWS_REGION: AWS Region (default: us-east-1)

mod adapter_impl;
mod streamer;

pub use adapter_impl::*;
