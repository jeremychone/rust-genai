# Test Specification for rust-genai Library

## Overview

This document outlines the comprehensive testing strategy for the rust-genai library, focusing on Anthropic and OpenRouter API compatibility. The testing approach includes both live API tests and mock server tests to ensure reliability and offline development capabilities.

## Testing Architecture

### 1. Live API Tests
- **Purpose**: Validate real-world API compatibility
- **Execution**: Run against actual provider APIs
- **Requirements**: Valid API keys and network access
- **Frequency**: Nightly builds and before releases

### 2. Mock Server Tests  
- **Purpose**: Enable offline testing and CI/CD reliability
- **Execution**: Run against local mock servers
- **Requirements**: No external dependencies
- **Frequency**: Every commit and PR

## Test Categories

### A. Core Chat Functionality

#### A1. Simple Chat Completion
**Input**: Basic user message
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{"role": "user", "content": "Hello, how are you?"}],
  "max_tokens": 100
}
```

**Expected Output**: 
```json
{
  "content": [{"type": "text", "text": "Hello! I'm doing well, thank you for asking."}],
  "usage": {"prompt_tokens": 12, "completion_tokens": 15, "total_tokens": 27}
}
```

**Actions**:
- Verify response contains text content
- Validate token usage counts
- Ensure response time < 30 seconds
- Check content is non-empty

#### A2. System Message Handling
**Input**: System message + user message
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant. Be concise."},
    {"role": "user", "content": "Explain quantum computing"}
  ],
  "max_tokens": 150
}
```

**Expected Output**: Concise explanation of quantum computing

**Actions**:
- Verify system message influences response style
- Check response is concise (< 100 words)
- Validate content accuracy

#### A3. Multi-turn Conversation
**Input**: Conversation history
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [
    {"role": "user", "content": "What is 2+2?"},
    {"role": "assistant", "content": "2+2 equals 4."},
    {"role": "user", "content": "What is 4+4?"}
  ],
  "max_tokens": 50
}
```

**Expected Output**: "4+4 equals 8."

**Actions**:
- Verify context preservation
- Check mathematical accuracy
- Validate conversation flow

### B. Advanced Features

#### B1. Streaming Responses
**Input**: Streaming request
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{"role": "user", "content": "Count to 10"}],
  "stream": true,
  "max_tokens": 100
}
```

**Expected Output**: Server-sent events with incremental content

**Actions**:
- Verify streaming format compliance
- Check content chunk integrity
- Validate final assembled content
- Measure streaming latency

#### B2. Tool/Function Calling
**Input**: Tool definition + user query
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{"role": "user", "content": "What's the weather in Paris?"}],
  "tools": [{
    "name": "get_weather",
    "description": "Get weather information",
    "input_schema": {
      "type": "object",
      "properties": {
        "location": {"type": "string"},
        "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}
      },
      "required": ["location"]
    }
  }],
  "max_tokens": 100
}
```

**Expected Output**: Tool call request
```json
{
  "content": [{
    "type": "tool_use",
    "id": "toolu_01...",
    "name": "get_weather",
    "input": {"location": "Paris", "unit": "celsius"}
  }]
}
```

**Actions**:
- Verify tool call structure
- Validate parameter extraction
- Check tool response handling

#### B3. JSON Mode
**Input**: JSON mode request
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{"role": "user", "content": "List 3 colors in JSON format"}],
  "response_format": {"type": "json_object"},
  "max_tokens": 100
}
```

**Expected Output**: Valid JSON array of colors

**Actions**:
- Verify JSON validity
- Check content structure
- Validate schema compliance

### C. Error Handling

#### C1. Authentication Errors
**Input**: Invalid API key
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{"role": "user", "content": "Hello"}],
  "headers": {"Authorization": "Bearer invalid-key"}
}
```

**Expected Output**: 401 Unauthorized

**Actions**:
- Verify error code 401
- Check error message clarity
- Validate error handling in client

#### C2. Rate Limiting
**Input**: Rapid successive requests
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{"role": "user", "content": "Hello"}]
}
```

**Expected Output**: 429 Too Many Requests

**Actions**:
- Verify rate limit detection
- Check retry-after header
- Validate backoff mechanism

#### C3. Invalid Requests
**Input**: Malformed request
```json
{
  "model": "invalid-model",
  "messages": [{"role": "invalid", "content": 123}]
}
```

**Expected Output**: 400 Bad Request

**Actions**:
- Verify error validation
- Check error message helpfulness
- Validate input sanitization

### D. Performance Testing

#### D1. Response Time
**Input**: Standard request
**Actions**:
- Measure response time
- Verify < 30 seconds for simple queries
- Track percentiles (p50, p95, p99)

