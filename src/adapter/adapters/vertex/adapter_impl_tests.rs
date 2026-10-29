use super::VertexAdapter;
use crate::ModelIden;
use crate::ServiceTarget;
use crate::adapter::{Adapter, AdapterKind, ServiceType};
use crate::chat::{
	ChatOptions, ChatOptionsSet, ChatRequest, ChatResponseFormat, JsonSpec, ReasoningEffort, ToolChoice,
};
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

fn anthropic_url(service_type: ServiceType) -> String {
	let model = ModelIden::from_static(AdapterKind::Vertex, "claude-sonnet-4-6");
	let endpoint =
		Endpoint::from_static("https://us-east5-aiplatform.googleapis.com/v1/projects/p/locations/us-east5/");
	VertexAdapter::get_service_url(&model, service_type, endpoint).expect("service url")
}

#[test]
fn test_get_service_url_anthropic_chat_uses_raw_predict() {
	assert!(anthropic_url(ServiceType::Chat).ends_with("/publishers/anthropic/models/claude-sonnet-4-6:rawPredict"));
}

#[test]
fn test_get_service_url_anthropic_stream_uses_stream_raw_predict() {
	assert!(
		anthropic_url(ServiceType::ChatStream)
			.ends_with("/publishers/anthropic/models/claude-sonnet-4-6:streamRawPredict")
	);
}

#[test]
fn test_vertex_claude_sends_tool_choice_effort_and_format() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		endpoint: Endpoint::from_static("https://us-east5-aiplatform.googleapis.com/v1/projects/p/locations/us-east5/"),
		auth: AuthData::from_single("test-token"),
		model: ModelIden::from_static(AdapterKind::Vertex, "claude-sonnet-4-6"),
	};
	let schema = serde_json::json!({
		"type": "object",
		"properties": { "answer": { "type": "string" } },
		"required": ["answer"],
		"additionalProperties": false
	});
	let options = ChatOptions::default()
		.with_tool_choice(ToolChoice::Required)
		.with_reasoning_effort(ReasoningEffort::High)
		.with_response_format(ChatResponseFormat::JsonSpec(JsonSpec::new("answer", schema)));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let data =
		VertexAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hi"), options_set)?;

	// -- Check
	let payload = &data.payload;
	assert_eq!(payload["tool_choice"], serde_json::json!({ "type": "any" }));
	assert_eq!(payload["output_config"]["format"]["type"], "json_schema");
	assert!(payload["output_config"].get("effort").is_some(), "{payload}");
	assert!(payload.get("model").is_none(), "{payload}");
	assert_eq!(payload["anthropic_version"], "vertex-2023-10-16");
	assert_eq!(payload["stream"], false);

	Ok(())
}
