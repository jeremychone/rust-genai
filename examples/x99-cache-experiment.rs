//! EXPERIMENT (temporary): does `previous_interaction_id` earn better implicit-cache hits than
//! resending the full history the way `generateContent` / chat-completions callers do?
//!
//! Two arms, same model, same large prefix, same three logical turns:
//!   A) stateful  — turn N sends only the new message + `previous_interaction_id`
//!   B) stateless — turn N resends the whole transcript, `store: false`
//!
//! Implicit caching needs a 4096-token minimum for Gemini 3.5 Flash and rewards a large *common
//! prefix*, so both arms lead with an ~11k-token document and ask short questions about it.
//! Each arm prepends a unique run marker so the two arms cannot share cache entries with each
//! other — otherwise whichever runs second would inherit the first one's warm cache and the
//! comparison would be meaningless.
//!
//! Numbers are read from the raw response body, not genai's normalized `Usage`, so what is
//! reported is exactly what the provider said.
//!
//! Run: `GEMINI_API_KEY=... cargo run --example x99-cache-experiment`

use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatResponse, Tool, ToolResponse};
use serde_json::{Value, json};

/// Default is the Interactions API. Set `MODEL=gemini::gemini-3.7-flash` to run the identical
/// arms against `generateContent` instead — same model, different API surface.
fn model() -> String {
	std::env::var("MODEL_NAME").unwrap_or_else(|_| "gemini_ix::gemini-3.7-flash".to_string())
}

const QUESTIONS: [&str; 5] = [
	"In one sentence: what is the ChatRequest type for?",
	"In one sentence: what does ChatOptions::capture_usage do?",
	"In one sentence: what is an AdapterKind?",
	"In one sentence: what is a ServiceTarget?",
	"In one sentence: what does StopReason represent?",
];

#[derive(Debug, Default, Clone, Copy)]
struct TurnUsage {
	input: i64,
	cached: i64,
	output: i64,
	thought: i64,
}

/// Reads token counts straight off the raw body, so the numbers are the provider's own.
/// The two APIs report usage under different roots and different key names, so try both:
/// Interactions `usage.total_input_tokens`, generateContent `usageMetadata.promptTokenCount`.
fn read_usage(res: &ChatResponse) -> TurnUsage {
	let get = |ix_key: &str, gc_key: &str| -> i64 {
		let body = res.captured_raw_body.as_ref();
		body.and_then(|b| b.pointer(&format!("/usage/{ix_key}")))
			.or_else(|| body.and_then(|b| b.pointer(&format!("/usageMetadata/{gc_key}"))))
			.and_then(Value::as_i64)
			.unwrap_or(0)
	};
	TurnUsage {
		input: get("total_input_tokens", "promptTokenCount"),
		cached: get("total_cached_tokens", "cachedContentTokenCount"),
		output: get("total_output_tokens", "candidatesTokenCount"),
		thought: get("total_thought_tokens", "thoughtsTokenCount"),
	}
}

fn print_turn(arm: &str, turn: usize, usage: TurnUsage) {
	let pct = if usage.input > 0 {
		(usage.cached as f64 / usage.input as f64) * 100.0
	} else {
		0.0
	};
	println!(
		"  [{arm}] turn {turn}: input={:>6}  cached={:>6} ({pct:>5.1}%)  output={:>4}  thought={:>4}",
		usage.input, usage.cached, usage.output, usage.thought
	);
}

/// The shared prefix: a real document, large enough to clear the 4096-token minimum.
fn big_document(run_marker: &str) -> String {
	let doc = include_str!("../docs/for-llm/api-reference-for-llm.md");
	// The marker goes first so the two arms occupy distinct cache entries.
	format!("[run {run_marker}]\n\n{doc}")
}

/// Arm A: only the new message travels after turn 1; the server holds the transcript.
async fn run_stateful(
	client: &Client,
	options: &ChatOptions,
	marker: &str,
) -> Result<Vec<TurnUsage>, Box<dyn std::error::Error>> {
	println!("Arm A — stateful (previous_interaction_id)");
	let mut usages = Vec::new();
	let mut previous_id: Option<String> = None;

	for (turn, question) in QUESTIONS.iter().enumerate() {
		let chat_req = match &previous_id {
			// Turn 1 carries the document.
			None => ChatRequest::from_user(format!("{}\n\n{question}", big_document(marker))),
			// Later turns carry only the question.
			Some(id) => ChatRequest::from_user(*question).with_previous_response_id(id),
		}
		.with_store(true);

		let res = client.exec_chat(&model(), chat_req, Some(options)).await?;
		previous_id = res.response_id.clone();
		let usage = read_usage(&res);
		print_turn("A", turn + 1, usage);
		usages.push(usage);
	}

	Ok(usages)
}

