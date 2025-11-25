# Contributing to context

Thanks for your interest in contributing! This document outlines how to get started.

## Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/context.git
   cd context
   ```

2. **Build the project**
   ```bash
   cargo build
   ```

3. **Run tests**
   ```bash
   cargo test
   ```

4. **Run the CLI**
   ```bash
   cargo run -- --help
   cargo run -- src/
   ```

## Making Changes

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run clippy (`cargo clippy`)
6. Format code (`cargo fmt`)
7. Commit your changes (`git commit -am 'Add my feature'`)
8. Push to your fork (`git push origin feature/my-feature`)
9. Open a Pull Request

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and address any warnings
- Add tests for new functionality
- Keep commits focused and atomic

## Areas for Contribution

- **New output formats** - Add support for JSON, YAML, or custom templates
- **Performance** - Optimize for very large codebases
- **Features** - Token counting, file size limits, depth limits
- **Documentation** - Improve README, add examples
- **Tests** - Increase test coverage

## Reporting Issues

When reporting issues, please include:
- Your OS and Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Relevant error messages

## Questions?

Open an issue with the `question` label.
