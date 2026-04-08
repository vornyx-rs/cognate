# Documentation Index

Welcome to Cognate's documentation. This index helps you navigate all available resources.

## Getting Started

Start here if you're new to Cognate.

- [Getting Started Tutorial](GETTING_STARTED.md) - 10-minute quick start guide
  - Project setup
  - First LLM app
  - Adding tools and streaming
  - Production deployment

- [README](README.md) - Project overview
  - Feature comparison
  - Performance metrics
  - Installation
  - Quick examples

## For Users

Building LLM applications with Cognate.

- [Getting Started](GETTING_STARTED.md) - Tutorial and examples
- [README](README.md) - Quick reference and overview
- [API Documentation](https://docs.rs/cognate-core) - Detailed API docs
- [Examples](cognate-providers/examples) - Full working examples
  - simple_chat.rs - Basic completions
  - streaming_chat.rs - Token streaming
  - tool_usage.rs - Tool calling
  - agent.rs - Multi-turn agents
  - rag_pipeline.rs - Retrieval-augmented generation
  - chatgpt_clone.rs - Web server

## For Developers

Contributing to Cognate or extending it.

- [Architecture](ARCHITECTURE.md) - System design and internals
  - Crate organization
  - Design patterns
  - Data flow
  - Extension points

- [Contributing](CONTRIBUTING.md) - How to contribute
  - Development setup
  - Code submission process
  - Adding providers
  - Adding vector stores

- [CHANGELOG](CHANGELOG.md) - Version history and roadmap
  - v0.1.0 features
  - Future plans

## Reference

Technical details and comparisons.

- [Benchmarks](BENCHMARKS.md) - Performance analysis
  - Latency metrics
  - Throughput
  - Memory usage
  - Scaling characteristics

- [Architecture](ARCHITECTURE.md) - Technical deep dive
  - Design patterns
  - Crate dependencies
  - Extension guide

## Visual Guides

Diagrams and visual documentation.

- [Logo](assets/logo.svg) - Cognate brand identity
- [Architecture Diagram](assets/architecture.svg) - System layers and components
- [Feature Comparison](assets/comparison.svg) - Cognate vs alternatives
- [Provider Flow](assets/provider-flow.svg) - Request routing diagram

## Community

- [Contributing Guide](CONTRIBUTING.md) - Code of conduct, development setup
- [GitHub Discussions](https://github.com/YOUR_ORG/cognate/discussions) - Questions and ideas
- [GitHub Issues](https://github.com/YOUR_ORG/cognate/issues) - Bug reports and features

## Quick Links

- Main Repository: https://github.com/YOUR_ORG/cognate
- crates.io Package: https://crates.io/crates/cognate-core
- Documentation: https://docs.rs/cognate-core
- License: MIT or Apache-2.0

## Document Organization

```
Cognate/
├── README.md              # Project overview and quick start
├── GETTING_STARTED.md     # Tutorial (start here)
├── ARCHITECTURE.md        # Technical deep dive
├── BENCHMARKS.md          # Performance analysis
├── CONTRIBUTING.md        # Developer guide
├── CHANGELOG.md           # Release history and roadmap
├── assets/
│   ├── logo.svg           # Brand identity
│   ├── architecture.svg   # System diagram
│   ├── comparison.svg     # Feature matrix
│   └── provider-flow.svg  # Request flow
└── cognate-*/
    ├── src/              # Source code
    └── examples/         # Working examples
```

## Common Tasks

### I want to build an LLM app
1. Start with [Getting Started](GETTING_STARTED.md)
2. Check [Examples](cognate-providers/examples)
3. Review [API docs](https://docs.rs/cognate-core)

### I want to use multiple LLM providers
1. Read [Architecture - Provider Implementations](ARCHITECTURE.md#layer-3-features)
2. See [Getting Started - Switch Providers](GETTING_STARTED.md#switch-providers)

### I want to add tools to my LLM app
1. Check [Getting Started - Type-Safe Tools](GETTING_STARTED.md#type-safe-tool-calling)
2. Run `cargo run --example tool_usage -p cognate-tools`
3. Review [ARCHITECTURE.md - cognate-tools](ARCHITECTURE.md)

### I want to build a web server
1. Run `cargo run --example chatgpt_clone -p cognate-axum`
2. Review [ARCHITECTURE.md - cognate-axum](ARCHITECTURE.md)

### I want to implement RAG
1. Run `cargo run --example rag_pipeline -p cognate-rag`
2. Review [Architecture - cognate-rag](ARCHITECTURE.md)

### I want to contribute
1. Read [Contributing](CONTRIBUTING.md)
2. Set up development environment
3. Run `cargo test --workspace`

### I want to understand the design
1. Start with [Architecture Overview](ARCHITECTURE.md)
2. Review [Design Patterns](ARCHITECTURE.md#design-patterns)
3. Check data flow diagrams

### I want to see performance metrics
1. Review [Benchmarks](BENCHMARKS.md)
2. Compare against alternatives
3. See methodology and results

## FAQ

**Q: Which version of Rust do I need?**
A: Rust 1.70 or later. See [Status](README.md#status).

**Q: How do I set up development?**
A: Follow [Contributing - Development Setup](CONTRIBUTING.md#development-setup).

**Q: How do I add a new provider?**
A: See [Contributing - Adding a New Provider](CONTRIBUTING.md#adding-a-new-provider).

**Q: What are the performance characteristics?**
A: See [Benchmarks](BENCHMARKS.md) for detailed metrics.

**Q: How do I report a bug?**
A: See [Contributing - Reporting Issues](CONTRIBUTING.md#reporting-issues).

**Q: Can I use Cognate in production?**
A: Yes, it's production-ready. See [Status](README.md#status).

## Next Steps

1. **New to Cognate?** Start with [Getting Started](GETTING_STARTED.md)
2. **Want to understand the design?** Read [Architecture](ARCHITECTURE.md)
3. **Want to contribute?** Check [Contributing](CONTRIBUTING.md)
4. **Need detailed specs?** Review [Benchmarks](BENCHMARKS.md)

---

Last updated: April 8, 2026
