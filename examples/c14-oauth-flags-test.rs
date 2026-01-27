//! OAuth flags testing - determines which transformations are required vs optional.
//!
//! This example tests different combinations of OAuth config flags to identify:
//! - Which flags are REQUIRED for OAuth to work
//! - Which flags are OPTIONAL (can be disabled)
//!
//! Run with:
//! ```
//! cargo run --example c14-oauth-flags-test
//! ```
//!
//! Requires ANTHROPIC_OAUTH_TOKEN environment variable or .oauth_token.json file.

use genai::adapter::{OAuthConfig, OAuthRequestTransformer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

const MODEL: &str = "claude-sonnet-4-20250514";
const API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Debug, Serialize, Deserialize)]
struct SavedToken {
    access_token: String,
    refresh_token: Option<String>,
}

fn load_token() -> Option<String> {
    // Try environment variable first
    if let Ok(token) = std::env::var("ANTHROPIC_OAUTH_TOKEN") {
        return Some(token);
    }

    // Try saved token file
    let path = PathBuf::from(".oauth_token.json");
    if path.exists() {
        let content = std::fs::read_to_string(&path).ok()?;
        let saved: SavedToken = serde_json::from_str(&content).ok()?;
        return Some(saved.access_token);
    }

    None
}

/// Make a raw API call with the given payload
async fn make_api_call(access_token: &str, payload: Value) -> Result<(u16, String), String> {
    let client = reqwest::Client::new();
    let response = client
        .post(API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "oauth-2025-04-20")
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status().as_u16();
    let body = response.text().await.map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// Test a specific configuration
async fn test_config(
    access_token: &str,
    config_name: &str,
    config: &OAuthConfig,
    base_payload: Value,
) -> bool {
    println!("\n--- Testing: {} ---", config_name);
    println!("Config: {:?}", config);

    let payload = OAuthRequestTransformer::transform_with_config(base_payload, true, config);

    // Show transformed payload (truncated)
    let payload_str = serde_json::to_string_pretty(&payload).unwrap();
    if payload_str.len() > 500 {
        println!("Payload (truncated): {}...", &payload_str[..500]);
    } else {
        println!("Payload: {}", payload_str);
    }

    match make_api_call(access_token, payload).await {
        Ok((status, body)) => {
            println!("Status: {}", status);
            if status == 200 {
                // Parse response to show text content
                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
                        for item in content {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                println!("Response: {}", text);
                            }
                        }
                    }
                }
                println!("Result: ✅ SUCCESS");
                true
            } else {
                // Show error details
                if let Ok(json) = serde_json::from_str::<Value>(&body) {
                    if let Some(error) = json.get("error") {
                        println!("Error: {}", serde_json::to_string_pretty(error).unwrap());
                    }
                } else {
                    println!("Body: {}", body);
                }
                println!("Result: ❌ FAILED (status {})", status);
                false
            }
        }
        Err(e) => {
            println!("Request error: {}", e);
            println!("Result: ❌ ERROR");
            false
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OAuth Flags Test ===\n");
    println!("This test determines which OAuth config flags are required.\n");

    let access_token = load_token().ok_or("No OAuth token found. Set ANTHROPIC_OAUTH_TOKEN or create .oauth_token.json")?;

    println!("Token: {}...", &access_token[..30.min(access_token.len())]);

    // Base payload without any OAuth transformations
    let base_payload = json!({
        "model": MODEL,
        "max_tokens": 50,
        "messages": [{"role": "user", "content": "Say 'test OK' and nothing else."}]
    });

    let mut results: Vec<(&str, bool)> = Vec::new();

    // Test 1: Default config (all required flags enabled)
    {
        let config = OAuthConfig::default();
        let success = test_config(&access_token, "DEFAULT (all required flags)", &config, base_payload.clone()).await;
        results.push(("DEFAULT", success));
    }

    // Test 2: No transformations at all
    {
        let config = OAuthConfig::none();
        let success = test_config(&access_token, "NONE (no transformations)", &config, base_payload.clone()).await;
        results.push(("NONE", success));
    }

    // Test 3: Only system prompt (no cache_control, no tool prefix)
    {
        let config = OAuthConfig::none()
            .with_inject_system_prompt(true);
        let success = test_config(&access_token, "SYSTEM_PROMPT_ONLY (no cache_control)", &config, base_payload.clone()).await;
        results.push(("SYSTEM_PROMPT_ONLY", success));
    }

    // Test 4: System prompt + cache_control (no tool prefix)
    {
        let config = OAuthConfig::new()
            .with_prefix_tool_names(false);
        let success = test_config(&access_token, "SYSTEM+CACHE (no tool prefix)", &config, base_payload.clone()).await;
        results.push(("SYSTEM+CACHE", success));
    }

    // Test 5: System prompt + tool prefix (no cache_control)
    {
        let config = OAuthConfig::new()
            .with_cache_control(false);
        let success = test_config(&access_token, "SYSTEM+TOOLS (no cache_control)", &config, base_payload.clone()).await;
        results.push(("SYSTEM+TOOLS", success));
    }

    // Test 6: Only cache_control (no system prompt)
    // This tests if system prompt content matters or just the format
    {
        let config = OAuthConfig::none()
            .with_cache_control(true);  // This won't do anything without system prompt
        let success = test_config(&access_token, "CACHE_ONLY (no system prompt)", &config, base_payload.clone()).await;
        results.push(("CACHE_ONLY", success));
    }

    // Test 7: Full cloaking (all flags including optional)
    {
        let config = OAuthConfig::full_cloaking();
        let success = test_config(&access_token, "FULL_CLOAKING (all flags)", &config, base_payload.clone()).await;
        results.push(("FULL_CLOAKING", success));
    }

    // Test 8: Default + user_id injection
    {
        let config = OAuthConfig::default()
            .with_inject_user_id(true);
        let success = test_config(&access_token, "DEFAULT+USER_ID", &config, base_payload.clone()).await;
        results.push(("DEFAULT+USER_ID", success));
    }

    // Test 9: Default + obfuscation
    // Now works because "claude" and "anthropic" were removed from DEFAULT_SENSITIVE_WORDS
    {
        let config = OAuthConfig::default()
            .with_obfuscation(true);
        let success = test_config(&access_token, "DEFAULT+OBFUSCATION", &config, base_payload.clone()).await;
        results.push(("DEFAULT+OBFUSCATION", success));
    }

    // ========== TOOL USE TESTS ==========
    println!("\n\n========== TOOL USE TESTS ==========\n");

    // Payload with a tool
    let tool_payload = json!({
        "model": MODEL,
        "max_tokens": 100,
        "messages": [{"role": "user", "content": "What is 2+2? Use the calculator tool."}],
        "tools": [{
            "name": "calculator",
            "description": "Perform arithmetic calculations",
            "input_schema": {
                "type": "object",
                "properties": {
                    "expression": {"type": "string", "description": "Math expression"}
                },
                "required": ["expression"]
            }
        }],
        "tool_choice": {"type": "tool", "name": "calculator"}
    });

    // Test 10: Tools WITH prefix (should work)
    {
        let config = OAuthConfig::default(); // prefix_tool_names: true
        let success = test_config(&access_token, "TOOLS+PREFIX (default)", &config, tool_payload.clone()).await;
        results.push(("TOOLS+PREFIX", success));
    }

    // Test 11: Tools WITHOUT prefix (should fail or behave incorrectly)
    {
        let config = OAuthConfig::default()
            .with_prefix_tool_names(false);
        let success = test_config(&access_token, "TOOLS-NO_PREFIX", &config, tool_payload.clone()).await;
        results.push(("TOOLS-NO_PREFIX", success));
    }

    // Summary
    println!("\n\n========== SUMMARY ==========\n");
    for (name, success) in &results {
        let icon = if *success { "✅" } else { "❌" };
        println!("{} {}", icon, name);
    }

    println!("\n========== CONCLUSIONS ==========\n");

    // Analyze results
    let default_works = results.iter().find(|(n, _)| *n == "DEFAULT").map(|(_, s)| *s).unwrap_or(false);
    let none_works = results.iter().find(|(n, _)| *n == "NONE").map(|(_, s)| *s).unwrap_or(false);
    let system_only_works = results.iter().find(|(n, _)| *n == "SYSTEM_PROMPT_ONLY").map(|(_, s)| *s).unwrap_or(false);
    let obfuscation_works = results.iter().find(|(n, _)| *n == "DEFAULT+OBFUSCATION").map(|(_, s)| *s).unwrap_or(false);
    let tools_with_prefix = results.iter().find(|(n, _)| *n == "TOOLS+PREFIX").map(|(_, s)| *s).unwrap_or(false);
    let tools_no_prefix = results.iter().find(|(n, _)| *n == "TOOLS-NO_PREFIX").map(|(_, s)| *s).unwrap_or(false);

    if none_works {
        println!("⚡ No transformations required - OAuth works without any modifications!");
    } else if default_works {
        println!("✅ Default config works.\n");

        println!("Required flags:");
        println!("  → inject_system_prompt: REQUIRED (without it: 400 error)");

        println!("\nOptional flags (simple chat):");
        if system_only_works {
            println!("  → use_cache_control: OPTIONAL");
        }
        println!("  → prefix_tool_names: OPTIONAL for simple chat");
        println!("  → inject_user_id: OPTIONAL");
        println!("  → add_stainless_headers: OPTIONAL");
        if obfuscation_works {
            println!("  → obfuscate_sensitive_words: OPTIONAL (safe)");
        }

        println!("\nTool use:");
        if tools_with_prefix && !tools_no_prefix {
            println!("  → prefix_tool_names: REQUIRED for tool use!");
        } else if tools_with_prefix && tools_no_prefix {
            println!("  → prefix_tool_names: OPTIONAL (works without prefix too)");
        } else if !tools_with_prefix {
            println!("  ⚠️ Tool use not working even with prefix");
        }
    } else {
        println!("⚠️ Default config doesn't work - API may have changed!");
    }

    Ok(())
}
