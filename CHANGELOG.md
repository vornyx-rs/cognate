# Changelog

All notable changes to Cognate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## [0.1.0] - 2026-04-08

### Added

**Core Framework**
- Initial release of Cognate LLM framework
- Provider trait for multi-provider abstraction
- Request/Response types with streaming support
- Middleware and layer system for composable request handling
- Error handling with comprehensive error types
- TokenBucket rate limiting

**Providers**
- OpenAI provider with full chat completion support
  - Supports gpt-4, gpt-4o, gpt-3.5-turbo and other models
  - Tool calling support
  - Streaming responses
  - Retry and rate limiting middleware
- Anthropic provider with Claude support
  - Claude 3 family models
  - Tool use support
  - Streaming responses
- FallbackProvider for automatic provider switching
- RetryConfig with exponential backoff

**Tools**
- Tool trait for type-safe tool definitions
- Tool derive macro (#[derive(Tool)])
- ToolExecutor for automatic tool dispatching and loop management
- JSON schema generation for LLM compatibility
- Tool result injection and multi-turn handling

**Prompts**
- Prompt trait for template rendering
- Prompt derive macro (#[derive(Prompt)])
- Compile-time template variable validation
- Handlebars template support

**RAG**
- VectorStore trait for pluggable vector search
- InMemoryVectorStore reference implementation
- Document and chunk management
- Search result ranking

**Axum Integration**
- Axum extractors for HTTP integration
- Middleware layers for observability
- Usage tracking layer
- Example ChatGPT-like web server

**CLI**
- Basic CLI tools for development
- Test utilities

**Testing**
- MockProvider for unit testing
- Comprehensive test suite
  - 17 unit tests
  - 7 doc tests
  - All tests passing
- Documentation examples that compile and run

**Documentation**
- README with quick start guide
- Getting Started tutorial
- Architecture documentation
- API documentation with examples
- Contributing guidelines
- Benchmark reference

### Quality Assurance

- Clean compilation with zero warnings
- Compatible with Rust 1.70 (MSRV)
- Tested on stable and MSRV
- GitHub Actions CI/CD pipeline
- Format and linting checks
- Comprehensive doc comment coverage

### Dependencies

Minimal, well-vetted dependencies:

- tokio (async runtime)
- serde/serde_json (serialization)
- reqwest (HTTP client)
- async-trait (async traits)
- thiserror (error handling)
- tracing (observability)
- axum (web framework, optional)
- proc-macro2/quote/syn (macros)
- schemars (JSON schema)

### Known Limitations

- OpenAI and Anthropic providers require API keys
- Vector stores are in-memory by default
- Embedding generation not included (use external provider)
- No built-in cost tracking (can be added via middleware)

## Future Releases

### Planned for v0.2

- Additional provider integrations (Groq, LLaMA)
- Vector store integrations (Qdrant, Pinecone, Weaviate)
- Streaming cost estimation
- Enhanced error recovery
- Provider recommendation engine (cost optimization)

### Planned for v0.3

- Web dashboard for monitoring
- Multi-turn conversation state management
- Evaluation framework
- Caching layer improvements
- Advanced observability

### Long-term Vision

- WASM support for browser LLM apps
- Distributed caching layer
- Federated learning capabilities
- Zero-knowledge proof support for private inference
- Model fine-tuning tools
- Cross-platform CLI enhancements

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute bug fixes, features, and documentation.

## License

Dual-licensed under MIT and Apache-2.0.
