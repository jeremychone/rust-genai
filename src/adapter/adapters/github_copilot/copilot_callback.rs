// region:    --- CopilotAuthCallback

pub trait CopilotAuthCallback: Send + Sync {
	fn on_device_code(&self, user_code: &str, verification_uri: &str);
}

pub struct PrintCopilotCallback;

impl CopilotAuthCallback for PrintCopilotCallback {
	fn on_device_code(&self, user_code: &str, verification_uri: &str) {
		eprintln!(
			"\n=== GitHub Copilot Authentication ===\nOpen: {verification_uri}\nEnter code: {user_code}\nWaiting for authorization...\n"
		);
	}
}

impl<F> CopilotAuthCallback for F
where
	F: Fn(&str, &str) + Send + Sync,
{
	fn on_device_code(&self, user_code: &str, verification_uri: &str) {
		self(user_code, verification_uri)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::{Arc, Mutex};

	#[test]
	fn test_closure_as_callback() {
		let stored = Arc::new(Mutex::new(String::new()));
		let stored_clone = Arc::clone(&stored);

		let callback = move |user_code: &str, _verification_uri: &str| {
			*stored_clone.lock().unwrap() = user_code.to_string();
		};

		callback.on_device_code("TEST-CODE", "https://example.com");

		assert_eq!(stored.lock().unwrap().as_str(), "TEST-CODE");
	}
}

// endregion: --- CopilotAuthCallback
