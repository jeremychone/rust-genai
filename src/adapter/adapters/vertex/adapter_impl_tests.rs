use super::VertexAdapter;
use crate::adapter::{Adapter, AdapterKind};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebClient;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_vertex_all_model_names_lists_no_retired_gemini() -> Result<()> {
	// -- Exec
	let names = VertexAdapter::all_model_names(
		AdapterKind::Vertex,
		Endpoint::from_static("https://aiplatform.googleapis.com/"),
		AuthData::from_single("test-key"),
		&WebClient::from_reqwest_client(reqwest::Client::new()),
	)
	.await?;

	// -- Check
	// Vertex retired gemini-2.5-pro, gemini-2.5-flash and gemini-2.5-flash-lite on 2026-10-20.
	assert!(
		!names.iter().any(|name| name.starts_with("gemini-2.5")),
		"retired Gemini models listed: {names:?}"
	);
	assert!(names.iter().any(|name| name == "gemini-3.8-flash"), "{names:?}");
	assert!(names.iter().any(|name| name.starts_with("claude-")), "{names:?}");

	Ok(())
}
