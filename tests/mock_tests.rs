//! Mock server integration tests using wiremock

use serial_test::serial;
use uuid::Uuid;
use wiremock::{
	Mock, MockServer, ResponseTemplate,
	matchers::{header, method, path},
};

/// Generate a mock message ID
fn generate_message_id() -> String {
	format!("msg_{}", Uuid::new_v4().simple())
}

/// Generate a mock chat completion ID
fn generate_chat_id() -> String {
	format!("chatcmpl-{}", Uuid::new_v4().simple())
}

/// Create a standard success response structure
fn create_standard_usage() -> serde_json::Value {
	serde_json::json!({
		"prompt_tokens": 10,
		"completion_tokens": 5,
		"total_tokens": 15
	})
}

/// Create Anthropic-style response
fn create_anthropic_response() -> serde_json::Value {
	serde_json::json!({
		"id": generate_message_id(),
		"type": "message",
		"role": "assistant",
		"content": [{"type": "text", "text": "Hello! I'm a mock Anthropic response."}],
		"model": "claude-3-5-haiku-latest",
		"stop_reason": "end_turn",
		"stop_sequence": null,
		"usage": create_standard_usage()
	})
}

/// Create OpenRouter-style response
fn create_openrouter_response() -> serde_json::Value {
	serde_json::json!({
		"id": generate_chat_id(),
		"object": "chat.completion",
		"created": 1234567890,
		"model": "anthropic/claude-3.5-sonnet",
		"choices": [{
			"index": 0,
			"message": {
				"role": "assistant",
				"content": "Hello! I'm a mock OpenRouter response."
			},
			"finish_reason": "stop"
		}],
		"usage": create_standard_usage()
	})
}

/// Create Anthropic tool response
fn create_anthropic_tool_response() -> serde_json::Value {
	serde_json::json!({
		"id": generate_message_id(),
		"type": "message",
		"role": "assistant",
		"content": [{
			"type": "tool_use",
			"id": format!("toolu_{}", Uuid::new_v4().simple()),
			"name": "get_weather",
			"input": {
				"location": "Paris",
				"unit": "celsius"
			}
		}],
		"model": "claude-3-5-haiku-latest",
		"stop_reason": "tool_use",
		"stop_sequence": null,
		"usage": create_standard_usage()
	})
}

#[tokio::test]
#[serial]
async fn test_anthropic_mock_server_basic() {
	let mock_server = MockServer::start().await;

	// Mock the messages endpoint
	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(200).set_body_json(create_anthropic_response()))
		.mount(&mock_server)
		.await;

	// Test basic request
	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 200);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "message");
	assert_eq!(json["role"], "assistant");
	assert_eq!(json["content"][0]["text"], "Hello! I'm a mock Anthropic response.");
}

#[tokio::test]
#[serial]
async fn test_openrouter_mock_server_basic() {
	let mock_server = MockServer::start().await;

	// Mock the chat completions endpoint
	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer test-key"))
		.respond_with(ResponseTemplate::new(200).set_body_json(create_openrouter_response()))
		.mount(&mock_server)
		.await;

	// Test basic request
	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 200);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["object"], "chat.completion");
	assert_eq!(json["choices"][0]["message"]["role"], "assistant");
	assert_eq!(
		json["choices"][0]["message"]["content"],
		"Hello! I'm a mock OpenRouter response."
	);
}

#[tokio::test]
#[serial]
async fn test_anthropic_tool_call() {
	let mock_server = MockServer::start().await;

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(200).set_body_json(create_anthropic_tool_response()))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "What's the weather?"}],
			"tools": [{
				"name": "get_weather",
				"description": "Get weather information",
				"input_schema": {
					"type": "object",
					"properties": {
						"location": {"type": "string"},
						"unit": {"type": "string"}
					},
					"required": ["location"]
				}
			}],
			"max_tokens": 100
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 200);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["content"][0]["type"], "tool_use");
	assert_eq!(json["content"][0]["name"], "get_weather");
	assert_eq!(json["content"][0]["input"]["location"], "Paris");
}

#[tokio::test]
#[serial]
async fn test_anthropic_streaming() {
	let mock_server = MockServer::start().await;

	Mock::given(method("POST"))
        .and(path("/v1/messages/beta/stream"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "event: message_start\ndata: {\"type\": \"message_start\"}\n\nevent: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"delta\": {\"text\": \"Hello\"}}\n\nevent: message_stop\ndata: {\"type\": \"message_stop\"}\n\n"
        ))
        .mount(&mock_server)
        .await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages/beta/stream", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 200);
	// Note: wiremock may not preserve content-type header exactly
}

