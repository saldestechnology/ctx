# Code Intelligence

ctx includes a powerful code intelligence system that indexes your codebase, extracts symbols and relationships, and enables sophisticated queries.

## Building the Index

### Basic Indexing

```bash
ctx index
```

This creates `.ctx/codebase.sqlite` containing:
- **Symbols** - Functions, classes, interfaces, structs, enums, traits
- **Edges** - Call relationships, imports, dependencies
- **Files** - Metadata and compressed source code
- **FTS Index** - Full-text search across symbol names and documentation

### Incremental Updates

By default, ctx only reindexes files that have changed (based on content hash):

```bash
ctx index  # Only processes modified files
```

### Force Full Reindex

When you update `.contextignore` or want a clean slate:

```bash
ctx index --force
```

This removes the existing database and rebuilds from scratch.

### Watch Mode

Automatically reindex when files change:

```bash
ctx index --watch
```

Press `Ctrl+C` to stop.

### Verbose Output

See which files are being indexed:

```bash
ctx index --verbose
```

## Searching

### Basic Search

```bash
ctx search "handleRequest"
```

Returns symbols matching the query, ranked by relevance.

### Semantic Search

ctx uses FTS5 for intelligent matching:

```bash
ctx search "error handling"     # Matches handleError, ErrorHandler, etc.
ctx search "auth token"         # Matches authentication-related symbols
ctx search "parse config"       # Finds configuration parsers
```

### Search Options

```bash
ctx search "query" --limit 10   # Limit results
ctx search "query" --output json  # JSON output
```

## Querying Relationships

### Find Symbols

Search by name pattern:

```bash
ctx query find "handle*"              # Wildcard matching
ctx query find "process" --kind function  # Filter by kind
ctx query find "User" --limit 5       # Limit results
```

### Callers (Who Calls This?)

Find all functions that call a given function:

```bash
ctx query callers authenticate
ctx query callers processPayment --depth 3
```

Output shows the call chain:
```
Functions that call 'authenticate':
------------------------------------------------------------
  handleLogin (src/auth/login.ts:45)
    > authenticate(username, password)
  validateSession (src/auth/session.ts:23)
    > const user = authenticate(token)
```

### Dependencies (What Does This Call?)

See what a function depends on:

```bash
ctx query deps handleRequest
ctx query deps UserService --depth 2
```

Output:
```
Dependencies of 'handleRequest':
------------------------------------------------------------
  calls validateInput (line 12)
  calls processData (line 18)
  calls sendResponse (line 25)
```

### Call Graph

Visualize the call graph from a starting point:

```bash
# Text format (default)
ctx query graph main --depth 3

# JSON format
ctx query graph main --depth 3 --output json

# GraphViz DOT format
ctx query graph main --depth 3 --output dot > graph.dot
dot -Tpng graph.dot -o graph.png
```

### Impact Analysis

Understand what would be affected by changing a symbol:

```bash
ctx query impact validateToken --depth 5
```

Output:
```
Impact analysis for 'validateToken' (depth=5):
The following would be affected by changes:
----------------------------------------------------------------------

Distance 1:
  authenticate (src/auth/auth.ts) [function]
  refreshToken (src/auth/refresh.ts) [function]

Distance 2:
  handleLogin (src/routes/login.ts) [function]
  protectedRoute (src/middleware/auth.ts) [function]

Distance 3:
  UserController (src/controllers/user.ts) [class]

Total: 5 symbols affected
```

### Statistics

Get an overview of your codebase:

```bash
ctx query stats
```

Output:
```
Codebase Statistics
============================================================
Files indexed:  218
Total symbols:  1727
  - Functions:  1009
  - Structs:    71
  - Enums:      3
  - Traits:     167
Total edges:    2996

Per-file breakdown:
------------------------------------------------------------
FILE                                 TOTAL  FUNCS    PUB  TYPES
src/auth/handler.ts                     58     46     52      0
src/api/routes.ts                       55     41     48      0
...

Most connected functions:
------------------------------------------------------------
FUNCTION                        CALLS OUT  CALLED BY
handleRequest                         203          0
processData                           107          5
validateInput                          94         12
```

### List Files

See all indexed files:

```bash
ctx query files
```

## Symbol Information

### Explain

Get detailed information about a symbol:

```bash
ctx explain handleAuth
```

Output:
```
Symbol: handleAuth
============================================================
Kind:       function
File:       src/auth/handler.ts:45
Visibility: public

Signature:
  async function handleAuth(req: Request): Promise<Response>

Description:
  Handles authentication requests and returns JWT tokens.

Called by (3):
  loginRoute (src/routes/auth.ts:12)
  refreshRoute (src/routes/auth.ts:34)
  apiMiddleware (src/middleware/api.ts:8)

Calls (5):
  validateCredentials [function]
  generateToken [function]
  hashPassword [function]
  ...
```

### Source

Retrieve the source code for a symbol:

```bash
ctx source handleAuth
```

Output:
```typescript
// Source: src/auth/handler.ts::handleAuth::45
async function handleAuth(req: Request): Promise<Response> {
  const { username, password } = req.body;
  
  const user = await validateCredentials(username, password);
  if (!user) {
    return new Response("Unauthorized", { status: 401 });
  }
  
  const token = generateToken(user);
  return Response.json({ token });
}
```

## Database Location

The index is stored at `.ctx/codebase.sqlite` in your project root. This single file contains:

- Symbol definitions
- Call graph edges
- Compressed source code
- FTS5 search index

You can:
- Add `.ctx/` to `.gitignore` (recommended for most projects)
- Commit it for shared code intelligence
- Back it up for large codebases

## Performance Tips

1. **Use `.contextignore`** - Exclude test fixtures, generated code, and vendored dependencies
2. **Incremental indexing** - Let ctx only reindex changed files
3. **Watch mode for development** - Keep the index fresh automatically
4. **Force reindex sparingly** - Only when necessary (e.g., after updating ignores)
