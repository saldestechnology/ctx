# Architecture

This document explains how ctx works under the hood, the design decisions made, and why they result in a fast, reliable tool.

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         ctx CLI                              │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────┐    ┌───────────────────────────────┐  │
│  │ Context Generation│    │      Code Intelligence        │  │
│  │                  │    │                               │  │
│  │  • File walker   │    │  ┌─────────────────────────┐  │  │
│  │  • Ignore system │    │  │     Tree-sitter         │  │  │
│  │  • Formatters    │    │  │  (Multi-language parse) │  │  │
│  │  • Output stream │    │  └───────────┬─────────────┘  │  │
│  │                  │    │              │                │  │
│  └──────────────────┘    │  ┌───────────▼─────────────┐  │  │
│                          │  │       SQLite            │  │  │
│                          │  │  • Symbols & Edges      │  │  │
│                          │  │  • FTS5 Search          │  │  │
│                          │  │  • Compressed Source    │  │  │
│                          │  └───────────┬─────────────┘  │  │
│                          │              │                │  │
│                          │  ┌───────────▼─────────────┐  │  │
│                          │  │      DuckDB             │  │  │
│                          │  │  (In-memory analytics)  │  │  │
│                          │  │  • Recursive queries    │  │  │
│                          │  │  • Graph traversal      │  │  │
│                          │  └─────────────────────────┘  │  │
│                          └───────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Storage: Single SQLite File

All data lives in `.ctx/codebase.sqlite`:

```sql
-- Files table
CREATE TABLE files (
    path TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    size_bytes INTEGER,
    language TEXT,
    last_indexed INTEGER,
    source_compressed BLOB  -- gzip compressed
);

-- Symbols table
CREATE TABLE symbols (
    id TEXT PRIMARY KEY,           -- "file_path::name::line"
    name TEXT NOT NULL,
    kind TEXT NOT NULL,            -- function, struct, enum, etc.
    file_path TEXT NOT NULL,
    line_start INTEGER,
    line_end INTEGER,
    visibility TEXT DEFAULT 'private',
    signature TEXT,
    brief TEXT,                    -- First line of docstring
    parent_id TEXT,
    FOREIGN KEY (file_path) REFERENCES files(path)
);

-- Edges table (call graph)
CREATE TABLE edges (
    source_id TEXT NOT NULL,       -- Caller symbol ID
    target_name TEXT NOT NULL,     -- Called function name
    target_id TEXT,                -- Resolved symbol ID (if found)
    kind TEXT NOT NULL,            -- 'calls', 'imports', etc.
    line INTEGER,
    context TEXT,                  -- Code snippet
    FOREIGN KEY (source_id) REFERENCES symbols(id)
);

-- FTS5 virtual table for search
CREATE VIRTUAL TABLE symbols_fts USING fts5(
    name, 
    brief, 
    signature,
    content='symbols',
    content_rowid='rowid'
);
```

### Why SQLite?

1. **Single file** - Easy to manage, backup, and delete
2. **ACID transactions** - Safe concurrent access
3. **FTS5** - Built-in full-text search
4. **Widespread support** - Can inspect with any SQLite tool
5. **Fast writes** - Optimized for our insert-heavy indexing workload

## Analytics: DuckDB

For complex analytical queries (recursive CTEs, graph traversal), we use DuckDB as an in-memory query engine:

```rust
// Open in-memory DuckDB
let conn = Connection::open_in_memory()?;

// Attach SQLite database read-only
conn.execute(
    "ATTACH 'codebase.sqlite' AS code (TYPE sqlite, READ_ONLY)",
    [],
)?;

// Now we can run analytical queries
conn.execute("
    WITH RECURSIVE graph AS (
        SELECT ...
        UNION ALL
        SELECT ...
    )
    SELECT * FROM graph
", [])?;
```

### Why DuckDB?

1. **Columnar storage** - Efficient for analytical queries
2. **Recursive CTEs** - Native support for graph traversal
3. **SQLite attachment** - Query SQLite data directly, no ETL needed
4. **In-memory** - No additional files to manage
5. **OLAP optimized** - Fast aggregations and joins