/// Arm C: the *same* request, fired back to back. Not a conversation — a control that maximises
/// every condition Google lists for an implicit cache hit: an identical large prefix, repeated,
/// in quick succession. If caching does not engage here it will not engage in a real chat.
async fn run_repeat_probe(
	client: &Client,
	options: &ChatOptions,
	marker: &str,
	iterations: usize,
) -> Result<Vec<TurnUsage>, Box<dyn std::error::Error>> {
	println!("Arm C — identical request repeated (control, store=false)");
	let mut usages = Vec::new();
	let prompt = format!("{}\n\n{}", big_document(marker), QUESTIONS[0]);

	for turn in 0..iterations {
		let chat_req = ChatRequest::from_user(prompt.clone()).with_store(false);
		let res = client.exec_chat(&model(), chat_req, Some(options)).await?;
		let usage = read_usage(&res);
		print_turn("C", turn + 1, usage);
		usages.push(usage);
	}

	Ok(usages)
}

/// Arm D: same large document prefix, a *different* question appended each time, no history.
/// Isolates the one variable that separates arm C from a real conversation — does a changing
/// suffix still earn a hit on the unchanged prefix?
async fn run_varying_suffix_probe(
	client: &Client,
	options: &ChatOptions,
	marker: &str,
	iterations: usize,
) -> Result<Vec<TurnUsage>, Box<dyn std::error::Error>> {
	println!("Arm D — same prefix, different question each time (store=false)");
	let mut usages = Vec::new();
	let document = big_document(marker);

	for turn in 0..iterations {
		// A distinct suffix every iteration; the leading document never changes.
		let question = format!("{} (variant {turn})", QUESTIONS[turn % QUESTIONS.len()]);
		let chat_req = ChatRequest::from_user(format!("{document}\n\n{question}")).with_store(false);
		let res = client.exec_chat(&model(), chat_req, Some(options)).await?;
		let usage = read_usage(&res);
		print_turn("D", turn + 1, usage);
		usages.push(usage);
	}

	Ok(usages)
}

