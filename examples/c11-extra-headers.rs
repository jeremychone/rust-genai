//! Example demonstrating how to add extra headers to chat completion calls

use genai::Client;
use genai::chat::{ChatMessage, ChatRequest, ChatOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let _client = Client::default();

    // Create a simple chat request
    let _chat_req = ChatRequest::new(vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Hello, how are you?"),
    ]);

    // Example 1: Add extra headers using individual with_extra_header calls
    let options_individual = ChatOptions::default()
        .with_extra_header("X-Custom-App", "my-rust-app")
        .with_extra_header("X-Request-ID", "req-12345")
        .with_temperature(0.7);

    // Example 2: Add extra headers using with_extra_headers with a vec
    let headers = vec![
        ("X-Custom-App".to_string(), "my-rust-app".to_string()),
        ("X-Request-ID".to_string(), "req-67890".to_string()),
        ("X-User-Agent".to_string(), "GenAI-Rust-Client/1.0".to_string()),
    ];
    
    let options_batch = ChatOptions::default()
        .with_extra_headers(headers)
        .with_temperature(0.7);

    println!("Extra headers example - these would be included in HTTP requests:");
    println!("Individual headers: {:?}", options_individual.extra_headers);
    println!("Batch headers: {:?}", options_batch.extra_headers);

    // Note: To actually test with a real API, you would need valid API keys
    // For now, we'll just show how the options would be configured
    println!("\nTo use these options in a real chat call:");
    println!("let response = client.exec_chat(\"gpt-4o-mini\", chat_req, Some(&options_individual)).await?;");

    Ok(())
}