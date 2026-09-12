#![cfg(feature = "bedrock-sigv4")]

//! Live smoke test for `bedrock_sigv4` with an explicitly selected AWS profile.
//!
//! Needs the `bedrock-sigv4` feature and AWS credentials for the chosen profile; set
//! `BEDROCK_SIGV4_TEST_PROFILE` to pick it (defaults to `default`).
//!
//! Run with: `cargo test --features bedrock-sigv4 --test tests_p_bedrock_sigv4 -- --nocapture`

mod support;

use crate::support::{TestResult, common_tests};
use genai::resolver::AuthData;
use serial_test::serial;

// Cross-region inference profile prefix: newer Bedrock models are only reachable through one.
const MODEL: &str = "bedrock_sigv4::us.amazon.nova-lite-v1:0";

/// The profile is selected in code; a second client could use a different one in the same process.
#[tokio::test(flavor = "multi_thread")]
#[serial(bedrock_sigv4)]
async fn test_chat_with_selected_profile_ok() -> TestResult<()> {
	// -- Setup & Fixtures
	let profile = std::env::var("BEDROCK_SIGV4_TEST_PROFILE").unwrap_or_else(|_| "default".to_string());

	// -- Exec & Check
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::Key(profile)).await
}
