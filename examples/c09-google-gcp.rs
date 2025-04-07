use gcp_auth::{CustomServiceAccount, TokenProvider};
use genai::adapter::AdapterKind;
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest};
use genai::resolver::Error;
use genai::resolver::{AuthData, AuthResolver};
use genai::Client;
use genai::ModelIden;

const MODEL: &str = "gemini-2.0-flash";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
	let resolve_fn = async |model: ModelIden| {
		if model.adapter_kind != AdapterKind::Gemini {
			return Ok(None);
		}
		let gcp_json = std::env::var("GCP_SERVICE_ACCOUNT").map_err(|_err| Error::ApiKeyEnvNotFound {
			env_name: "GCP_SERVICE_ACCOUNT".to_string(),
		})?;
		let account = CustomServiceAccount::from_json(&gcp_json).map_err(|e| Error::External(Box::new(e)))?;
		let scopes: &[&str] = &["https://www.googleapis.com/auth/cloud-platform"];
		let token = account.token(scopes).await.map_err(|e| Error::External(Box::new(e)))?;
		let location = std::env::var("GCP_LOCATION").unwrap_or("us-central1".to_string());
		let project_id = account.project_id().ok_or_else(|| {
			genai::resolver::Error::External(Box::new(gcp_auth::Error::Str(
				"GCP Auth: Service account has no project_id",
			)))
		})?;
		let url = format!(
      	"https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
      	location, project_id, location, model.model_name
      );
		let auth_header = vec![("Authorization".to_string(), format!("Bearer {}", token.as_str()))];
		Ok(Some(AuthData::RequestOverride {
			headers: auth_header,
			url,
		}))
	};
	let auth_resolver = AuthResolver::from_resolver_async_fn(resolve_fn);
	let chat_request = ChatRequest::default().with_system("Answer in one sentence");
	let chat_request = chat_request.append_message(ChatMessage::user("Why is the sky blue?"));
	let client = Client::builder().with_auth_resolver(auth_resolver).build();
	let stream = client.exec_chat_stream(MODEL, chat_request, None).await.unwrap();

	print_chat_stream(stream, None).await.unwrap();
	Ok(())
}
