#!/bin/bash
# Quick test script to demonstrate the genai library's model resolution capabilities
# This script shows how genai maps model names to providers without requiring API keys

echo "🧪 Testing genai model-to-provider resolution (no API keys required)..."
echo ""

# Build the project
echo "🔨 Building..."
cargo build --quiet

echo ""
echo "📋 Testing model resolution for various providers..."
echo ""

# Test different model patterns
cat << 'EOF' | cargo run --bin genai_resolve_test 2>/dev/null || echo "Creating test binary..."

use genai::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::default();
    
    // Test model patterns and their resolution
    let test_models = vec![
        // OpenAI patterns
        ("gpt-4o-mini", "OpenAI"),
        ("gpt-4-turbo", "OpenAI"),
        ("o1-preview", "OpenAI"),
        
        // Anthropic patterns
        ("claude-3-5-sonnet-20241022", "Anthropic"),
        ("claude-3-haiku-20240307", "Anthropic"),
        
        // Gemini patterns
        ("gemini-2.0-flash", "Gemini"),
        ("gemini-pro", "Gemini"),
        
        // Groq patterns (should resolve to Groq)
        ("llama-3.1-8b-instant", "Groq"),
        ("llama-3.1-70b-versatile", "Groq"),
        
        // Cohere patterns
        ("command-r-plus", "Cohere"),
        ("command-light", "Cohere"),
        
        // DeepSeek patterns
        ("deepseek-chat", "DeepSeek"),
        ("deepseek-coder", "DeepSeek"),
        
        // Namespaced models
        ("openrouter::anthropic/claude-3.5-sonnet", "OpenRouter"),
        ("cerebras::llama3.1-8b", "Cerebras"),
        ("openai::gpt-4o", "OpenAI"),
        
        // Default (should go to Ollama)
        ("codellama:7b", "Ollama"),
        ("mistral", "Ollama"),
    ];
    
    println!("Model Resolution Test Results:");
    println!("=============================");
    println!();
    
    let mut success_count = 0;
    let mut total_count = 0;
    
    for (model, expected_provider) in test_models {
        total_count += 1;
        
        match client.resolve_service_target(model).await {
            Ok(target) => {
                let actual_provider = format!("{:?}", target.model.adapter_kind);
                if actual_provider.contains(expected_provider) {
                    println!("✅ {:<35} -> {} (expected: {})", model, actual_provider, expected_provider);
                    success_count += 1;
                } else {
                    println!("⚠️  {:<35} -> {} (expected: {})", model, actual_provider, expected_provider);
                }
            }
            Err(e) => {
                println!("❌ {:<35} -> ERROR: {}", model, e);
            }
        }
    }
    
    println!();
    println!("Summary: {}/{} models resolved successfully", success_count, total_count);
    
    Ok(())
}
EOF

# Create a proper test binary
cat > src/bin/genai_resolve_test.rs << 'EOF'
use genai::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::default();
    
    // Test model patterns and their resolution
    let test_models = vec![
        // OpenAI patterns
        ("gpt-4o-mini", "OpenAI"),
        ("gpt-4-turbo", "OpenAI"),
        ("o1-preview", "OpenAI"),
        
        // Anthropic patterns
        ("claude-3-5-sonnet-20241022", "Anthropic"),
        ("claude-3-haiku-20240307", "Anthropic"),
        
        // Gemini patterns
        ("gemini-2.0-flash", "Gemini"),
        ("gemini-pro", "Gemini"),
        
        // Groq patterns (should resolve to Groq)
        ("llama-3.1-8b-instant", "Groq"),
        ("llama-3.1-70b-versatile", "Groq"),
        
        // Cohere patterns
        ("command-r-plus", "Cohere"),
        ("command-light", "Cohere"),
        
        // DeepSeek patterns
        ("deepseek-chat", "DeepSeek"),
        ("deepseek-coder", "DeepSeek"),
        
        // Namespaced models
        ("openrouter::anthropic/claude-3.5-sonnet", "OpenRouter"),
        ("cerebras::llama3.1-8b", "Cerebras"),
        ("openai::gpt-4o", "OpenAI"),
        
        // Default (should go to Ollama)
        ("codellama:7b", "Ollama"),
        ("mistral", "Ollama"),
    ];
    
    println!("Model Resolution Test Results:");
    println!("=============================");
    println!();
    
    let mut success_count = 0;
    let mut total_count = 0;
    
    for (model, expected_provider) in test_models {
        total_count += 1;
        
        match client.resolve_service_target(model).await {
            Ok(target) => {
                let actual_provider = format!("{:?}", target.model.adapter_kind);
                if actual_provider.contains(expected_provider) {
                    println!("✅ {:<35} -> {} (expected: {})", model, actual_provider, expected_provider);
                    success_count += 1;
                } else {
                    println!("⚠️  {:<35} -> {} (expected: {})", model, actual_provider, expected_provider);
                }
            }
            Err(e) => {
                println!("❌ {:<35} -> ERROR: {}", model, e);
            }
        }
    }
    
    println!();
    println!("Summary: {}/{} models resolved successfully", success_count, total_count);
    
    Ok(())
}
EOF

echo "Running model resolution test..."
echo ""
cargo run --bin genai_resolve_test

echo ""
echo "✨ Model resolution test completed!"
echo ""
echo "💡 To test with actual API calls:"
echo "   eval \$(op inject -i .env.template)"
echo "   cargo test --test test_model_listing -- --nocapture"