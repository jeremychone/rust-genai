//! Demonstrate speech-to-text with `gemini-3.5-transcribe`.
//!
//! This model exists only on the Interactions API, so it is reachable only through the
//! `gemini_interactions` adapter — which `gemini-3*` model names select automatically.
//!
//! Options worth knowing (see <https://ai.google.dev/gemini-api/docs/transcribe>):
//!   - `language_codes`: BCP-47 hints. Omit or leave empty for automatic detection.
//!   - `mode: {"type": "verbatim"}`: word-for-word, keeps disfluencies. Supports
//!     `diarization_mode: "speaker"` and `timestamp_granularities: ["word"]`.
//!   - `mode: {"type": "smart"}`: cleans up filler words and applies formatting.
//!     Incompatible with diarization and word timestamps.
//!
//! Requires: GEMINI_API_KEY, and an audio file path as the first argument.
//!
//! Run: `GEMINI_API_KEY=... cargo run --example c99-gemini-transcribe -- ./path/to/audio.mp3`

use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart};
use serde_json::json;

const MODEL: &str = "gemini_ix::gemini-3.5-transcribe";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let audio_path = std::env::args()
		.nth(1)
		.ok_or("Usage: cargo run --example c99-gemini-transcribe -- ./path/to/audio.mp3")?;

	let client = Client::new()?;

	// `from_binary_file` reads the file, infers the MIME type from the extension, and base64s it.
	let cp_audio = ContentPart::from_binary_file(&audio_path)?;
	let chat_req = ChatRequest::new(vec![ChatMessage::user(vec![cp_audio])]);

	let options = ChatOptions::default().with_capture_raw_body(true).with_extra_body(json!({
		"generation_config": {
			"transcription_config": {
				// Empty list = automatic language detection, including mid-sentence code-switching.
				"language_codes": [],
				"mode": {
					"type": "verbatim",
					"diarization_mode": "speaker",
					"timestamp_granularities": ["word"],
				}
			}
		}
	}));

	let res = client.exec_chat(MODEL, chat_req, Some(&options)).await?;

	println!("--- Transcript\n{}\n", res.first_text().unwrap_or("NO TRANSCRIPT"));

	// -- Word-level detail, straight off the raw body.
	if let Some(raw_body) = res.captured_raw_body.as_ref() {
		let annotations = raw_body
			.pointer("/steps/0/content/0/annotations")
			.and_then(|value| value.as_array());

		if let Some(annotations) = annotations {
			println!("--- Words ({} total, first 20)", annotations.len());
			for word in annotations.iter().filter(|a| a["type"] == "word_info").take(20) {
				let speaker = word["speaker"].as_str().unwrap_or("?");
				let start = word["start_offset"].as_str().unwrap_or("");
				let end = word["end_offset"].as_str().unwrap_or("");
				let text = word["text"].as_str().unwrap_or("");
				println!("[{speaker}] ({start} -> {end}) {text}");
			}
		}
	}

	Ok(())
}
