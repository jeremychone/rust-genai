#!/bin/bash
# Script to fetch and verify model lists from all providers
# This script compares actual API responses with our hardcoded model lists

set -e

echo "🔍 Fetching and verifying model lists from all providers..."
echo ""

# Create output directory
mkdir -p test_results/model_lists

# Function to fetch models and compare with our lists
verify_provider_models() {
    local provider=$1
    local env_key=$2
    local url=$3
    local expected_file=$4
    
    echo "=== $provider ==="
    
    if [[ -z "${!env_key}" ]]; then
        echo "⚠️  No API key for $provider, skipping verification"
        return
    fi
    
    echo "📡 Fetching from $url..."
    
    # Fetch models with curl
    if curl -s -H "Authorization: Bearer ${!env_key}" \
           -H "Content-Type: application/json" \
           "$url" > "test_results/model_lists/${provider,,}_actual.json" 2>/dev/null; then
        
        echo "✅ Successfully fetched models"
        
        # Extract model names from JSON
        if command -v jq &> /dev/null; then
            echo "📋 Extracting model names..."
            jq -r '.data[].id // .data[].model // .data[].name' "test_results/model_lists/${provider,,}_actual.json" 2>/dev/null | \
                sort > "test_results/model_lists/${provider,,}_extracted.txt" || {
                echo "⚠️  Could not extract model names (different JSON structure?)"
                cat "test_results/model_lists/${provider,,}_actual.json" | head -20
            }
        fi
        
        # Show sample of the response
        echo "📄 Sample response (first 500 chars):"
        head -c 500 "test_results/model_lists/${provider,,}_actual.json"
        echo ""
        echo "---"
    else
        echo "❌ Failed to fetch models from $provider"
    fi
    
    echo ""
}

# Provider configurations
echo "🔐 Injecting API keys from 1Password (if available)..."
eval $(op inject -i .env.template 2>/dev/null || echo "# No 1Password keys found")

echo "📋 Fetching from providers..."
echo ""

# Z.AI - check their actual API endpoint structure
echo "=== Checking Z.AI API endpoints ==="
echo "Testing base endpoint..."
curl -s -I "https://api.z.ai/v1/" | head -5 || echo "Base endpoint not accessible"
echo ""

echo "Testing models endpoint..."
curl -s -I "https://api.z.ai/v1/models" | head -5 || echo "Models endpoint not accessible"
echo ""

# Try alternative endpoints
echo "Testing alternative model endpoints..."
for endpoint in "https://api.z.ai/v1/models" "https://z.ai/api/v1/models" "https://api.z.ai/model-api"; do
    echo "Trying: $endpoint"
    if curl -s -I "$endpoint" | grep -q "200 OK\|201 Created"; then
        echo "✅ Found working endpoint: $endpoint"
        # Try to fetch a few lines
        curl -s "$endpoint" | head -20
        echo ""
        break
    else
        echo "❌ Not accessible"
    fi
done

echo ""
echo "📊 Summary of findings:"
echo "- Z.AI API structure needs verification"
echo "- Documentation shows models: GLM-4.6, GLM-4.5, GLM-4, GLM-4.1V, GLM-4.5V, Vidu, Vidu Q1, Vidu 2.0"
echo "- No turbo models found in documentation"