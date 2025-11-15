use std::error::Error;
use std::sync::Arc;

use genai::adapter::AdapterKind;
use genai::resolver::{Endpoint, ServiceTargetResolver};
use genai::{Client, ServiceTarget};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn test_ollama_all_model_names_uses_service_target_resolver() -> Result<(), Box<dyn Error>> {
	// Arrange: start a tiny HTTP server that returns a synthetic models list.
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	let server_task = tokio::spawn(async move {
		if let Ok((mut socket, _)) = listener.accept().await {
			let mut buf = [0_u8; 1024];
			let _ = socket.read(&mut buf).await;
			let body = r#"{"data":[{"id":"custom-model"}]}"#;
			let response = format!(
				"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
				body.len(),
				body
			);
			let _ = socket.write_all(response.as_bytes()).await;
			let _ = socket.shutdown().await;
		}
	});

	// Arrange: build a client whose resolver rewrites the Ollama endpoint to the mock server.
	let endpoint_url: Arc<str> = format!("http://{addr}/").into();
	let resolver = ServiceTargetResolver::from_resolver_fn(move |mut target: ServiceTarget| {
		target.endpoint = Endpoint::from_owned(endpoint_url.clone());
		Ok(target)
	});

	let client = Client::builder().with_service_target_resolver(resolver).build();

	// Act: query the Ollama adapter for model names.
	let models = timeout(Duration::from_secs(5), client.all_model_names(AdapterKind::Ollama)).await??;

	// Assert: the response comes from our local server and contains the mocked model.
	assert_eq!(models, vec!["custom-model".to_string()]);

	let _ = server_task.await;

	Ok(())
}
