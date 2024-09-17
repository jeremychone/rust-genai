use genai::chat::{ChatMessage, ChatRequest};

pub fn seed_chat_req_simple() -> ChatRequest {
	ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is the sky blue?"),
	])
}
