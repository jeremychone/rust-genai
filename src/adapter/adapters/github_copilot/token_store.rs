use crate::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::copilot_types::PersistedOAuthToken;

// region:    --- Token Store

#[derive(Debug, Clone)]
pub struct CopilotTokenStore {
	path: PathBuf,
}

impl CopilotTokenStore {
	pub fn new() -> Result<Self> {
		let base = config_base_dir().ok_or_else(|| {
			Error::Internal(
				"Cannot determine config directory: no HOME, XDG_CONFIG_HOME, APPDATA, USERPROFILE, or LOCALAPPDATA env var set"
					.to_string(),
			)
		})?;
		Ok(Self::with_path(default_token_path(&base)))
	}

	pub fn with_path(path: PathBuf) -> Self {
		Self { path }
	}

	pub fn token_path(&self) -> &Path {
		&self.path
	}

	pub fn save(&self, token: &PersistedOAuthToken) -> Result<()> {
		if let Some(parent) = self.path.parent() {
			fs::create_dir_all(parent).map_err(|err| Error::Internal(format!("{}", err)))?;
		}

		let json = serde_json::to_string_pretty(token).map_err(|err| Error::Internal(format!("{}", err)))?;

		#[cfg(unix)]
		{
			use std::io::Write;
			use std::os::unix::fs::OpenOptionsExt;

			let tmp_path = self.path.with_extension("tmp");
			let mut file = fs::OpenOptions::new()
				.write(true)
				.create(true)
				.truncate(true)
				.mode(0o600)
				.open(&tmp_path)
				.map_err(|err| Error::Internal(format!("{}", err)))?;
			file.write_all(json.as_bytes())
				.map_err(|err| Error::Internal(format!("{}", err)))?;
			file.flush().map_err(|err| Error::Internal(format!("{}", err)))?;
			drop(file);
			fs::rename(&tmp_path, &self.path).map_err(|err| Error::Internal(format!("{}", err)))?;
		}

		#[cfg(not(unix))]
		{
			fs::write(&self.path, json).map_err(|err| Error::Internal(format!("{}", err)))?;
		}

		Ok(())
	}

	pub fn load(&self) -> Result<Option<PersistedOAuthToken>> {
		let json = match fs::read_to_string(&self.path) {
			Ok(json) => json,
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(err) => return Err(Error::Internal(format!("{}", err))),
		};

		let token = serde_json::from_str(&json).map_err(|err| Error::Internal(format!("{}", err)))?;
		Ok(Some(token))
	}

	pub fn clear(&self) -> Result<()> {
		match fs::remove_file(&self.path) {
			Ok(()) => Ok(()),
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
			Err(err) => Err(Error::Internal(format!("{}", err))),
		}
	}
}

fn config_base_dir() -> Option<PathBuf> {
	if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
		&& !xdg.is_empty()
	{
		return Some(PathBuf::from(xdg));
	}

	#[cfg(unix)]
	if let Ok(home) = std::env::var("HOME")
		&& !home.is_empty()
	{
		return Some(PathBuf::from(home).join(".config"));
	}

	#[cfg(windows)]
	{
		if let Ok(appdata) = std::env::var("APPDATA")
			&& !appdata.is_empty()
		{
			return Some(PathBuf::from(appdata));
		}

		if let Ok(userprofile) = std::env::var("USERPROFILE")
			&& !userprofile.is_empty()
		{
			return Some(PathBuf::from(userprofile).join("AppData").join("Roaming"));
		}

		if let Ok(localappdata) = std::env::var("LOCALAPPDATA")
			&& !localappdata.is_empty()
		{
			return Some(PathBuf::from(localappdata));
		}
	}

	None
}

fn default_token_path(base: &Path) -> PathBuf {
	base.join("genai/github-copilot/oauth_token.json")
}

// endregion: --- Token Store

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	fn temp_store() -> (CopilotTokenStore, PathBuf) {
		let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("time").as_nanos();
		let path = std::env::temp_dir().join(format!("genai_copilot_token_store_{nonce}_{}.json", std::process::id()));
		(CopilotTokenStore::with_path(path.clone()), path)
	}

	#[test]
	fn test_save_load_roundtrip() {
		let (store, path) = temp_store();
		let token = PersistedOAuthToken {
			access_token: "ghu_test".to_string(),
			token_type: "bearer".to_string(),
			scope: "read:user".to_string(),
			created_at: 1234567890,
		};

		store.save(&token).unwrap();
		let loaded = store.load().unwrap().unwrap();

		assert_eq!(loaded.access_token, token.access_token);
		assert_eq!(loaded.token_type, token.token_type);
		assert_eq!(loaded.scope, token.scope);
		assert_eq!(loaded.created_at, token.created_at);

		let _ = fs::remove_file(&path);
	}

	#[test]
	fn test_load_missing_file() {
		let (store, path) = temp_store();
		let _ = fs::remove_file(&path);

		assert!(store.load().unwrap().is_none());
	}

	#[cfg(unix)]
	#[test]
	fn test_file_permissions() {
		use std::os::unix::fs::PermissionsExt;

		let (store, path) = temp_store();
		let token = PersistedOAuthToken {
			access_token: "ghu_test".to_string(),
			token_type: "bearer".to_string(),
			scope: "read:user".to_string(),
			created_at: 1234567890,
		};

		store.save(&token).unwrap();
		let mode = fs::metadata(&path).unwrap().permissions().mode();
		assert_eq!(mode & 0o777, 0o600);

		let _ = fs::remove_file(&path);
	}
}

// endregion: --- Tests
