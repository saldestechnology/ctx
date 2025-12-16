# ctx Improvements

This document outlines discovered issues, limitations, and recommended improvements for the `ctx` CLI tool.

## Table of Contents

- [Critical Issues](#critical-issues)
- [High Priority Issues](#high-priority-issues)
- [Medium Priority Issues](#medium-priority-issues)
- [Lower Priority Improvements](#lower-priority-improvements)
- [Performance Improvements](#performance-improvements)
- [Documentation Improvements](#documentation-improvements)

---

## Critical Issues

### 1. `new` Function Over-Counting Bug

**Status:** Resolved (call graph/complexity now join by `target_id`; edge resolution tightened to avoid receiver false positives).

**Problem:** When running `ctx complexity` or `ctx query stats`, common function names like `new`, `default`, `from` show inflated "called by" counts.

**Evidence:**
```
FUNCTION                        CALLS OUT  CALLED BY
new                                    12        173
new                                    11        173
```

**Root Cause:**
- All `new` functions across different types share the same name
- Edge resolution uses `target_name` string matching instead of fully qualified IDs
- All calls to any `new()` are counted together

**Recommendation:** Use fully-qualified symbol IDs (`file::parent::name@line`) for edge targets instead of just name matching. Update the analytics queries to join on `target_id` when available.

### 2. Call Graph and Impact Analysis Join on Name Instead of ID

**Status:** Resolved (call graph/impact/has_path now resolve symbols and traverse only `target_id` edges).

**Problem:** The `call_graph()` and `impact_analysis()` functions in `src/analytics/mod.rs` join edges to symbols using `target_name` instead of `target_id`, causing incorrect results when multiple symbols share the same name across different files/modules.

**Location:** `src/analytics/mod.rs:147-233`

**Evidence:**
```sql
-- Current (incorrect): joins on name, merging all 'new' functions
LEFT JOIN code.symbols t ON e.target_name = t.name

-- Should be: join on ID first, fallback to name
LEFT JOIN code.symbols t ON (e.target_id IS NOT NULL AND e.target_id = t.id) 
                         OR (e.target_id IS NULL AND e.target_name = t.name)
```

**Impact:**
- Call graphs show edges to wrong symbols when names collide
- Impact analysis returns callers of unrelated functions with same name
- Results are mixed across files/modules incorrectly

**Recommendation:** Update the recursive CTEs to:
1. Prefer `target_id` when available (resolved references)
2. Fall back to `target_name` only for external/unresolved symbols
3. Accept qualified names or symbol IDs as start parameters

---

## High Priority Issues

### 3. JSON Output Format Not Implemented

**Status:** Resolved (JsonFormatter implemented with proper JSON output structure).

**Problem:** The `--format json` option silently falls back to plain text output.

**Evidence:**
```bash
ctx "src/cli.rs" -f json  # Outputs plain text, not JSON
```

**Location:** `src/formatter.rs:176`

**Solution:** Implemented `JsonFormatter` with the following structure:
- Non-streaming (`--no-stream`): Single valid JSON object with `tree` and `files` array
- Streaming (default): NDJSON format (one JSON object per line) for progressive output

```json
{
  "tree": "src/\n  main.rs",
  "files": [
    { "name": "main.rs", "path": "/src/main.rs", "content": "..." }
  ]
}
```

### 4. No XML Escaping

**Status:** Resolved (XmlFormatter now escapes all special XML characters in content and attributes).

**Problem:** The XML formatter doesn't escape special characters (`<`, `>`, `&`, `"`, `'`) in file content. This produces invalid XML if source files contain these characters (very common in code).

**Solution:** Added two escape functions in `XmlFormatter`:
- `escape_xml_text()` - Escapes `&`, `<`, `>` in element content (tree, file content)
- `escape_xml_attr()` - Escapes `&`, `<`, `>`, `"`, `'` in attribute values (filename, path)

Applied to:
- `format_tree()` - Tree content is now escaped
- `format_file()` - Filename, path, and content are all escaped

### 5. Fragile OpenAI HTTP Client

**Problem:** The OpenAI integration (`src/embeddings/openai.rs`) uses raw HTTP over `native-tls` with fragile response parsing:
```rust
let json_start = response.find('{')  // May fail on edge cases
```

**Issues:**
- No redirect handling
- No connection reuse
- No automatic retries
- Chunked transfer encoding handling is brittle

**Recommendation:** Replace hand-rolled HTTP with `reqwest` or `ureq` for:
- Automatic retries
- Proper error handling
- Connection pooling
- Redirect handling
- Streaming support

### 6. `ctx source` Command Ignores File Patterns

**Status:** Resolved (all symbol lookup commands now support `--file` and `--kind` filters with disambiguation).

**Problem:** The `ctx source` command:
1. Takes only a symbol name - no pattern/path filtering
2. Calls `find_symbols(symbol, 1)` - returns just the first match
3. Ignores the `[PATTERNS]...` CLI arguments - even though they're inherited from the parent command

**Solution:** Implemented file and kind filtering with automatic disambiguation for all affected commands:

**Commands Updated:**
- `ctx source <symbol>` - Added `--file` and `--kind` filters
- `ctx explain <symbol>` - Added `--file` and `--kind` filters
- `ctx query find <pattern>` - Added `--file` filter (already had `--kind`)
- `ctx query callers <function>` - Added `--file` filter (kind not applicable - always searches functions)
- `ctx query deps <symbol>` - Added `--file` and `--kind` filters

**Usage Examples:**
```bash
# Filter by file pattern (glob syntax)
ctx source new --file "src/parser/*.rs"
ctx source new -f "parser/rust.rs"

# Filter by symbol kind
ctx source new --kind method
ctx explain parse --kind function

# Combine filters
ctx query callers new --file "src/embeddings/*"
ctx query deps run --file "src/main.rs" --kind function
```

**Disambiguation:**
When multiple symbols match and no filters are provided, the command now shows helpful disambiguation:
```bash
$ ctx source new
Found 18 symbols named 'new'. Use --file or --kind to disambiguate:

  new (method) - src/embeddings/local.rs:20
  new (method) - src/embeddings/mod.rs:76
  new (method) - src/parser/rust.rs:35
  ...

Example: ctx source new --file "src/embeddings/local.rs"
```

**Implementation Details:**
- Added `find_symbols_filtered()` to `db/schema.rs` with SQL-level file pattern and kind filtering
- File patterns support glob syntax (converted to SQL LIKE patterns: `*` -> `%`)
- All commands show helpful error messages when no symbols match with filters applied

### 7. `ctx index` Ignores All CLI Pattern/Ignore Flags

**Status:** Resolved (CLI flags now wired through to indexer and watch mode with full parity).

**Problem:** The `ctx index` command always uses `WalkerConfig::default()`, completely ignoring CLI flags like `--no-gitignore`, `--no-default-ignores`, `-i/--ignore`, and any include patterns.

**Solution:** Implemented full CLI flag support for `ctx index`:
- Added `--no-gitignore`, `--no-default-ignores`, `-i/--ignore`, `-p/--pattern` flags
- `Indexer::with_config()` accepts `WalkerConfig` from CLI
- Watch mode uses `FileFilter` for consistent filtering with initial index
- `FileFilter` replicates all `WalkBuilder` ignore sources:
  - Hidden files (dotfiles)
  - `.gitignore`, `.ignore` at all levels
  - `.contextignore` at all levels  
  - `.git/info/exclude`
  - `core.excludesFile` from git config (system/global/local)
  - Default ignores and custom CLI ignores
- Absolute glob patterns normalized against root
- Both `Any` and `AnyContinuous` debouncer events handled in watch mode

---

## Medium Priority Issues

### 8. Go Language Listed but Not Implemented

**Problem:** Go is listed in the `Language` enum and has `tree-sitter-go` as a dependency, but the parser returns empty results:
```rust
Language::Go => ParseResult::default(), // Not implemented
```

**Recommendation:** Implement extraction for:
- Functions, methods
- Structs, interfaces
- Import statements

### 9. Duplicate Detection Shows Repeated Pairs

**Status:** Resolved (duplicate finder now tracks seen pairs before returning results).

**Problem:** The duplicates command shows the same pair multiple times:
```
1. Similarity: 88.2% (5 lines)
   stream_start (src/formatter.rs:106)
   stream_start (src/formatter.rs:142)

2. Similarity: 88.2% (5 lines)  <-- Same pair again
   stream_start (src/formatter.rs:106)
   stream_start (src/formatter.rs:142)
```

**Recommendation:** Fix the `find_duplicates()` function to deduplicate results before returning.

### 10. Vector Search Doesn't Scale

**Problem:** Semantic search loads ALL embeddings into memory and does O(n) cosine similarity:
```rust
pub fn get_all_embeddings(&self) -> Result<Vec<EmbeddingRecord>>
```

For large codebases with thousands of symbols, this becomes slow.

**Recommendation:** Options:
- Use SQLite with a vector extension (sqlite-vec)
- Implement HNSW (Hierarchical Navigable Small World) indexing
- Use approximate nearest neighbor search
- Lazy loading: only load embeddings for top-N FTS results

### 11. Unused Parameter in Tree Rendering

**Problem:** `src/tree.rs:71` has `_is_last: bool` parameter that's never used.

**Recommendation:** Remove the unused parameter or implement its intended functionality.

### 12. Solidity Call Extraction is Imprecise

**Problem:** The Solidity parser (`src/parser/solidity.rs`) uses a simplistic query that matches all identifiers as potential calls:
```rust
(identifier) @call.name
```

This matches variable names, type names, and other identifiers as function calls.

**Recommendation:** Improve the tree-sitter query to specifically match call expressions.

### 13. YAML Marked as Supported but Has No Parser

**Problem:** YAML is included in `is_supported()` check, so YAML files are indexed, but the parser falls through to the catch-all branch returning empty symbols.

**Location:** `src/parser/mod.rs:125-136`

**Evidence:**
```rust
pub fn is_supported(&self, path: &Path) -> bool {
    matches!(
        Language::from_path(path),
        // ...
        | Language::Yaml  // Listed as supported
    )
}

// But in parse():
_ => {
    // Returns empty symbols for YAML (and Go)
    Some(ParseResult { symbols: Vec::new(), ... })
}
```

**Impact:**
- YAML files are indexed but produce no symbols
- Wastes storage and indexing time
- Misleading documentation claims YAML support

**Recommendation:** Either:
1. Remove YAML from `is_supported()` until a parser is implemented
2. Implement basic YAML parsing (keys as symbols)
3. Document that YAML is "tracked" but not "parsed"

### 14. `hybrid_search` Limit Division Reduces Results for Small Limits

**Problem:** The `hybrid_search()` function divides the limit between exact and semantic branches using integer division, which can reduce effective results for small limits.

**Location:** `src/db/schema.rs:490-529`

**Evidence:**
```rust
let exact_matches = self.find_symbols(query, limit / 2)?;
// ...
if let Ok(semantic_matches) = self.semantic_search(query, limit / 2) {
```

For `limit = 1`: `limit / 2 = 0`, so both branches get limit 0.
For `limit = 3`: each branch gets 1, potentially missing results.

**Impact:**
- Small limits return fewer results than expected
- Users may think no matches exist when they do

**Recommendation:** Clamp the split to at least 1 per branch:
```rust
let half = (limit / 2).max(1);
let exact_matches = self.find_symbols(query, half)?;
```

---

## Lower Priority Improvements

### 15. Add More Language Support

High-value additions:
- **C/C++** - Very common in systems programming
- **Java** - Enterprise codebases
- **Ruby** - Web applications
- **PHP** - Legacy web codebases
- **Swift/Kotlin** - Mobile development

### 16. Add TOML/JSON Config File Support

Instead of just `.contextignore`, support a `.ctx.toml` config file:
```toml
[context]
format = "xml"
default_patterns = ["src/**/*.rs"]

[index]
languages = ["rust", "typescript"]
exclude = ["generated/"]
```

### 17. Add LSP Integration

Expose code intelligence via Language Server Protocol for IDE integration.

### 18. Add `--dry-run` Flag

Show what files would be included without actually generating output.

### 19. Add Progress Indicators

For long-running operations (indexing, embedding), show progress bars using `indicatif`.

### 20. Add Export/Import for Index

Allow exporting the index to JSON for backup or sharing.

### 21. Add `--max-file-size` Option

Skip extremely large files that would overwhelm LLM context windows.

### 22. SVG Files Ignored by Default

**Problem:** SVG files are in the default ignore list (`*.svg`), but SVG is often a text-based format that might be relevant for code context.

**Recommendation:** Consider removing `*.svg` from default ignores or making it configurable.

---

## Performance Improvements

### 23. Batch Database Writes

**Problem:** Currently each symbol/edge is inserted individually during indexing.

**Recommendation:** Batch inserts for 10-50x speedup on large codebases.

### 24. Parallelize Parsing

**Problem:** File parsing is single-threaded.

**Recommendation:** Use `rayon` for parallel file parsing during indexing:
```rust
files.par_iter().map(|f| parser.parse(f)).collect()
```

### 25. Connection Pooling

**Problem:** Single `Connection` per `Database` instance.

**Recommendation:** Use a connection pool for concurrent access.

### 26. Incremental FTS Updates

**Problem:** FTS triggers fire on every insert.

**Recommendation:** Consider batching FTS updates or deferring them.

### 27. Inefficient Clone in File Reading

**Location:** `src/output.rs:154`
```rust
match String::from_utf8(bytes.clone()) {
```

**Problem:** Bytes are cloned even when UTF-8 succeeds.

**Recommendation:** Use `String::from_utf8(bytes)` and only fall back to lossy on error.

---

## Documentation Improvements

### 28. Fix Repository URLs

`Cargo.toml` still has `yourusername` placeholder:
```toml
repository = "https://github.com/yourusername/context"
```

### 29. Add Architecture Diagram

Visual representation of data flow between components.

### 30. Add Benchmarks

Document actual performance numbers for various codebase sizes.

### 31. Add Troubleshooting Guide

Common issues and solutions.

### 32. Maintain Changelog

Keep `CHANGELOG.md` updated with each release.

---

## Summary Table

| # | Priority | Issue | Effort | Impact |
|---|----------|-------|--------|--------|
| 1 | Critical | `new` function over-counting (Resolved) | Medium | High |
| 2 | Critical | Call graph/impact joins on name not ID (Resolved) | Medium | High |
| 3 | High | JSON format not implemented (Resolved) | Low | Medium |
| 4 | High | XML escaping missing (Resolved) | Low | Medium |
| 5 | High | Fragile OpenAI HTTP client | Medium | High |
| 6 | High | `ctx source` ignores patterns (Resolved) | Medium | High |
| 7 | High | `ctx index` ignores CLI flags (Resolved) | Low | High |
| 8 | Medium | Go parser not implemented | Medium | Medium |
| 9 | Medium | Duplicate detection shows repeated pairs (Resolved) | Low | Low |
| 10 | Medium | Vector search scalability | High | High |
| 11 | Medium | Unused tree parameter | Low | Low |
| 12 | Medium | Solidity call extraction imprecise | Medium | Low |
| 13 | Medium | YAML marked supported but no parser | Low | Low |
| 14 | Medium | `hybrid_search` limit division issue | Low | Medium |
| 15-22 | Low | Various feature additions | Varies | Medium |
| 23-27 | Low | Performance improvements | Varies | Medium |
| 28-32 | Low | Documentation improvements | Low | Low |
