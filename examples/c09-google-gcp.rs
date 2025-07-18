use gcp_auth::{CustomServiceAccount, TokenProvider};
use genai::Client;
use genai::Headers;
use genai::ModelIden;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::{AuthData, AuthResolver};
use std::pin::Pin;
use std::sync::Arc;

const MODEL: &str = "gemini-2.0-flash";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	// Just an example of a data that will get captured (needs to be locable)
	let gcp_env_name: Arc<str> = "GCP_SERVICE_ACCOUNT".into();

	// -- Create the Async Auth resolve_fn closure
	let resolve_fn = move |model: ModelIden| -> Pin<
		Box<dyn Future<Output = Result<Option<AuthData>, genai::resolver::Error>> + Send + 'static>,
	> {
		let gcp_env_name = gcp_env_name.clone();
		Box::pin(async move {
			let gcp_json = std::env::var(&*gcp_env_name).map_err(|_err| genai::resolver::Error::ApiKeyEnvNotFound {
				env_name: gcp_env_name.to_string(),
			})?;
			let account = CustomServiceAccount::from_json(&gcp_json)
				.map_err(|e| genai::resolver::Error::Custom(e.to_string()))?;
			let scopes: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];
			let token = account
				.token(scopes)
				.await
				.map_err(|e| genai::resolver::Error::Custom(e.to_string()))?;
			let location = std::env::var("GCP_LOCATION").unwrap_or("us-central1".to_string());
			let project_id = account.project_id().ok_or_else(|| {
				genai::resolver::Error::Custom("GCP Auth: Service account has no project_id".to_string())
			})?;
			let url = format!(
				"https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
				location, project_id, location, model.model_name
			);

			let auth_value = format!("Bearer {}", token.as_str());
			let auth_header = Headers::from(("Authorization", auth_value));
			Ok(Some(AuthData::RequestOverride {
				headers: auth_header,
				url,
			}))
		})
	};

	// -- Create the AuthResolver
	let auth_resolver = AuthResolver::from_resolver_async_fn(resolve_fn);

	// -- Create Chat Client
	let client = Client::builder().with_auth_resolver(auth_resolver).build();

	// -- Create Chat Request
	let chat_request = ChatRequest::default().with_system("Answer in one sentence");
	let chat_request = chat_request.append_message(ChatMessage::user("Why is the sky blue?"));

	// -- Executed
	let stream = client.exec_chat_stream(MODEL, chat_request, None).await.unwrap();

	print_chat_stream(stream, None).await.unwrap();
	Ok(())
}
