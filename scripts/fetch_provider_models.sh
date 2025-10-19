#!/bin/bash
# Script to fetch model lists and pricing from all providers
# Uses 1Password to securely inject API keys

set -e

echo "🔐 Injecting API keys from 1Password..."
eval $(op inject -i .env.template)

echo "📡 Fetching model lists and pricing from providers..."

# Create output directory
mkdir -p test_results

# Function to fetch models from a provider using curl
fetch_provider_models() {
    local provider=$1
    local api_key_env=$2
    local url=$3
    local auth_header=$4
    
    echo ""
    echo "=== Fetching from $provider ==="
    
    if [[ -z "${!api_key_env}" ]]; then
        echo "❌ No API key found for $provider (env var: $api_key_env)"
        return 1
    fi
    
    echo "✅ API key found for $provider"
    
    # Prepare curl command based on provider
    case $provider in
        "OpenRouter")
            curl -s -H "Authorization: Bearer ${!api_key_env}" \
                 -H "HTTP-Referer: https://github.com/jeremychone/rust-genai" \
                 -H "X-Title: genai-test" \
                 "$url" > "test_results/${provider,,}_models.json"
            ;;
        "Groq")
            curl -s -H "Authorization: Bearer ${!api_key_env}" \
                 "$url" > "test_results/${provider,,}_models.json"
            ;;
        "Cerebras")
            curl -s -H "Authorization: Bearer ${!api_key_env}" \
                 "$url" > "test_results/${provider,,}_models.json"
            ;;
        "Z.AI")
            curl -s -H "Authorization: Bearer ${!api_key_env}" \
                 "$url" > "test_results/${provider,,}_models.json"
            ;;
    esac
    
    if [[ $? -eq 0 ]]; then
        echo "✅ Successfully fetched models from $provider"
        # Pretty print the JSON if possible
        if command -v jq &> /dev/null; then
            echo "📊 Model count: $(jq 'if type == "array" then length else if .data? then (.data | length) else 1 end end' "test_results/${provider,,}_models.json")"
            echo "💰 Sample pricing info:"
            jq -r 'if type == "array" then .[0] else if .data? then .data[0] else . end end | if .pricing? then .pricing else if .id? then "Model: \(.id)" else "Unknown structure" end end' "test_results/${provider,,}_models.json" 2>/dev/null || echo "   Could not extract pricing info"
        fi
    else
        echo "❌ Failed to fetch models from $provider"
    fi
}

# Provider configurations
echo ""
echo "📋 Starting provider model fetches..."

# OpenRouter
fetch_provider_models "OpenRouter" "OPENROUTER_API_KEY" "https://openrouter.ai/api/v1/models"

# Groq
fetch_provider_models "Groq" "GROQ_API_KEY" "https://api.groq.com/openai/v1/models"

# Cerebras
fetch_provider_models "Cerebras" "CEREBRAS_API_KEY" "https://api.cerebras.ai/v1/models"

# Z.AI (if API key is available)
if [[ -n "$ZAI_API_KEY" ]] && [[ "$ZAI_API_KEY" != "op://"* ]]; then
    fetch_provider_models "Z.AI" "ZAI_API_KEY" "https://api.z.ai/v1/models"
else
    echo ""
    echo "=== Skipping Z.AI ==="
    echo "ℹ️  No API key available for Z.AI"
fi

echo ""
echo "✨ All fetches completed!"
echo "📁 Results saved in test_results/ directory"

# Summary
echo ""
echo "📈 Summary:"
for file in test_results/*_models.json; do
    if [[ -f "$file" ]]; then
        provider=$(basename "$file" _models.json | sed 's/.*/\u&/')
        if command -v jq &> /dev/null; then
            count=$(jq 'if type == "array" then length else if .data? then (.data | length) else 1 end end' "$file" 2>/dev/null || echo "N/A")
            echo "  - $provider: $count models"
        else
            size=$(wc -c < "$file")
            echo "  - $provider: $(($size / 1024))KB response"
        fi
    fi
done