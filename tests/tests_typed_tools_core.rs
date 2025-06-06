mod support;

use genai::{GenAiTool, chat::tool::GenAiTool as _};
use genai::chat::{ChatMessage, ChatRequest, Tool, ToolCall};
use serde::{Deserialize, Serialize};
use serde_json::json;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// region:    --- Test Fixtures

#[derive(GenAiTool, Deserialize, Serialize, Debug, Clone)]
#[tool(name = "get_weather", description = "Get the current weather for a location")]
struct WeatherParams {
    /// The city name
    city: String,
    /// The country of the city
    country: String,
    /// Temperature unit (C for Celsius, F for Fahrenheit)
    #[tool(enum_values = ["C", "F"])]
    unit: String,
}

#[derive(GenAiTool, Deserialize, Serialize, Debug, Clone)]
#[tool(name = "search", description = "Search for information")]
struct SearchParams {
    /// Search query (required)
    query: String,
    /// Maximum number of results (optional)
    limit: Option<u32>,
    /// Include images in results
    include_images: Option<bool>,
}

// endregion: --- Test Fixtures

// region:    --- Schema Generation Tests

#[tokio::test]
async fn test_schema_generation() -> Result<()> {
    // Test that schema generation works correctly
    let schema = WeatherParams::json_schema();
    
    // Verify schema structure
    assert_eq!(schema["type"], "object");
    
    let properties = schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("city"));
    assert!(properties.contains_key("country"));
    assert!(properties.contains_key("unit"));
    
    // Check field types
    assert_eq!(properties["city"]["type"], "string");
    assert_eq!(properties["country"]["type"], "string");
    assert_eq!(properties["unit"]["type"], "string");
    
    // Check enum constraint
    let unit_enum = &properties["unit"]["enum"];
    assert!(unit_enum.is_array());
    let enum_values = unit_enum.as_array().unwrap();
    assert_eq!(enum_values.len(), 2);
    assert!(enum_values.contains(&json!("C")));
    assert!(enum_values.contains(&json!("F")));
    
    Ok(())
}

#[tokio::test]
async fn test_optional_fields() -> Result<()> {
    // Test that optional fields are handled correctly
    let schema = SearchParams::json_schema();
    
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties["query"]["type"], "string");
    assert_eq!(properties["limit"]["type"], "integer");
    assert_eq!(properties["include_images"]["type"], "boolean");
    
    // Check required fields - only query should be required
    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 1);
    assert!(required.contains(&json!("query")));
    
    Ok(())
}

// endregion: --- Schema Generation Tests

// region:    --- Tool Trait Tests

#[tokio::test]
async fn test_tool_trait_methods() -> Result<()> {
    // Test all GenAiTool trait methods
    assert_eq!(WeatherParams::tool_name(), "get_weather");
    assert_eq!(WeatherParams::tool_description(), Some("Get the current weather for a location"));
    
    assert_eq!(SearchParams::tool_name(), "search");
    assert_eq!(SearchParams::tool_description(), Some("Search for information"));
    
    Ok(())
}

#[tokio::test]
async fn test_tool_conversion() -> Result<()> {
    // Test conversion to existing Tool struct
    let tool = WeatherParams::to_tool();
    
    assert_eq!(tool.name, "get_weather");
    assert_eq!(tool.description, Some("Get the current weather for a location".to_string()));
    assert!(tool.schema.is_some());
    
    // Verify the schema matches our generated schema
    let expected_schema = WeatherParams::json_schema();
    assert_eq!(tool.schema.unwrap(), expected_schema);
    
    Ok(())
}

// endregion: --- Tool Trait Tests

// region:    --- Deserialization Tests

#[tokio::test]
async fn test_tool_call_deserialization() -> Result<()> {
    // Test type-safe deserialization from ToolCall
    let tool_call = ToolCall {
        call_id: "test_call_123".to_string(),
        fn_name: "get_weather".to_string(),
        fn_arguments: json!({
            "city": "Tokyo",
            "country": "Japan",
            "unit": "C"
        }),
    };
    
    let params = WeatherParams::from_tool_call(&tool_call)?;
    
    assert_eq!(params.city, "Tokyo");
    assert_eq!(params.country, "Japan");
    assert_eq!(params.unit, "C");
    
    Ok(())
}

#[tokio::test]
async fn test_tool_call_deserialization_with_optional() -> Result<()> {
    // Test deserialization with optional fields
    let tool_call = ToolCall {
        call_id: "test_call_456".to_string(),
        fn_name: "search".to_string(),
        fn_arguments: json!({
            "query": "rust programming",
            "limit": 10
            // include_images is omitted (optional)
        }),
    };
    
    let params = SearchParams::from_tool_call(&tool_call)?;
    
    assert_eq!(params.query, "rust programming");
    assert_eq!(params.limit, Some(10));
    assert_eq!(params.include_images, None);
    
    Ok(())
}

#[tokio::test]
async fn test_tool_call_deserialization_error() -> Result<()> {
    // Test that invalid data produces proper errors
    let tool_call = ToolCall {
        call_id: "test_call_error".to_string(),
        fn_name: "get_weather".to_string(),
        fn_arguments: json!({
            "city": "Tokyo",
            "country": "Japan"
            // unit is missing (required field)
        }),
    };
    
    let result = WeatherParams::from_tool_call(&tool_call);
    assert!(result.is_err());
    
    Ok(())
}

// endregion: --- Deserialization Tests

// region:    --- Integration Tests

#[tokio::test]
async fn test_chat_request_with_typed_tool() -> Result<()> {
    // Test ChatRequest integration with typed tools
    let chat_req = ChatRequest::new(vec![
        ChatMessage::user("What's the weather in Paris?")
    ]).with_typed_tool::<WeatherParams>();
    
    // Verify tool was added
    assert!(chat_req.tools.is_some());
    let tools = chat_req.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 1);
    
    let tool = &tools[0];
    assert_eq!(tool.name, "get_weather");
    assert_eq!(tool.description, Some("Get the current weather for a location".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_mixed_manual_and_typed_tools() -> Result<()> {
    // Test that manual and typed tools can be mixed
    let manual_tool = Tool::new("manual_tool")
        .with_description("A manually defined tool")
        .with_schema(json!({
            "type": "object",
            "properties": {
                "param": {"type": "string"}
            }
        }));
    
    let chat_req = ChatRequest::new(vec![
        ChatMessage::user("Test message")
    ])
    .with_tools(vec![manual_tool])
    .with_typed_tool::<WeatherParams>();
    
    // Verify both tools are present
    assert!(chat_req.tools.is_some());
    let tools = chat_req.tools.as_ref().unwrap();
    assert_eq!(tools.len(), 2);
    
    // Check tool names
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(tool_names.contains(&"manual_tool"));
    assert!(tool_names.contains(&"get_weather"));
    
    Ok(())
}

// endregion: --- Integration Tests 