use genai::chat::{ChatRequest, Tool};
use serde_json::json;

const MODEL: &str = "gemini-3-flash-preview";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let question = "Give me the latest 3 news about Rust Programming.
For each new, have it in a markdown section ## _concise_summary_, 3 bullet points, and source link. Use `-` for bullet points.";

	let client = genai::Client::default();
	let web_search_tool = Tool::new("googleSearch").with_config(json!({}));
	let chat_req = ChatRequest::from_user(question).append_tool(web_search_tool);

	// Exec
	println!("\n=== AI Question:\n{question}");
	let res = client.exec_chat(MODEL, chat_req, None).await?;

	// Check
	let res_txt = res.content.into_first_text().ok_or("Should have result")?;
	assert!(res_txt.contains("Rust"), "should contains 'Rust'");

	println!("\n=== AI Response:\n{res_txt}");

	Ok(())
}
