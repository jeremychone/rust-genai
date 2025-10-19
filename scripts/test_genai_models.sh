#!/bin/bash
# Script to test model listing with the genai library
# Run with: ./scripts/test_genai_models.sh

set -e

echo "🚀 Testing genai model listing capabilities..."
echo ""

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo "❌ Error: Must be run from the rust-genai root directory"
    exit 1
fi

# Build the project first
echo "🔨 Building the project..."
cargo build --quiet

echo ""
echo "📋 Running model listing tests..."
echo ""

# Run the model listing test
echo "Running: cargo test --test test_model_listing test_list_models_all_providers -- --nocapture"
cargo test --test test_model_listing test_list_models_all_providers -- --nocapture

echo ""
echo "📋 Running accessibility tests..."
echo ""

# Run the accessibility test
echo "Running: cargo test --test test_model_listing test_provider_accessibility -- --nocapture"
cargo test --test test_model_listing test_provider_accessibility -- --nocapture

echo ""
echo "✨ All tests completed!"
echo ""
echo "💡 To test with real API keys:"
echo "   1. Set environment variables manually:"
echo "      export OPENROUTER_API_KEY='your-key'"
echo "      export GROQ_API_KEY='your-key'"
echo "      export CEREBRAS_API_KEY='your-key'"
echo ""
echo "   2. Or use 1Password:"
echo "      eval \$(op inject -i .env.template)"
echo "      ./scripts/test_genai_models.sh"