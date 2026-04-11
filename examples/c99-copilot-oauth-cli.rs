//! Interactive CLI for testing the GitHub Copilot OAuth Device Flow.
//!
//! Commands:
//!   1) login   — run the full OAuth device flow (or skip if token already saved)
//!   2) status  — show saved token path + whether a token file exists
//!   3) chat    — send a test chat message using the current token
//!   4) logout  — delete the saved token file
//!   5) quit

use genai::Client;
use genai::adapter::{CopilotTokenManager, CopilotTokenStore, PrintCopilotCallback};
use genai::chat::{ChatMessage, ChatRequest};
use std::io::{self, Write};

const MODEL: &str = "github_copilot::gpt-4o";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn prompt(msg: &str) -> String {
	print!("{msg}");
	io::stdout().flush().unwrap();
	let mut buf = String::new();
	io::stdin().read_line(&mut buf).unwrap();
	buf.trim().to_string()
}

fn print_status() {
	match CopilotTokenStore::new() {
		Ok(store) => {
			let path = store.token_path().display();
			match store.load() {
				Ok(Some(_)) => println!("  Token file : {path}  ✓ (exists)"),
				Ok(None) => println!("  Token file : {path}  ✗ (not found — login required)"),
				Err(e) => println!("  Token file : {path}  ERROR: {e}"),
			}
		}
		Err(e) => println!("  Cannot resolve config dir: {e}"),
	}
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== GitHub Copilot OAuth CLI ===");
	println!("Model: {MODEL}");
	println!();

	loop {
		println!("Commands:");
		println!("  1) login   — authenticate via OAuth device flow");
		println!("  2) status  — show token file path and state");
		println!("  3) chat    — send a test chat message");
		println!("  4) logout  — delete saved token");
		println!("  5) quit");
		println!();

		let cmd = prompt("> ");

		match cmd.as_str() {
			"1" | "login" => {
				println!();
				println!("Starting OAuth device flow...");
				println!("(If a valid token is already saved, auth will be skipped.)");
				println!();

				let manager = CopilotTokenManager::new(PrintCopilotCallback);
				match manager.get_session().await {
					Ok((token, api_url)) => {
						println!("  ✓ Authenticated!");
						println!("  API URL      : {api_url}");
						if token.len() >= 16 {
							println!("  Session token: {}...{}", &token[..8], &token[token.len() - 4..]);
						} else {
							println!("  Session token: [REDACTED ({} chars)]", token.len());
						}
					}
					Err(e) => println!("  ✗ Auth failed: {e}"),
				}
			}

			"2" | "status" => {
				println!();
				print_status();
			}

			"3" | "chat" => {
				println!();
				let question = prompt("Question (leave blank for default): ");
				let question = if question.is_empty() {
					"Why is the sky blue? Answer in one sentence.".to_string()
				} else {
					question
				};

				println!("Sending: {question}");
				println!();

				let manager = CopilotTokenManager::new(PrintCopilotCallback);
				let resolver = manager.into_service_target_resolver();
				let client = Client::builder().with_service_target_resolver(resolver).build();

				let chat_req = ChatRequest::new(vec![
					ChatMessage::system("Answer in one sentence."),
					ChatMessage::user(question),
				]);

				match client.exec_chat(MODEL, chat_req, None).await {
					Ok(res) => println!("  ✓ {}", res.first_text().unwrap_or("(no text)")),
					Err(e) => println!("  ✗ Chat failed: {e}"),
				}
			}

			"4" | "logout" => {
				println!();
				match CopilotTokenStore::new() {
					Ok(store) => match store.clear() {
						Ok(()) => println!("  ✓ Token deleted ({})", store.token_path().display()),
						Err(e) => println!("  ✗ Failed to delete token: {e}"),
					},
					Err(e) => println!("  ✗ Cannot resolve config dir: {e}"),
				}
			}

			"5" | "q" | "quit" | "exit" => {
				println!("Bye.");
				break;
			}

			"" => {}

			other => println!("Unknown command: {other:?}"),
		}

		println!();
	}

	Ok(())
}
