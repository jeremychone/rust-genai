use crate::chat::{ToolCall, ToolResponse};
use derive_more::derive::From;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

/// Note: MessageContent is use for the ChatRequest as well as the ChatResponse
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum MessageContent {
    /// Text content
    #[from(&str, &String, String)]
    Text(String),

    /// Content parts
    Parts(Vec<ContentPart>),

    /// Tool calls
    #[from]
    ToolCalls(Vec<ToolCall>),

    /// Tool call responses
    #[from]
    ToolResponses(Vec<ToolResponse>),
}

/// Constructors
impl MessageContent {
    /// Create a new MessageContent with the Text variant
    pub fn from_text(content: impl Into<String>) -> Self {
        MessageContent::Text(content.into())
    }

    /// Create a new MessageContent from provided content parts
    pub fn from_parts(parts: impl Into<Vec<ContentPart>>) -> Self {
        MessageContent::Parts(parts.into())
    }

    /// Create a new MessageContent with the ToolCalls variant
    pub fn from_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        MessageContent::ToolCalls(tool_calls)
    }
}

/// Getters
impl MessageContent {
    /// Returns the MessageContent as &str, only if it is MessageContent::Text
    /// Otherwise, it returns None.
    ///
    /// NOTE: When multi-part content is present, this will return None and won't concatenate the text parts.
    pub fn text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(content) => Some(content.as_str()),
            MessageContent::Parts(_) => None,
            MessageContent::ToolCalls(_) => None,
            MessageContent::ToolResponses(_) => None,
        }
    }

    /// Consumes the MessageContent and returns it as &str,
    /// only if it is MessageContent::Text; otherwise, it returns None.
    ///
    /// NOTE: When multi-part content is present, this will return None and won't concatenate the text parts.
    pub fn into_text(self) -> Option<String> {
        match self {
            MessageContent::Text(content) => Some(content),
            MessageContent::Parts(_) => None,
            MessageContent::ToolCalls(_) => None,
            MessageContent::ToolResponses(_) => None,
        }
    }

    pub fn tool_calls(&self) -> Option<Vec<&ToolCall>> {
        match self {
            MessageContent::ToolCalls(tool_calls) => Some(tool_calls.iter().collect()),
            _ => None,
        }
    }

    pub fn into_tool_calls(self) -> Option<Vec<ToolCall>> {
        match self {
            MessageContent::ToolCalls(tool_calls) => Some(tool_calls),
            _ => None,
        }
    }

    /// Checks if the text content or the tool calls are empty.
    pub fn is_empty(&self) -> bool {
        match self {
            MessageContent::Text(content) => content.is_empty(),
            MessageContent::Parts(parts) => parts.is_empty(),
            MessageContent::ToolCalls(tool_calls) => tool_calls.is_empty(),
            MessageContent::ToolResponses(tool_responses) => tool_responses.is_empty(),
        }
    }
}

// region:    --- Froms

impl From<ToolResponse> for MessageContent {
    fn from(tool_response: ToolResponse) -> Self {
        MessageContent::ToolResponses(vec![tool_response])
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        MessageContent::Parts(parts)
    }
}

// endregion: --- Froms

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum ContentPart {
    Text(String),
    Image {
        content_type: String,
        source: ImageSource,
    },
    Pdf(DocumentSource),
}

/// Constructors
impl ContentPart {
    pub fn from_text(text: impl Into<String>) -> ContentPart {
        ContentPart::Text(text.into())
    }

    pub fn from_image_base64(
        content_type: impl Into<String>,
        content: impl Into<Arc<str>>,
    ) -> ContentPart {
        ContentPart::Image {
            content_type: content_type.into(),
            source: ImageSource::Base64(content.into()),
        }
    }

    pub fn from_image_url(content_type: impl Into<String>, url: impl Into<String>) -> ContentPart {
        ContentPart::Image {
            content_type: content_type.into(),
            source: ImageSource::Url(url.into()),
        }
    }
}

// region:    --- Froms

impl<'a> From<&'a str> for ContentPart {
    fn from(s: &'a str) -> Self {
        ContentPart::Text(s.to_string())
    }
}

// endregion: --- Froms

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageSource {
    /// For models/services that support URL as input
    /// NOTE: Few AI services support this.
    Url(String),

    /// The base64 string of the image
    ///
    /// NOTE: Here we use an Arc<str> to avoid cloning large amounts of data when cloning a ChatRequest.
    ///       The overhead is minimal compared to cloning relatively large data.
    ///       The downside is that it will be an Arc even when used only once, but for this particular data type, the net benefit is positive.
    Base64(Arc<str>),
}

// No `Local` location; this would require handling errors like "file not found" etc.
// Such a file can be easily provided by the user as Base64, and we can implement a convenient
// TryFrom<File> to Base64 version. All LLMs accept local images only as Base64.

#[derive(Clone, Serialize, Deserialize)]
pub enum DocumentSource {
    Url(String),
    Base64(String),
}

impl Debug for DocumentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentSource::Url(url) => write!(f, "{}", url),
            DocumentSource::Base64(bytes) => write!(f, "<base64:{}>", bytes.len()),
        }
    }
}

impl Display for DocumentSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentSource::Url(url) => write!(f, "{}", url),
            DocumentSource::Base64(bytes) => write!(f, "<base64:{}>", bytes.len()),
        }
    }
}
