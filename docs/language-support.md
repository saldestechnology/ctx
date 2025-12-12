# Language Support

ctx uses tree-sitter for parsing, providing accurate symbol extraction across multiple languages.

## Supported Languages

| Language | Extensions | Parser | Status |
|----------|-----------|--------|--------|
| Rust | `.rs` | tree-sitter-rust | Full |
| TypeScript | `.ts` | tree-sitter-typescript | Full |
| TSX | `.tsx` | tree-sitter-typescript | Full |
| JavaScript | `.js`, `.mjs`, `.cjs` | tree-sitter-javascript | Full |
| JSX | `.jsx` | tree-sitter-javascript | Full |
| Python | `.py`, `.pyi` | tree-sitter-python | Full |
| Solidity | `.sol` | tree-sitter-solidity | Full |
| YAML | `.yaml`, `.yml` | N/A | File tracking only |
| Go | `.go` | tree-sitter-go | Planned |

## Symbol Extraction by Language

### Rust

**Extracted Symbols:**
- Functions (`fn`)
- Methods (in `impl` blocks)
- Structs
- Enums
- Traits
- Type aliases

**Example:**
```rust
/// A user in the system.
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    /// Create a new user.
    pub fn new(name: &str) -> Self {
        Self { id: 0, name: name.to_string() }
    }
    
    /// Validate the user.
    fn validate(&self) -> bool {
        !self.name.is_empty()
    }
}
```

**Extracted:**
- `User` (struct, public)
- `User::new` (method, public)
- `User::validate` (method, private)

**Edge Tracking:**
- **Calls**: `validate()`, `to_string()`
- **Implements**: `impl Animal for Dog` creates an implements edge from `Dog` to `Animal`
- **Imports**: `use std::collections::HashMap` creates an import edge

### TypeScript

**Extracted Symbols:**
- Functions (named and arrow)
- Classes
- Methods
- Interfaces
- Type aliases
- Enums

**Example:**
```typescript
/** User service for authentication. */
export interface UserService {
  authenticate(token: string): Promise<User>;
}

/** Default implementation. */
export class DefaultUserService implements UserService {
  constructor(private db: Database) {}
  
  async authenticate(token: string): Promise<User> {
    const decoded = decodeToken(token);
    return this.db.findUser(decoded.userId);
  }
}

/** Decode a JWT token. */
export const decodeToken = (token: string): TokenPayload => {
  return jwt.decode(token);
};
```

**Extracted:**
- `UserService` (interface, public)
- `DefaultUserService` (class, public)
- `DefaultUserService.authenticate` (method, public)
- `decodeToken` (function, public)

**Edge Tracking:**
- **Extends**: `class Child extends Parent` creates an extends edge
- **Implements**: `class Foo implements IBar` creates an implements edge
- **Imports**: `import { x } from 'module'` creates import edges

### JavaScript/JSX

Same as TypeScript, minus type-specific constructs (interfaces, type aliases, enums).

**Additional JSX Support:**
- React components (function components)
- Arrow function components

### TSX

Combines TypeScript and JSX support:

```tsx
interface ButtonProps {
  label: string;
  onClick: () => void;
}

export const Button: React.FC<ButtonProps> = ({ label, onClick }) => {
  return <button onClick={onClick}>{label}</button>;
};
```

**Extracted:**
- `ButtonProps` (interface, private)
- `Button` (function, public)

### Python

**Extracted Symbols:**
- Functions (including async functions)
- Classes
- Methods (including decorated methods)
- Module-level constants (UPPER_CASE names)

**Example:**
```python
"""User module for authentication."""

MAX_RETRIES = 3

class User:
    """A user in the system."""
    
    def __init__(self, name: str):
        """Initialize the user."""
        self.name = name
    
    def validate(self) -> bool:
        """Validate the user."""
        return len(self.name) > 0
    
    @staticmethod
    def from_dict(data: dict) -> "User":
        """Create a user from a dictionary."""
        return User(data["name"])

async def fetch_user(user_id: int) -> User:
    """Fetch a user from the database."""
    data = await db.get(user_id)
    return User.from_dict(data)
```

**Extracted:**
- `MAX_RETRIES` (constant, public)
- `User` (class, public)
- `User.__init__` (method, private - starts with `_`)
- `User.validate` (method, public)
- `User.from_dict` (method, public)
- `fetch_user` (function, public)

**Visibility Rules:**
- Names starting with `_` are private
- Names starting with `__` (but not ending with `__`) use name mangling (private)
- All other names are public

**Edge Tracking:**
- **Calls**: `validate()`, `db.get()`, `User.from_dict()`
- **Extends**: `class Dog(Animal)` creates an extends edge from `Dog` to `Animal`
- **Imports**: `from typing import List` creates import edges

### Solidity

**Extracted Symbols:**
- Contracts
- Functions
- Modifiers
- Events
- Structs
- Enums
- Errors

**Example:**
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title Token contract
contract Token {
    mapping(address => uint256) public balances;
    
    event Transfer(address indexed from, address indexed to, uint256 amount);
    
    error InsufficientBalance(uint256 available, uint256 required);
    
    /// @notice Transfer tokens
    function transfer(address to, uint256 amount) external {
        if (balances[msg.sender] < amount) {
            revert InsufficientBalance(balances[msg.sender], amount);
        }
        balances[msg.sender] -= amount;
        balances[to] += amount;
        emit Transfer(msg.sender, to, amount);
    }
}
```

**Extracted:**
- `Token` (contract, public)
- `Token.transfer` (function, external)
- `Transfer` (event)
- `InsufficientBalance` (error)

### YAML

YAML files are tracked but not parsed for symbols (YAML doesn't have functions/classes).

**What's indexed:**
- File path
- File hash (for change detection)
- Language type

**Use case:**
- Include config files in `ctx query files`
- Track changes to CI/CD configs
- Include in context generation

## Adding Language Support

To add a new language:

1. Add the tree-sitter crate to `Cargo.toml`
2. Create a parser module in `src/parser/`
3. Define tree-sitter queries for symbol extraction
4. Add the language to the `Language` enum
5. Update `is_supported()` in `CodeParser`

See `src/parser/rust.rs` or `src/parser/typescript.rs` for examples.

## Limitations

### Cross-File Resolution

Call targets are resolved by name within the same codebase. External calls (to libraries) show as unresolved:

```
ctx query deps myFunction
Dependencies of 'myFunction':
------------------------------------------------------------
  calls internalHelper (line 12)      # Resolved
  calls externalLib (line 15)         # Unresolved (external)
```

### Dynamic Calls

Dynamic or computed calls cannot be tracked:

```typescript
// Static call - tracked
processData(input);

// Dynamic call - not tracked
const fn = getHandler(type);
fn(input);
```

### Macros

Macro-generated code is not analyzed in Rust:

```rust
// The generated impl is not indexed
#[derive(Debug, Clone)]
struct MyStruct { ... }
```

### Type Inference

We don't perform type inference, so method calls on inferred types may not resolve:

```typescript
// Resolved (explicit type)
const user: User = getUser();
user.validate();

// May not resolve (inferred type)
const user = getUser();
user.validate();  // We don't know user is User
```
