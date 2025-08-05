//! Data files / constants for tests

use base64::Engine;
use base64::engine::general_purpose;
use std::fs::File;
use std::io::Read;

pub const IMAGE_URL_JPG_DUCK: &str = "https://upload.wikimedia.org/wikipedia/commons/thumb/b/bf/Bucephala-albeola-010.jpg/440px-Bucephala-albeola-010.jpg";

/// Get the base64 of the image above (but resized/lower to fit 5kb)
pub fn get_b64_duck() -> Result<String, Box<dyn std::error::Error>> {
	get_b64_file("./tests/data/duck-small.jpg")
}

pub fn get_b64_pdf() -> Result<String, Box<dyn std::error::Error>> {
	get_b64_file("./tests/data/small.pdf")
}

pub fn get_b64_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
	// Open the file and read its contents into a buffer
	let mut file = File::open(file_path)?;
	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer)?;

	// Use the general-purpose Base64 engine for encoding
	let base64_encoded = general_purpose::STANDARD.encode(&buffer);

	Ok(base64_encoded)
}
