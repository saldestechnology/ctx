# Configuration

ctx uses file-based configuration through ignore files. There are no config files or environment variables required.

## .contextignore

Create a `.contextignore` file in your project root to exclude files from both context generation and code intelligence indexing.

### Syntax

Uses the same syntax as `.gitignore`:

```gitignore
# Comments start with #

# Ignore specific files
secret.key
config.local.json

# Ignore directories (trailing slash)
node_modules/
dist/
build/

# Glob patterns
*.test.ts
*.spec.js
**/__tests__/**

# Negation (include despite earlier rules)
!important.test.ts
```

### Common Patterns

#### JavaScript/TypeScript Projects

```gitignore
# Dependencies (usually in .gitignore, but be explicit)
node_modules/

# Build outputs
dist/
build/
.next/
out/

# Test files
**/*.test.ts
**/*.spec.ts
**/__tests__/
**/__mocks__/
coverage/

# Generated files
*.generated.ts
*.d.ts
generated/

# Storybook
storybook-static/

# Config files (usually not useful for AI context)
*.config.js
*.config.ts
tsconfig.json
```

#### Rust Projects

```gitignore
# Build output
target/

# Generated files
*.generated.rs

# Benchmarks (usually not needed)
benches/

# Examples (optional)
examples/
```

#### Solidity Projects

```gitignore
# Foundry
cache/
out/
broadcast/

# Vendored dependencies
lib/forge-std/
lib/openzeppelin-contracts/
lib/solmate/

# Generated
*.generated.sol
typechain-types/
```

#### Monorepo Projects

```gitignore
# All node_modules
**/node_modules/

# All build outputs
**/dist/
**/build/
**/.next/

# Vendored libs in any package
**/lib/forge-std/
**/lib/openzeppelin-*/

# Package-specific ignores
packages/*/coverage/
apps/*/storybook-static/
```

## Built-in Ignores

ctx automatically excludes 170+ patterns. These are always applied unless you use `--no-default-ignores`.

### Categories

**Version Control**
- `.git/`, `.svn/`, `.hg/`, `.bzr/`

**Dependencies**
- `node_modules/`, `vendor/`, `Pods/`, `target/`

**Build Outputs**
- `dist/`, `build/`, `out/`, `.next/`, `.nuxt/`

**Binary Files**
- Images: `*.png`, `*.jpg`, `*.gif`, `*.ico`, `*.webp`, `*.svg`
- Media: `*.mp3`, `*.mp4`, `*.avi`, `*.mov`
- Executables: `*.exe`, `*.dll`, `*.so`, `*.dylib`
- Archives: `*.zip`, `*.tar`, `*.gz`, `*.rar`

**Lock Files**
- `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`
- `Cargo.lock`, `Gemfile.lock`, `poetry.lock`

**Environment & Secrets**
- `.env`, `.env.*`, `*.pem`, `*.key`, `*.crt`

**IDE & Editor**
- `.vscode/`, `.idea/`, `*.swp`, `*.swo`

**Cache & Temp**
- `.cache/`, `tmp/`, `temp/`, `*.tmp`

See [default_ignores.rs](https://github.com/yourusername/ctx/blob/main/src/default_ignores.rs) for the complete list.

## Command-Line Ignores

Add patterns on the fly:

```bash
ctx -i "*.test.ts" -i "fixtures/" src/
```

These are combined with `.gitignore`, `.contextignore`, and built-in patterns.

## Disabling Ignores

### Disable .gitignore

```bash
ctx --no-gitignore src/
```

### Disable Built-in Patterns

```bash
ctx --no-default-ignores src/
```

### Include Everything

```bash
ctx --no-gitignore --no-default-ignores src/
```

Note: `.contextignore` is always respected if present.

## Database Location

The code intelligence database is stored at:

```
your-project/
└── .ctx/
    └── codebase.sqlite
```

### Git Recommendations

**Ignore (Recommended for most projects)**

Add to `.gitignore`:
```gitignore
.ctx/
```

This is recommended because:
- Database can be rebuilt with `ctx index`
- Avoids large binary files in git
- Each developer can have their own index

**Commit (For shared code intelligence)**

Don't ignore `.ctx/` if you want to:
- Share the index across the team
- Include code intelligence in CI/CD
- Avoid rebuild time for new clones

## Environment Variables

ctx does not use environment variables. All configuration is done through:
- Command-line arguments
- `.contextignore` files
- `.gitignore` files
