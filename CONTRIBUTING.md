# Contributing to Cognate

Thank you for your interest in contributing to Cognate. This document provides guidelines and instructions for getting involved.

## Code of Conduct

Cognate maintains a welcoming, inclusive community. We expect contributors to:

- Be respectful and constructive in discussions
- Welcome feedback and diverse perspectives
- Focus on the work, not the person
- Report concerns to project maintainers

## Ways to Contribute

### Reporting Issues

Found a bug? Have a feature idea? Open an issue on GitHub:

- Search existing issues first (to avoid duplicates)
- Use a clear, descriptive title
- Include steps to reproduce (for bugs)
- Include expected vs actual behavior
- Attach logs, code snippets if relevant

### Documentation

Documentation improvements are always welcome:

- Fix typos or unclear explanations
- Add examples or tutorials
- Improve API documentation
- Expand architecture docs
- Create getting-started guides

### Code

Code contributions are welcome for:

- Bug fixes with test cases
- Performance improvements with benchmarks
- New features with tests and documentation
- Provider implementations (OpenAI, Anthropic, etc.)
- Vector store integrations

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- OpenAI API key (for testing OpenAI provider)

### Getting Started

Clone the repository:

```bash
git clone https://github.com/YOUR_ORG/cognate
cd cognate
```

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

### Running Tests

Run all tests:

```bash
cargo test --workspace
```

Run specific crate tests:

```bash
cargo test -p cognate-core
cargo test -p cognate-providers
cargo test -p cognate-tools
```

Run tests with logging:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

Run integration tests (requires API keys):

```bash
OPENAI_API_KEY=sk-... cargo test --features integration
```

### Code Quality

Format code:

```bash
cargo fmt --all
```

Check formatting:

```bash
cargo fmt --all -- --check
```

Run linter:

```bash
cargo clippy --workspace --all-targets
```

Address linter warnings:

```bash
cargo clippy --workspace --all-targets --fix
```

Check documentation:

```bash
cargo doc --workspace --no-deps --open
```

## Submission Process

### Before You Start

1. Check if a GitHub issue exists for your change
2. For major features, open an issue to discuss first
3. Fork the repository
4. Create a feature branch: `git checkout -b feature/your-feature`

### While Developing

- Write tests for new functionality
- Update documentation
- Run tests locally: `cargo test --workspace`
- Run formatter: `cargo fmt --all`
- Run linter: `cargo clippy --workspace`

### Commit Messages

Write clear, descriptive commit messages:

```
feat: add retry middleware to core

- Implement ExponentialBackoff retry strategy
- Add RetryConfig struct
- Add tests for retry logic
- Update documentation

Closes #123
```

Format: `type: short description` where type is one of:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `perf`: Performance improvement
- `test`: Test additions
- `refactor`: Code refactoring
- `chore`: Build/tooling changes

### Pull Request

1. Push to your fork
2. Open a pull request to `main` branch
3. Provide a clear description of changes
4. Link related issues

Pull request template:

```markdown
## Description
Brief description of changes

## Related Issues
Closes #123

## Changes
- Change 1
- Change 2

## Testing
How was this tested?

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Code formatted (cargo fmt)
- [ ] No clippy warnings (cargo clippy)
- [ ] All tests pass (cargo test)
```

## Adding a New Provider

### 1. Implement the Provider Trait

In `cognate-providers/src/your_provider.rs`:

```rust
use cognate_core::{Provider, Request, Response, Chunk};
use async_trait::async_trait;
use futures::stream::BoxStream;

pub struct YourProvider {
    api_key: String,
    // config fields
}

impl YourProvider {
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(Error::Configuration("API key required".into()));
        }
        Ok(Self { api_key })
    }
}

#[async_trait]
impl Provider for YourProvider {
    async fn complete(&self, req: Request) -> Result<Response> {
        // Implement completion logic
        todo!()
    }
    
    async fn stream(&self, req: Request) -> Result<BoxStream<Chunk>> {
        // Implement streaming logic
        todo!()
    }
}
```

### 2. Add Tests

In `cognate-providers/tests/your_provider.rs`:

```rust
#[tokio::test]
async fn test_basic_completion() {
    let provider = YourProvider::new(test_key()).unwrap();
    let request = Request::new()
        .with_model("your-model")
        .with_messages(vec![Message::user("Hello")]);
    
    let response = provider.complete(request).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_streaming() {
    let provider = YourProvider::new(test_key()).unwrap();
    // Test streaming implementation
}
```

### 3. Add to Documentation

In `cognate-providers/src/lib.rs`:

```rust
//! # Providers
//!
//! | Provider | Struct | Chat | Stream | Tools |
//! |----------|--------|------|--------|-------|
//! | YourLLM  | [`YourProvider`] | ✓ | ✓ | ✓ |
```

### 4. Export in lib.rs

```rust
pub mod your_provider;
pub use your_provider::YourProvider;
```

## Adding a New VectorStore

### 1. Implement the Trait

In `cognate-rag/src/your_store.rs`:

```rust
use cognate_rag::{VectorStore, Document};
use async_trait::async_trait;

pub struct YourVectorStore {
    // storage fields
}

#[async_trait]
impl VectorStore for YourVectorStore {
    async fn add(&self, id: String, content: String) -> Result<()> {
        todo!()
    }
    
    async fn search(&self, query: String, limit: usize) -> Result<Vec<Document>> {
        todo!()
    }
    
    async fn delete(&self, id: String) -> Result<()> {
        todo!()
    }
}
```

### 2. Add Tests and Documentation

Similar to provider setup above.

## Guidelines

### Code Style

- Follow Rust idioms and conventions
- Use meaningful variable names
- Add doc comments to public APIs
- Keep functions small and focused
- Prefer composition over inheritance

### Documentation

- Document public APIs with rustdoc comments
- Include examples in doc comments
- Keep README and guides updated
- Link related concepts

### Testing

- Write tests for all public APIs
- Test both happy path and error cases
- Use descriptive test names
- Include integration tests for features
- Aim for >80% code coverage

### Performance

- Include benchmarks for performance-sensitive code
- Document performance characteristics
- Use `#[inline]` where appropriate
- Avoid unnecessary allocations

### Security

- Validate all external input
- Don't log sensitive information
- Review dependencies for security issues
- Use secure defaults

## Review Process

Pull requests are reviewed by maintainers:

- Code quality and style
- Test coverage
- Documentation
- Performance impact
- Security considerations

Be patient and constructive during review. Questions and suggestions are designed to improve the project.

## Release Process

Releases follow semantic versioning:

- v0.x.y: Pre-1.0 development releases
- v1.x.y: Stable releases
- Major.Minor.Patch

Releasing (maintainers only):

```bash
# Update version in Cargo.toml
cargo update
cargo test --all
git tag v0.x.y
git push origin v0.x.y
cargo publish
```

## License

Cognate is dual-licensed under MIT and Apache-2.0. By contributing, you agree that your contributions are licensed under these terms.

## Questions?

- Read the [ARCHITECTURE.md](ARCHITECTURE.md)
- Check [GETTING_STARTED.md](GETTING_STARTED.md)
- Open a discussion on GitHub
- Ask in issues

## Thank You

Thank you for contributing to Cognate. Your work makes this project better for everyone.
