//! Macros for adapter dispatching.

use paste::paste;

// Dispatch adapter method call based on `AdapterKind`, avoiding repeated `match` arms.
//
// Usage:
// ```ignore
// dispatch_adapter!(kind, A::some_method(args))
// ```
//
// Inside the provided expression, a type alias `A` is bound to the appropriate
// concrete adapter struct (e.g., `OpenAIAdapter`), allowing static dispatch.
//
// The macro contains the full mapping from every `AdapterKind` variant to
// its corresponding adapter struct, using fully qualified paths.

paste! {
macro_rules! dispatch_adapter {
	($kind:expr, $body:expr) => {
		match $kind {
			crate::adapter::AdapterKind::OpenAI => {
				type A = crate::adapter::adapters::all_adapters::[<OpenAI Adapter>];
				$body
			}
			crate::adapter::AdapterKind::OpenAIResp => {
				type A = crate::adapter::adapters::all_adapters::[<OpenAIResp Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Gemini => {
				type A = crate::adapter::adapters::all_adapters::[<Gemini Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Anthropic => {
				type A = crate::adapter::adapters::all_adapters::[<Anthropic Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Fireworks => {
				type A = crate::adapter::adapters::all_adapters::[<Fireworks Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Together => {
				type A = crate::adapter::adapters::all_adapters::[<Together Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Groq => {
				type A = crate::adapter::adapters::all_adapters::[<Groq Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Aihubmix => {
				type A = crate::adapter::adapters::all_adapters::[<Aihubmix Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Kimi => {
				type A = crate::adapter::adapters::all_adapters::[<Kimi Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Moonshot => {
				type A = crate::adapter::adapters::all_adapters::[<Moonshot Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Mimo => {
				type A = crate::adapter::adapters::all_adapters::[<Mimo Adapter>];
				$body
			}
			crate::adapter::AdapterKind::MiniMax => {
				type A = crate::adapter::adapters::all_adapters::[<Minimax Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Nebius => {
				type A = crate::adapter::adapters::all_adapters::[<Nebius Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Xai => {
				type A = crate::adapter::adapters::all_adapters::[<Xai Adapter>];
				$body
			}
			crate::adapter::AdapterKind::DeepSeek => {
				type A = crate::adapter::adapters::all_adapters::[<DeepSeek Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Zai => {
				type A = crate::adapter::adapters::all_adapters::[<Zai Adapter>];
				$body
			}
			crate::adapter::AdapterKind::BigModel => {
				type A = crate::adapter::adapters::all_adapters::[<BigModel Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Aliyun => {
				type A = crate::adapter::adapters::all_adapters::[<Aliyun Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Baidu => {
				type A = crate::adapter::adapters::all_adapters::[<Baidu Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Cohere => {
				type A = crate::adapter::adapters::all_adapters::[<Cohere Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Ollama => {
				type A = crate::adapter::adapters::all_adapters::[<Ollama Adapter>];
				$body
			}
			crate::adapter::AdapterKind::OllamaCloud => {
				type A = crate::adapter::adapters::all_adapters::[<OllamaCloud Adapter>];
				$body
			}
			crate::adapter::AdapterKind::Vertex => {
				type A = crate::adapter::adapters::all_adapters::[<Vertex Adapter>];
				$body
			}
			crate::adapter::AdapterKind::GithubCopilot => {
				type A = crate::adapter::adapters::all_adapters::[<GithubCopilot Adapter>];
				$body
			}
			crate::adapter::AdapterKind::OpenCodeGo => {
				type A = crate::adapter::adapters::all_adapters::[<OpenCodeGo Adapter>];
				$body
			}
			crate::adapter::AdapterKind::BedrockApi => {
				type A = crate::adapter::adapters::all_adapters::[<BedrockApi Adapter>];
				$body
			}
			#[cfg(feature = "bedrock-sigv4")]
			crate::adapter::AdapterKind::BedrockSigv4 => {
				type A = crate::adapter::adapters::all_adapters::[<BedrockSigv4 Adapter>];
				$body
			}
			crate::adapter::AdapterKind::OpenRouter => {
				type A = crate::adapter::adapters::all_adapters::[<OpenRouter Adapter>];
				$body
			}

			crate::adapter::AdapterKind::Custom(_) => {
				type A = crate::adapter::adapters::all_adapters::[<Custom Adapter>];
				$body
			}
		}
	};
}
}

pub(crate) use dispatch_adapter;