/// Arm E: a real agentic loop — big system prompt, tools, and a conversation that *strictly
/// grows*: every request contains the previous one entirely as its prefix, then appends the
/// assistant's tool call and the tool result. This is the shape that should be maximally
/// prefix-cache friendly, and it is what arms C and D do not cover.
///
/// `MODE=stateful` runs the same loop over `previous_interaction_id` instead of resending.
async fn run_agentic_probe(
	client: &Client,
	options: &ChatOptions,
	marker: &str,
	stateful: bool,
	max_rounds: usize,
) -> Result<Vec<TurnUsage>, Box<dyn std::error::Error>> {
	println!(
		"Arm E — agentic tool loop, {} (system prompt + tools, history grows every round)",
		if stateful { "stateful" } else { "stateless" }
	);

	let tool = Tool::new("lookup_population")
		.with_description("Look up the population of a single city.")
		.with_schema(json!({
			"type": "object",
			"properties": { "city": { "type": "string" } },
			"required": ["city"],
		}));

	// The large, unchanging prefix lives in the system instruction — the usual agent shape.
	let system = big_document(marker);
	let task = "Using the lookup_population tool, get the population of Tokyo, then Paris, then 	            Cairo, then Lima, then Oslo. Call the tool for exactly one city per turn and wait 	            for each result before the next. Finish with a one-sentence summary.";

	let mut usages = Vec::new();
	let mut messages: Vec<ChatMessage> = vec![ChatMessage::user(task)];
	let mut previous_id: Option<String> = None;

	for round in 0..max_rounds {
		// `system` and `tools` are interaction-scoped — re-sent every round in both modes.
		let chat_req = match (&previous_id, stateful) {
			// Stateful: only the newest turn travels.
			(Some(id), true) => ChatRequest::new(vec![
				messages.last().cloned().ok_or("stateful round needs a message to send")?,
			])
			.with_previous_response_id(id)
			.with_store(true),
			// Stateless (and the first stateful round): the whole transcript so far.
			_ => ChatRequest::new(messages.clone()).with_store(stateful),
		}
		.with_system(&system)
		.append_tool(tool.clone());

		let res = client.exec_chat(&model(), chat_req, Some(options)).await?;
		previous_id = res.response_id.clone();
		let usage = read_usage(&res);
		print_turn("E", round + 1, usage);
		usages.push(usage);

		// Grow the transcript: assistant turn, then the tool results.
		let tool_calls = res.into_tool_calls();
		if tool_calls.is_empty() {
			println!("      (no more tool calls — loop finished)");
			break;
		}
		let tool_responses: Vec<ToolResponse> = tool_calls
			.iter()
			.map(|call| {
				let city = call.fn_arguments["city"].as_str().unwrap_or("unknown");
				// `from_tool_call` carries the tool name. In stateful mode the adapter cannot
				// recover it — the function_call lives server-side — and the API rejects a
				// function_result that lacks the real name.
				ToolResponse::from_tool_call(call, format!(r#"{{"city":"{city}","population":"9,733,000"}}"#))
			})
			.collect();
		messages.push(ChatMessage::from(tool_calls));
		messages.push(ChatMessage::from(tool_responses));
	}

	Ok(usages)
}

/// Arm B: the whole transcript is resent every turn, the generateContent / chat-completions way.
async fn run_stateless(
	client: &Client,
	options: &ChatOptions,
	marker: &str,
) -> Result<Vec<TurnUsage>, Box<dyn std::error::Error>> {
	println!("Arm B — stateless (full history resent, store=false)");
	let mut usages = Vec::new();
	let mut messages: Vec<ChatMessage> = Vec::new();

	for (turn, question) in QUESTIONS.iter().enumerate() {
		let user_message = match turn {
			0 => ChatMessage::user(format!("{}\n\n{question}", big_document(marker))),
			_ => ChatMessage::user(*question),
		};
		messages.push(user_message);

		let chat_req = ChatRequest::new(messages.clone()).with_store(false);
		let res = client.exec_chat(&model(), chat_req, Some(options)).await?;

		// Carry the assistant turn forward, the way a generateContent caller would.
		if let Some(text) = res.first_text() {
			messages.push(ChatMessage::assistant(text));
		}

		let usage = read_usage(&res);
		print_turn("B", turn + 1, usage);
		usages.push(usage);
	}

	Ok(usages)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::new()?;
	let options = ChatOptions::default().with_capture_raw_body(true);
	let run_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();

	// Implicit caching is opportunistic, so arm order is a real confound: whichever runs second
	// has had more wall-clock time for the service to settle. Run both orders to see through it.
	let order = std::env::var("ARM_ORDER").unwrap_or_else(|_| "AB".to_string());
	println!("model: {}   order: {order}   run: {run_id}\n", model());

	let marker_a = format!("A-{run_id}");
	let marker_b = format!("B-{run_id}");

	// Control arms: run on their own so they cannot borrow a cache entry from A or B.
	if order == "C" || order == "D" || order == "E" {
		let iterations = std::env::var("ITERATIONS").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
		let usages = if order == "E" {
			let stateful = std::env::var("MODE").is_ok_and(|m| m == "stateful");
			run_agentic_probe(&client, &options, &format!("E-{run_id}"), stateful, iterations).await?
		} else if order == "D" {
			run_varying_suffix_probe(&client, &options, &format!("D-{run_id}"), iterations).await?
		} else {
			run_repeat_probe(&client, &options, &format!("C-{run_id}"), iterations).await?
		};
		let (input, cached) = usages
			.iter()
			.fold((0, 0), |(input, cached), u| (input + u.input, cached + u.cached));
		let hits = usages.iter().filter(|u| u.cached > 0).count();
		println!("\n{:-<64}", "");
		println!(
			"Arm {order}: billed input {input:>7}   cached {cached:>7} ({:.1}%)   hits {hits}/{}",
			if input > 0 {
				cached as f64 / input as f64 * 100.0
			} else {
				0.0
			},
			usages.len()
		);
		return Ok(());
	}

	let (stateful_usages, stateless_usages) = if order == "BA" {
		let b = run_stateless(&client, &options, &marker_b).await?;
		println!();
		let a = run_stateful(&client, &options, &marker_a).await?;
		(a, b)
	} else {
		let a = run_stateful(&client, &options, &marker_a).await?;
		println!();
		let b = run_stateless(&client, &options, &marker_b).await?;
		(a, b)
	};

	// -- Summary
	println!("\n{:-<64}", "");
	let sum = |usages: &[TurnUsage]| -> (i64, i64) {
		usages
			.iter()
			.fold((0, 0), |(input, cached), u| (input + u.input, cached + u.cached))
	};
	let (a_input, a_cached) = sum(&stateful_usages);
	let (b_input, b_cached) = sum(&stateless_usages);

	println!("Arm A (stateful):  billed input {a_input:>7}   cached {a_cached:>7}");
	println!("Arm B (stateless): billed input {b_input:>7}   cached {b_cached:>7}");
	println!(
		"\nUncached input tokens — A: {}   B: {}",
		a_input - a_cached,
		b_input - b_cached
	);

	Ok(())
}
