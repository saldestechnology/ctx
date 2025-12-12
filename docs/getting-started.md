# Getting Started

## Installation

### From Source (Recommended)

```bash
git clone https://github.com/yourusername/ctx
cd ctx
cargo build --release

# Add to your PATH
cp target/release/ctx /usr/local/bin/
# or
export PATH="$PATH:$(pwd)/target/release"
```

### Using Cargo

```bash
cargo install --path .
```

## Your First Context

Navigate to any project and generate context:

```bash
cd my-project
ctx
```

This outputs all source files in XML format, ready to paste into an LLM.

### Copy to Clipboard

macOS:
```bash
ctx src/ | pbcopy
```

Linux:
```bash
ctx src/ | xclip -selection clipboard
```

### Select Specific Files

```bash
# Just Rust files
ctx "**/*.rs"

# Multiple patterns
ctx "src/**/*.ts" "lib/**/*.ts"

# Specific directories
ctx src/ lib/ tests/
```

## Your First Index

Build the code intelligence database:

```bash
ctx index
```

This creates `.ctx/codebase.sqlite` with:
- All symbols (functions, classes, interfaces, etc.)
- Call relationships between symbols
- Compressed source code
- Full-text search index

### Search Your Code

```bash
# Find symbols by name
ctx search "handleRequest"

# Semantic search
ctx search "error handling"
```

### Explore Relationships

```bash
# Who calls this function?
ctx query callers processPayment

# What does this function call?
ctx query deps validateInput

# What would break if I change this?
ctx query impact authenticate
```

## Project Configuration

Create a `.contextignore` file to exclude project-specific paths:

```gitignore
# Test fixtures
fixtures/
__snapshots__/

# Generated code
*.generated.ts
dist/

# Large data files
*.sql
*.csv
```

## Next Steps

- [Context Generation](context-generation.md) - Deep dive into output formats and filtering
- [Code Intelligence](code-intelligence.md) - Learn about indexing and querying
- [Architecture](architecture.md) - Understand how ctx works under the hood
