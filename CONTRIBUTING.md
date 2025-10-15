# Contributing to genai

Thank you for your interest in contributing to genai! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites

- Rust (latest stable version)
- Git

### Setup Steps

1. Fork the repository
2. Clone your fork locally
3. Create a new branch for your feature or bugfix
4. Make your changes
5. Run tests and ensure code quality
6. Submit a pull request

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test file
cargo test --test live_api_tests
```

### Live API Testing

genai includes comprehensive live API tests that validate functionality against real AI providers. These tests are located in `/tests/live_api_tests.rs`.

#### **IMPORTANT: Live API Tests Require Real Credentials**

The live API tests make actual API calls to providers like OpenRouter and Anthropic. To run these tests:

1. **Set API Keys as Environment Variables:**

```bash
export OPENROUTER_API_KEY="your-openrouter-api-key"
export ANTHROPIC_API_KEY="your-anthropic-api-key"
# Optional: Cerebras
export CEREBRAS_API_KEY="your-cerebras-api-key"
```

2. **Run the Live API Tests:**

```bash
cargo test --test live_api_tests -- --nocapture
```

#### **Available Live API Tests**

The test suite includes comprehensive validation of:

- ✅ **Basic Chat Functionality** - Tests basic chat completion
- ✅ **Streaming Support** - Validates real-time streaming responses
- ✅ **Tool/Function Calling** - Tests function calling capabilities
- ✅ **JSON Mode** - Validates structured JSON output
- ✅ **Image Processing** - Tests image analysis functionality
- ✅ **Multiple Providers** - Cross-provider compatibility testing
- ✅ **Error Handling** - Validates proper error scenarios
- ✅ **Model Resolution** - Tests model name resolution

#### **Test Structure**

- **Enabled Tests**: Core functionality tests are enabled by default
- **Ignored Tests**: Some tests are marked with `#[ignore]` to avoid excessive API calls during development
- **Environment Checks**: Tests automatically skip if required API keys are not set

#### **Adding New Live API Tests**

When adding new live API tests:

1. Follow the existing patterns in `/tests/live_api_tests.rs`
2. Include environment variable checks for required API keys
3. Use the `TestResult` type for consistent error handling
4. Add appropriate assertions and logging
5. Consider marking expensive tests with `#[ignore]`

Example test structure:

```rust
#[tokio::test]
async fn test_new_feature() -> TestResult<()> {
    if !has_env_key("PROVIDER_API_KEY") {
        println!("Skipping PROVIDER_API_KEY not set");
        return Ok(());
    }

    let client = Client::default();
    let chat_req = ChatRequest::new(vec![
        ChatMessage::user("Test message"),
    ]);

    let result = client.exec_chat("model-name", chat_req, None).await?;
    let content = result.first_text().ok_or("Should have content")?;
    
    assert!(!content.is_empty(), "Content should not be empty");
    println!("✅ Test passed: {}", content);
    
    Ok(())
}
```

## Code Quality

### Formatting

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

### Linting

```bash
# Run clippy with strict warnings
cargo clippy --all-targets --all-features -- -W clippy::all

# Run clippy with default settings
cargo clippy --all-targets --all-features
```

### Code Style Guidelines

- Follow Rust idioms and conventions
- Use meaningful variable and function names
- Add documentation for public APIs
- Keep functions focused and small
- Handle errors appropriately

## Provider-Specific Testing

### Model Names

When testing with specific providers, ensure model names are current and available:

- **OpenRouter**: Use namespaced models (e.g., `openrouter::anthropic/claude-3.5-sonnet`)
- **Anthropic**: Use current model names (e.g., `claude-3-5-sonnet-20241022`)
- **Other Providers**: Check provider documentation for latest model names

### API Key Management

- Never commit API keys to the repository
- Use environment variables for API keys
- Document required environment variables in test files
- Consider using `.env` files for local development (add to `.gitignore`)

## Submitting Changes

### Pull Request Process

1. Ensure all tests pass
2. Run code formatting and linting
3. Update documentation if needed
4. Write clear commit messages
5. Submit pull request with descriptive title and description

### Commit Message Format

```
feat: add new feature description
fix: resolve issue description
docs: update documentation
test: add or improve tests
refactor: code refactoring
```

## Getting Help

- Check existing issues and pull requests
- Review the codebase and examples
- Ask questions in pull requests
- Refer to provider documentation for API-specific details

## Provider Documentation

When working with specific AI providers, refer to their official documentation:

- [OpenRouter API](https://openrouter.ai/docs)
- [Anthropic API](https://docs.anthropic.com)
- [OpenAI API](https://platform.openai.com/docs)
- [Google Gemini API](https://ai.google.dev/docs)
- [Other Providers](https://github.com/jeremychone/rust-genai#provider-mapping)

## Security Considerations

- Never expose API keys in code or commits
- Validate and sanitize user inputs when appropriate
- Follow security best practices for API integrations
- Report security vulnerabilities privately

Thank you for contributing to genai! 🚀