### No Separate File

DuckDB runs entirely in-memory and reads from SQLite. This means:
- No data duplication
- No sync issues
- Single source of truth
- Simpler deployment

## Parsing: Tree-sitter

We use tree-sitter for parsing source code:

```rust
// Create parser
let mut parser = Parser::new();
parser.set_language(tree_sitter_rust::language())?;

// Parse source
let tree = parser.parse(source, None)?;

// Query for symbols
let query = Query::new(
    tree_sitter_rust::language(),
    "(function_item name: (identifier) @name)"
)?;

let mut cursor = QueryCursor::new();
for match_ in cursor.matches(&query, tree.root_node(), source.as_bytes()) {
    // Extract symbol information
}
```

### Why Tree-sitter?

1. **Fast** - Incremental parsing, handles large files
2. **Accurate** - Real parser, not regex-based
3. **Multi-language** - Same API for all languages
4. **Error tolerant** - Parses incomplete/invalid code
5. **Battle-tested** - Used by GitHub, Neovim, Zed, etc.

### Supported Languages

| Language | Crate | Symbols Extracted |
|----------|-------|-------------------|
| Rust | `tree-sitter-rust` | fn, struct, enum, trait, impl |
| TypeScript | `tree-sitter-typescript` | function, class, interface, type, enum |
| JavaScript | `tree-sitter-javascript` | function, class, arrow functions |
| Solidity | `tree-sitter-solidity` | contract, function, event, struct |

## File Walking

The `ignore` crate handles file discovery:

```rust
let mut builder = WalkBuilder::new(root);
builder.git_ignore(true);           // Respect .gitignore
builder.add_custom_ignore_filename(".contextignore");

for entry in builder.build() {
    // Process file
}
```

### Three-Tier Ignore System

1. **.gitignore** - Standard git ignore patterns
2. **.contextignore** - Project-specific exclusions
3. **Built-in patterns** - 170+ common non-source patterns

## Incremental Indexing

We track file changes using content hashes:

```rust
fn needs_update(&self, path: &str, content: &str) -> bool {
    let new_hash = sha256(content);
    let old_hash = self.db.get_hash(path);
    new_hash != old_hash
}
```

This means:
- Only changed files are re-parsed
- Unchanged files are skipped instantly
- Hash comparison is faster than timestamp checks

## Source Compression

We store compressed source for `ctx source` queries:

```rust
fn compress_source(content: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content.as_bytes())?;
    encoder.finish()
}
```

Typical compression: 60-70% size reduction.

## Symbol IDs

Symbols are identified by a composite key:

```
file_path::name::line
```

For example:
```
src/auth/handler.ts::handleAuth::45
```

This ensures uniqueness even with:
- Same function name in different files
- Overloaded functions
- Nested functions

## Call Graph Extraction

We extract calls using tree-sitter queries:

```scheme
; Rust function calls
(call_expression
  function: (identifier) @callee)

; Method calls  
(call_expression
  function: (field_expression
    field: (field_identifier) @callee))
```

Edges are stored with context:
```sql
INSERT INTO edges (source_id, target_name, kind, line, context)
VALUES (
    'src/main.rs::process::10',
    'validate',
    'calls',
    15,
    'let result = validate(input);'
);
```

## Performance Characteristics

### Indexing
- ~2000 files in <10 seconds
- Incremental updates: <1 second for changed files
- Memory: ~100MB for large codebases

### Queries
- Symbol search: <10ms
- Call graph (depth 5): <50ms
- Impact analysis: <100ms

### Storage
- Symbols: ~500 bytes each (uncompressed)
- Compressed source: ~30% of original size
- Typical project: 10-50MB database

## Design Principles

1. **Single file** - One `.ctx/codebase.sqlite`, no complexity
2. **Incremental** - Only do work when needed
3. **Portable** - Standard formats, no daemon processes
4. **Fast** - Rust + SQLite + DuckDB = speed
5. **Accurate** - Real parsers, not regex hacks
