use derive_more::Display;

/// Error types for tool operations
#[derive(Debug, Display)]
pub enum ToolError {
    /// Tool parameter deserialization failed
    #[display("Tool parameter deserialization failed: {_0}")]
    DeserializationError(serde_json::Error),
    
    /// Tool validation failed
    #[display("Tool validation failed: {field} - {message}")]
    ValidationError { field: String, message: String },
    
    /// Tool not found in registry
    #[display("Tool '{name}' not found in registry")]
    ToolNotFound { name: String },
    
    /// Tool execution failed
    #[display("Tool execution failed: {_0}")]
    ExecutionError(String),
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        ToolError::DeserializationError(err)
    }
}

impl std::error::Error for ToolError {} 