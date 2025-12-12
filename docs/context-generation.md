# Context Generation

The primary use case for ctx is generating formatted context for LLMs. This page covers all the options for selecting files and formatting output.

## File Selection

### Glob Patterns

ctx supports standard glob patterns:

```bash
# All TypeScript files
ctx "**/*.ts"

# All files in src/
ctx "src/**/*"

# Multiple patterns
ctx "src/**/*.rs" "tests/**/*.rs"

# Specific extensions
ctx "**/*.{ts,tsx,js,jsx}"
```

### Direct Paths

You can also specify files and directories directly:

```bash
ctx src/ lib/ Cargo.toml README.md
```

### Combining Patterns and Paths

```bash
ctx src/ "tests/**/*.test.ts" package.json
```

## Output Formats

### XML (Default)

Best for most LLMs. Clear structure with file paths as attributes.

```bash
ctx --format xml src/
```

```xml
<context>
<project_tree>
project/
├── src/
│   ├── main.rs
│   └── lib.rs
└── Cargo.toml
</project_tree>
<project_files>
<file name="main.rs" path="/src/main.rs">
fn main() {
    println!("Hello!");
}
</file>
<file name="lib.rs" path="/src/lib.rs">
pub fn greet() -> String {
    "Hello".to_string()
}
</file>
</project_files>
</context>
```

### Markdown

Good for chat interfaces that render markdown.

```bash
ctx --format markdown src/
# or
ctx --format md src/
```

````markdown
# Project Context

## Project Tree

```
project/
├── src/
│   ├── main.rs
│   └── lib.rs
└── Cargo.toml
```

## /src/main.rs

```rs
fn main() {
    println!("Hello!");
}
```

## /src/lib.rs

```rs
pub fn greet() -> String {
    "Hello".to_string()
}
```
````

### Plain Text

Simple format without markup.

```bash
ctx --format plain src/
```

```
=== PROJECT TREE ===

project/
├── src/
│   ├── main.rs
│   └── lib.rs
└── Cargo.toml

=== /src/main.rs ===

fn main() {
    println!("Hello!");
}

=== /src/lib.rs ===

pub fn greet() -> String {
    "Hello".to_string()
}
```

## Ignore System

ctx uses a three-tier ignore system:

### 1. .gitignore (Default)

Automatically respects your `.gitignore` file. Disable with:

```bash
ctx --no-gitignore
```

### 2. .contextignore

Create a `.contextignore` file for project-specific exclusions. Uses the same syntax as `.gitignore`:

```gitignore
# Test files
**/*.test.ts
**/*.spec.ts
__tests__/

# Fixtures and mocks
fixtures/
__mocks__/

# Generated files
*.generated.ts
*.pb.go
generated/

# Documentation
docs/
*.md

# Large files
*.sql
*.csv
data/
```

### 3. Built-in Ignores

ctx automatically excludes 170+ common non-source patterns:

- **Version control**: `.git/`, `.svn/`
- **Dependencies**: `node_modules/`, `vendor/`, `target/`
- **Build outputs**: `dist/`, `build/`, `.next/`, `out/`
- **Binary files**: `*.png`, `*.jpg`, `*.exe`, `*.dll`, `*.so`
- **Lock files**: `package-lock.json`, `yarn.lock`, `Cargo.lock`
- **Environment**: `.env`, `*.pem`, `*.key`
- **IDE**: `.vscode/`, `.idea/`

Disable with:

```bash
ctx --no-default-ignores
```

### Custom Ignores

Add patterns on the command line:

```bash
ctx -i "*.test.ts" -i "fixtures/" src/
```

## Additional Options

### Show File Sizes

Display file sizes in the project tree:

```bash
ctx --show-sizes src/
```

```
project/
├── src/
│   ├── main.rs (1.2 KB)
│   └── lib.rs (856 B)
└── Cargo.toml (423 B)
```

### Skip Project Tree

Omit the tree visualization:

```bash
ctx --no-tree src/
```

### Buffered Output

By default, ctx streams output as files are processed. To buffer everything first:

```bash
ctx --no-stream src/
```

### Statistics

Print stats to stderr after completion:

```bash
ctx --stats src/
# Generated context: 42 files, 156.3 KB in 23ms
```

## Common Workflows

### Copy to LLM

```bash
# macOS
ctx src/ | pbcopy

# Linux
ctx src/ | xclip -selection clipboard

# WSL
ctx src/ | clip.exe
```

### Save to File

```bash
ctx src/ > context.xml
ctx --format md src/ > CONTEXT.md
```

### Pipe to Another Tool

```bash
# Count tokens (hypothetical)
ctx src/ | token-counter

# Send to API
ctx src/ | curl -X POST -d @- https://api.example.com/analyze
```

### Compare Contexts

```bash
ctx src/ > before.xml
# make changes
ctx src/ > after.xml
diff before.xml after.xml
```