#### D2. Throughput
**Input**: Concurrent requests
**Actions**:
- Send 10 concurrent requests
- Measure total completion time
- Verify no request failures

#### D3. Token Efficiency
**Input**: Various prompt sizes
**Actions**:
- Test with 1K, 10K, 100K token prompts
- Measure processing time per token
- Verify linear scaling

### E. Media Handling

#### E1. Image Input
**Input**: Image + text
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{
    "role": "user",
    "content": [
      {"type": "text", "text": "What's in this image?"},
      {"type": "image", "source": {
        "type": "base64",
        "media_type": "image/jpeg",
        "data": "base64-encoded-image"
      }}
    ]
  }],
  "max_tokens": 100
}
```

**Expected Output**: Image description

**Actions**:
- Verify image processing
- Check content accuracy
- Validate media type handling

#### E2. Document Input
**Input**: PDF document + text
```json
{
  "model": "claude-3-5-haiku-latest",
  "messages": [{
    "role": "user",
    "content": [
      {"type": "text", "text": "Summarize this document"},
      {"type": "document", "source": {
        "type": "base64",
        "media_type": "application/pdf",
        "data": "base64-encoded-pdf"
      }}
    ]
  }],
  "max_tokens": 200
}
```

**Expected Output**: Document summary

**Actions**:
- Verify PDF processing
- Check content extraction
- Validate summary accuracy

## Mock Server Specifications

### Anthropic Mock Server

#### Endpoints:
- `POST /v1/messages` - Chat completions
- `POST /v1/messages/beta/stream` - Streaming chat

#### Response Templates:
```json
// Success response
{
  "id": "msg_01...",
  "type": "message",
  "role": "assistant",
  "content": [{"type": "text", "text": "Mock response"}],
  "model": "claude-3-5-haiku-latest",
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 10,
    "output_tokens": 5
  }
}

// Error response
{
  "type": "error",
  "error": {
    "type": "authentication_error",
    "message": "Invalid API key"
  }
}
```

### OpenRouter Mock Server

#### Endpoints:
- `POST /api/v1/chat/completions` - Chat completions
- `POST /api/v1/chat/completions/stream` - Streaming chat

#### Response Templates:
```json
// Success response
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "anthropic/claude-3.5-sonnet",
  "choices": [{
    "index": 0,
    "message": {
      "role": "assistant",
      "content": "Mock response"
    },
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 5,
    "total_tokens": 15
  }
}
```

## Test Configuration

### Environment Variables
```bash
# Live API Tests
ANTHROPIC_API_KEY=your_key_here
OPENROUTER_API_KEY=your_key_here

# Test Configuration
GENAI_TEST_MODE=live|mock
GENAI_TEST_TIMEOUT=30
GENAI_TEST_CONCURRENT=10
```

### Test Categories
```bash
# Run all tests
cargo test

# Run only live tests
cargo test --features live-tests

# Run only mock tests  
cargo test --features mock-tests

# Run performance tests
cargo test --features perf-tests

# Run error scenario tests
cargo test --features error-tests
```

## Implementation Plan

### Phase 1: Mock Server Infrastructure
1. Create mock server framework
2. Implement Anthropic mock endpoints
3. Implement OpenRouter mock endpoints
4. Add response template system

### Phase 2: Enhanced Test Suite
1. Implement error scenario tests
2. Add performance benchmarks
3. Create contract validation tests
4. Enhance streaming tests

### Phase 3: Integration & CI
1. Configure CI/CD pipelines
2. Add test reporting
3. Implement test data management
4. Add test documentation

## Success Criteria

### Functional Requirements
- [ ] All existing tests pass with mock servers
- [ ] New error scenarios are covered
- [ ] Performance benchmarks are established
- [ ] Streaming is thoroughly tested

### Non-Functional Requirements
- [ ] Tests run in < 5 minutes
- [ ] Mock servers start in < 2 seconds
- [ ] 95% test coverage maintained
- [ ] No external dependencies for CI

### Quality Requirements
- [ ] Clear error messages for failures
- [ ] Comprehensive test documentation
- [ ] Reproducible test results
- [ ] Proper test isolation

## Maintenance

### Regular Updates
- Update mock responses when APIs change
- Review test coverage monthly
- Update performance benchmarks quarterly
- Refresh test data as needed

### Monitoring
- Track test execution times
- Monitor flaky tests
- Alert on test failures
- Generate test reports

This specification provides a comprehensive foundation for improving the rust-genai library's test suite, ensuring reliability, performance, and compatibility with Anthropic and OpenRouter APIs.