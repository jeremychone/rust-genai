use genai::{Client, GenAiTool, chat::tool::GenAiTool as _};
use genai::chat::printer::print_chat_stream;
use genai::chat::{ChatMessage, ChatRequest, ToolResponse};
use serde::{Deserialize, Serialize};

const MODEL: &str = "gpt-4o-mini"; // or "gemini-2.0-flash" or other model supporting tool calls

// Define weather parameters using the new derive macro
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

// Weather response structure
#[derive(Serialize, Debug)]
struct WeatherResponse {
    temperature: f64,
    condition: String,
    humidity: u32,
}

// Simulate a weather API call
async fn get_weather_data(params: &WeatherParams) -> WeatherResponse {
    // In a real app, you would call an actual weather API here
    println!("Fetching weather for {} in {} (unit: {})", params.city, params.country, params.unit);
    
    WeatherResponse {
        temperature: 22.5,
        condition: "Sunny".to_string(),
        humidity: 65,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::default();

    // 1. Create chat request using the typed tool API
    let chat_req = ChatRequest::new(vec![
        ChatMessage::user("What's the weather like in Tokyo, Japan?")
    ])
    .with_typed_tool::<WeatherParams>();

    // 2. Make the initial call to get the function call
    println!("--- Getting function call from model");
    let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;

    // 3. Extract the tool calls from the response
    let tool_calls = chat_res.into_tool_calls().ok_or("Expected tool calls in the response")?;

    println!("--- Tool calls received:");
    for tool_call in &tool_calls {
        println!("Function: {}", tool_call.fn_name);
        println!("Arguments: {}", tool_call.fn_arguments);
        
        // 4. Parse tool call arguments using type-safe deserialization
        if tool_call.fn_name == WeatherParams::tool_name() {
            match WeatherParams::from_tool_call(tool_call) {
                Ok(params) => {
                    println!("Parsed parameters: {:?}", params);
                    
                    // 5. Execute the tool function with typed parameters
                    let weather_response = get_weather_data(&params).await;
                    println!("Weather response: {:?}", weather_response);
                    
                    // 6. Create tool response
                    let tool_response = ToolResponse::new(
                        tool_call.call_id.clone(),
                        serde_json::to_string(&weather_response)?,
                    );
                    
                    // 7. Add the tool response to chat history and get final response
                    let chat_req = chat_req.clone()
                        .append_message(tool_calls.clone())
                        .append_message(tool_response);
                    
                    println!("\n--- Getting final response with function results");
                    let chat_res = client.exec_chat_stream(MODEL, chat_req, None).await?;
                    
                    println!("\n--- Final response:");
                    print_chat_stream(chat_res, None).await?;
                }
                Err(e) => {
                    eprintln!("Failed to parse tool parameters: {}", e);
                }
            }
        }
    }

    Ok(())
}

// This example demonstrates:
// 1. Defining tool parameters using #[derive(GenAiTool)]
// 2. Automatic JSON schema generation
// 3. Type-safe tool parameter parsing
// 4. Integration with the existing tool system
// 5. Clean error handling with typed errors 