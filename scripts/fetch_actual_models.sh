#!/bin/bash
# Script to fetch actual model lists from providers via their APIs

set -e

echo "🔍 Fetching actual model lists from provider APIs..."
echo ""

# Create output directory
mkdir -p test_results/provider_models

# Function to fetch models from provider API
fetch_provider_models() {
    local provider=$1
    local env_key=$2
    local url=$3
    local output_file="test_results/provider_models/${provider,,}_models.json"
    
    echo "=== Fetching from $provider ==="
    
    if [[ -z "${!env_key}" ]]; then
        echo "⚠️  No API key for $provider, skipping"
        return
    fi
    
    echo "Fetching from $url..."
    
    # Fetch with curl, saving both raw response and extracted model names
    if curl -s -H "Authorization: Bearer ${!env_key}" \
           -H "Content-Type: application/json" \
           "$url" > "$output_file" 2>/dev/null; then
        
        echo "✅ Successfully fetched"
        
        # Extract model names if possible
        if command -v jq &> /dev/null; then
            echo ""
            echo "Available models:"
            jq -r 'if type == "array" then .[] else if .data? then .data[] else .[] end | select(.id // .model // .name) // empty' "$output_file" 2>/dev/null | head -20
        else
            echo ""
            echo "First 500 chars of response:"
            head -c 500 "$output_file"
        fi
    else
        echo "❌ Failed to fetch from $provider"
    fi
    
    echo ""
    echo "---"
    echo ""
}

# Check if we have any API keys
if [[ -n "$OPENROUTER_API_KEY" ]] || [[ -n "$ANTHROPIC_API_KEY" ]] || [[ -n "$GROQ_API_KEY" ]] || [[ -n "$CEREBRAS_API_KEY" ]]; then
    echo "🔐 Using environment variables (set them manually)"
else
    echo "⚠️  No API keys found in environment variables"
    echo "   Set them manually or use: eval \$(op inject -i .env.template)"
fi

echo ""

# Fetch from providers that might have API keys
if [[ -n "$GROQ_API_KEY" ]]; then
    fetch_provider_models "Groq" "GROQ_API_KEY" "https://api.groq.com/openai/v1/models"
fi

if [[ -n "$OPENROUTER_API_KEY" ]]; then
    fetch_provider_models "OpenRouter" "OPENROUTER_API_KEY" "https://openrouter.ai/api/v1/models"
fi

if [[ -n "$ANTHROPIC_API_KEY" ]]; then
    fetch_provider_models "Anthropic" "ANTHROPIC_API_KEY" "https://api.anthropic.com/v1/messages"
fi

if [[ -n "$CEREBRAS_API_KEY" ]]; then
    fetch_provider_models "Cerebras" "CEREBRAS_API_KEY" "https://api.cerebras.ai/v1/models"
fi

echo "📊 Results saved in test_results/provider_models/"
echo ""
echo "💡 Check the JSON files to see actual model structures and names"