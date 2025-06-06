use crate::chat::{ToolCall, Tool};
use super::ToolError;
use serde_json::Value;

/// Trait for types that can be used as GenAI tool parameters
pub trait GenAiTool: Sized {
    /// The name of the tool function
    fn tool_name() -> &'static str;
    
    /// Optional description of what this tool does
    fn tool_description() -> Option<&'static str>;
    
    /// Generate the JSON schema for this tool's parameters
    fn json_schema() -> Value;
    
    /// Convert from a ToolCall back to the typed parameters
    fn from_tool_call(tool_call: &ToolCall) -> Result<Self, ToolError>;
    
    /// Convert to a Tool instance for use with the existing API
    fn to_tool() -> Tool {
        Tool::new(Self::tool_name())
            .with_description(Self::tool_description().unwrap_or(""))
            .with_schema(Self::json_schema())
    }
} 