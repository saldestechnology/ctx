# ctx Documentation

Welcome to the ctx documentation. ctx is a fast CLI tool for generating AI-ready context from codebases, with built-in code intelligence for understanding symbol relationships.

## Table of Contents

* [Getting Started](getting-started.md)
* [Context Generation](context-generation.md)
* [Code Intelligence](code-intelligence.md)
* [Architecture](architecture.md)
* [Configuration](configuration.md)
* [Language Support](language-support.md)

## Why ctx?

### The Problem

When working with LLMs for coding assistance, you need to provide context about your codebase. This typically means:
- Manually copying files
- Losing track of what you've shared
- Including irrelevant files (node_modules, build artifacts)
- Missing important relationships between code

For larger codebases, you also need to understand:
- What functions call what
- What would break if you change something
- Where a particular pattern is used

### The Solution

ctx solves both problems:

1. **Context Generation** - Intelligently select and format code for LLMs
2. **Code Intelligence** - Build a searchable index with call graphs and impact analysis

## Quick Example

```bash
# Generate context for an LLM
ctx src/ | pbcopy

# Build code intelligence index
ctx index

# Find all functions that handle authentication
ctx search "auth"

# Semantic search with natural language
ctx embed                    # Generate embeddings (first time)
ctx semantic "error handling and recovery"

# See what calls the login function
ctx query callers handleLogin

# What breaks if I change this?
ctx query impact validateToken

# Code analysis
ctx complexity --warnings-only   # Find high fan-out functions
ctx duplicates                   # Find duplicate code
ctx graph --by-file              # Visualize dependencies
```

## Key Features

- **Fast** - Written in Rust, indexes thousands of files in seconds
- **Smart filtering** - Respects .gitignore, excludes binaries and build artifacts
- **Multi-language** - Rust, TypeScript, JavaScript, Python, Solidity, and more
- **Single file database** - Everything in one portable SQLite file
- **Incremental updates** - Only reindex what changed
- **Watch mode** - Auto-reindex on file changes
- **Semantic search** - Natural language queries with local or OpenAI embeddings
- **Code analysis** - Complexity scoring, duplicate detection, dependency graphs

## Getting Help

```bash
ctx --help           # General help
ctx index --help     # Index command help
ctx query --help     # Query command help
ctx complexity --help  # Complexity analysis help
ctx duplicates --help  # Duplicate detection help
ctx graph --help       # Dependency graph help
```