#[tokio::test]
#[serial]
async fn test_openrouter_streaming() {
	let mock_server = MockServer::start().await;

	Mock::given(method("POST"))
        .and(path("/api/v1/chat/completions/stream"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"id\": \"chatcmpl-...\", \"object\": \"chat.completion.chunk\", \"choices\": [{\"index\": 0, \"delta\": {\"content\": \"Hello\"}}]}\n\ndata: [DONE]\n\n"
        ))
        .mount(&mock_server)
        .await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions/stream", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Hello"}],
			"stream": true,
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 200);
	// Note: wiremock may not preserve content-type header exactly
}

#[tokio::test]
#[serial]
async fn test_anthropic_auth_error() {
	let mock_server = MockServer::start().await;

	let error_response = serde_json::json!({
		"type": "error",
		"error": {
			"type": "authentication_error",
			"message": "Invalid API key"
		}
	});

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "invalid-key"))
		.respond_with(ResponseTemplate::new(401).set_body_json(error_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "invalid-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 401);
}

#[tokio::test]
#[serial]
async fn test_openrouter_auth_error() {
	let mock_server = MockServer::start().await;

	let error_response = serde_json::json!({
		"error": {
			"message": "Invalid API key",
			"type": "invalid_api_key",
			"code": "invalid_api_key"
		}
	});

	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer invalid-key"))
		.respond_with(ResponseTemplate::new(401).set_body_json(error_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer invalid-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 401);
}

#[tokio::test]
#[serial]
async fn test_anthropic_json_mode() {
	let mock_server = MockServer::start().await;

	let json_response = serde_json::json!({
		"id": generate_message_id(),
		"type": "message",
		"role": "assistant",
		"content": [{"type": "text", "text": "{\"colors\": [\"red\", \"green\", \"blue\"]}"}],
		"model": "claude-3-5-haiku-latest",
		"stop_reason": "end_turn",
		"stop_sequence": null,
		"usage": create_standard_usage()
	});

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "List 3 colors in JSON format"}],
			"response_format": {"type": "json_object"},
			"max_tokens": 100
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 200);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "message");
	assert_eq!(
		json["content"][0]["text"],
		"{\"colors\": [\"red\", \"green\", \"blue\"]}"
	);
}

// ===== ENHANCED ERROR SCENARIO TESTS =====

#[tokio::test]
#[serial]
async fn test_anthropic_rate_limit_error() {
	let mock_server = MockServer::start().await;

	let rate_limit_response = serde_json::json!({
		"type": "error",
		"error": {
			"type": "rate_limit_error",
			"message": "Rate limit exceeded. Please try again later.",
			"error": {
				"type": "rate_limit_error",
				"message": "Rate limit exceeded"
			}
		}
	});

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(
			ResponseTemplate::new(429)
				.set_body_json(rate_limit_response)
				.insert_header("Retry-After", "60"),
		)
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 429);

	// Check retry-after header
	let retry_after = response.headers().get("Retry-After").unwrap();
	assert_eq!(retry_after, "60");

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "error");
	assert_eq!(json["error"]["type"], "rate_limit_error");
}

#[tokio::test]
#[serial]
async fn test_openrouter_rate_limit_error() {
	let mock_server = MockServer::start().await;

	let rate_limit_response = serde_json::json!({
		"error": {
			"message": "Rate limit exceeded. Please try again later.",
			"type": "rate_limit_exceeded",
			"code": "rate_limit_exceeded"
		}
	});

	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer test-key"))
		.respond_with(
			ResponseTemplate::new(429)
				.set_body_json(rate_limit_response)
				.insert_header("Retry-After", "30"),
		)
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 429);

	// Check retry-after header
	let retry_after = response.headers().get("Retry-After").unwrap();
	assert_eq!(retry_after, "30");

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["error"]["type"], "rate_limit_exceeded");
}

#[tokio::test]
#[serial]
async fn test_anthropic_invalid_request_error() {
	let mock_server = MockServer::start().await;

	let invalid_request_response = serde_json::json!({
		"type": "error",
		"error": {
			"type": "invalid_request_error",
			"message": "Invalid request: model 'invalid-model' not found",
			"error": {
				"type": "invalid_request_error",
				"message": "model 'invalid-model' not found"
			}
		}
	});

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(400).set_body_json(invalid_request_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "invalid-model",
			"messages": [{"role": "invalid", "content": 123}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 400);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "error");
	assert_eq!(json["error"]["type"], "invalid_request_error");
	assert!(json["error"]["message"].as_str().unwrap().contains("invalid-model"));
}

