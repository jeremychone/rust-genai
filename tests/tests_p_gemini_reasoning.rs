mod support;

use crate::support::{Check, common_tests};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// "gemini-2.5-flash-preview-04-17", "gemini-2.5-pro-preview-05-06"
const MODEL: &str = "gemini-2.5-flash-preview-04-17"; // can add "-medium" .. suffix

// NOTE: For now just single test to make sure reasonning token get captured.

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	// NOTE: At this point, gemini 2.5 does not seems to give back reasonning content.
	//       But it should have REASONING_USAGE
	common_tests::common_test_chat_simple_ok(MODEL, Some(Check::REASONING_USAGE)).await
}
