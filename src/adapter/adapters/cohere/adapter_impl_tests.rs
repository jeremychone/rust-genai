use super::CohereAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType};
use crate::chat::{ChatMessage, ChatOptionsSet, ChatRequest};
use crate::resolver::{AuthData, Endpoint};
use crate::{ModelIden, ServiceTarget};
use serde_json::json;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

fn chat_payload(chat_req: ChatRequest) -> Result<serde_json::Value> {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::Cohere, "command-r-plus"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.cohere.com/v1/"),
	};

	let web_req = CohereAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	Ok(web_req.payload)
}

#[test]
fn test_cohere_chat_history_entries_use_message_field() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("What is Rust?"),
		ChatMessage::assistant("A systems language."),
		ChatMessage::user("Who made it?"),
	]);

	// -- Exec
	let payload = chat_payload(chat_req)?;

	// -- Check
	// Cohere's v1 chat API reads `message` in each history entry (and the response parser reads it back).
	assert_eq!(payload["message"], "Who made it?");
	assert_eq!(
		payload["chat_history"],
		json!([
			{"role": "USER", "message": "What is Rust?"},
			{"role": "CHATBOT", "message": "A systems language."},
		])
	);

	Ok(())
}

#[test]
fn test_cohere_single_message_has_no_chat_history() -> Result<()> {
	let payload = chat_payload(ChatRequest::new(vec![ChatMessage::user("Hello")]))?;

	assert_eq!(payload["message"], "Hello");
	assert!(payload.get("chat_history").is_none());

	Ok(())
}