#[tokio::test]
#[serial]
async fn test_openrouter_invalid_request_error() {
	let mock_server = MockServer::start().await;

	let invalid_request_response = serde_json::json!({
		"error": {
			"message": "Invalid request: model 'invalid-model' not found",
			"type": "invalid_request_error",
			"code": "model_not_found"
		}
	});

	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer test-key"))
		.respond_with(ResponseTemplate::new(400).set_body_json(invalid_request_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "invalid-model",
			"messages": [{"role": "invalid", "content": 123}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 400);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["error"]["type"], "invalid_request_error");
	assert_eq!(json["error"]["code"], "model_not_found");
}

#[tokio::test]
#[serial]
async fn test_anthropic_server_error() {
	let mock_server = MockServer::start().await;

	let server_error_response = serde_json::json!({
		"type": "error",
		"error": {
			"type": "api_error",
			"message": "Internal server error. Please try again.",
			"error": {
				"type": "api_error",
				"message": "Internal server error"
			}
		}
	});

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(500).set_body_json(server_error_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 500);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "error");
	assert_eq!(json["error"]["type"], "api_error");
}

#[tokio::test]
#[serial]
async fn test_openrouter_server_error() {
	let mock_server = MockServer::start().await;

	let server_error_response = serde_json::json!({
		"error": {
			"message": "Internal server error. Please try again.",
			"type": "internal_server_error",
			"code": "internal_error"
		}
	});

	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer test-key"))
		.respond_with(ResponseTemplate::new(500).set_body_json(server_error_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 500);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["error"]["type"], "internal_server_error");
	assert_eq!(json["error"]["code"], "internal_error");
}

#[tokio::test]
#[serial]
async fn test_anthropic_timeout_error() {
	let mock_server = MockServer::start().await;

	// Simulate timeout by not responding and using a timeout template
	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(408).set_body_json(serde_json::json!({
			"type": "error",
			"error": {
				"type": "timeout_error",
				"message": "Request timeout. Please try again.",
				"error": {
					"type": "timeout_error",
					"message": "Request timeout"
				}
			}
		})))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 408);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "error");
	assert_eq!(json["error"]["type"], "timeout_error");
}

#[tokio::test]
#[serial]
async fn test_openrouter_timeout_error() {
	let mock_server = MockServer::start().await;

	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer test-key"))
		.respond_with(ResponseTemplate::new(408).set_body_json(serde_json::json!({
			"error": {
				"message": "Request timeout. Please try again.",
				"type": "timeout",
				"code": "request_timeout"
			}
		})))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Hello"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 408);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["error"]["type"], "timeout");
	assert_eq!(json["error"]["code"], "request_timeout");
}

#[tokio::test]
#[serial]
async fn test_anthropic_content_filter_error() {
	let mock_server = MockServer::start().await;

	let content_filter_response = serde_json::json!({
		"type": "error",
		"error": {
			"type": "content_filter",
			"message": "Content filtered due to policy violation.",
			"error": {
				"type": "content_filter",
				"message": "Content policy violation"
			}
		}
	});

	Mock::given(method("POST"))
		.and(path("/v1/messages"))
		.and(header("x-api-key", "test-key"))
		.respond_with(ResponseTemplate::new(400).set_body_json(content_filter_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/v1/messages", mock_server.uri()))
		.header("x-api-key", "test-key")
		.json(&serde_json::json!({
			"model": "claude-3-5-haiku-latest",
			"messages": [{"role": "user", "content": "Inappropriate content"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 400);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["type"], "error");
	assert_eq!(json["error"]["type"], "content_filter");
}

#[tokio::test]
#[serial]
async fn test_openrouter_content_filter_error() {
	let mock_server = MockServer::start().await;

	let content_filter_response = serde_json::json!({
		"error": {
			"message": "Content filtered due to policy violation.",
			"type": "content_filter",
			"code": "content_policy_violation"
		}
	});

	Mock::given(method("POST"))
		.and(path("/api/v1/chat/completions"))
		.and(header("authorization", "Bearer test-key"))
		.respond_with(ResponseTemplate::new(400).set_body_json(content_filter_response))
		.mount(&mock_server)
		.await;

	let client = reqwest::Client::new();
	let response = client
		.post(format!("{}/api/v1/chat/completions", mock_server.uri()))
		.header("authorization", "Bearer test-key")
		.json(&serde_json::json!({
			"model": "anthropic/claude-3.5-sonnet",
			"messages": [{"role": "user", "content": "Inappropriate content"}],
			"max_tokens": 10
		}))
		.send()
		.await
		.unwrap();

	assert_eq!(response.status(), 400);

	let json: serde_json::Value = response.json().await.unwrap();
	assert_eq!(json["error"]["type"], "content_filter");
	assert_eq!(json["error"]["code"], "content_policy_violation");
}
