//! SQLite schema and database operations.

use std::path::Path;

use rusqlite::{params, Connection, Result, Transaction};

use super::models::*;

/// SQLite database for code intelligence.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::configure_connection(&conn)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Create an in-memory database (for testing).
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_connection(&conn)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Configure the SQLite connection for optimal performance and concurrency.
    fn configure_connection(conn: &Connection) -> Result<()> {
        // Enable WAL mode for better concurrent access
        // This allows multiple readers and one writer simultaneously
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA cache_size = -64000;
            "#,
        )?;
        Ok(())
    }

    /// Initialize the database schema.
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Enable foreign keys
            PRAGMA foreign_keys = ON;

            -- File tracking for incremental updates
            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                content_hash TEXT NOT NULL,
                size_bytes INTEGER,
                language TEXT,
                last_indexed INTEGER DEFAULT (unixepoch()),
                source BLOB
            );

            -- All symbols (functions, structs, etc.)
            CREATE TABLE IF NOT EXISTS symbols (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                name TEXT NOT NULL,
                qualified_name TEXT,
                kind TEXT NOT NULL,
                visibility TEXT DEFAULT 'private',
                signature TEXT,
                brief TEXT,
                docstring TEXT,
                line_start INTEGER,
                line_end INTEGER,
                col_start INTEGER,
                col_end INTEGER,
                parent_id TEXT,
                source TEXT,
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            -- Relationships between symbols
            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id TEXT NOT NULL,
                target_id TEXT,
                target_name TEXT NOT NULL,
                kind TEXT NOT NULL,
                line INTEGER,
                col INTEGER,
                context TEXT,
                FOREIGN KEY (source_id) REFERENCES symbols(id) ON DELETE CASCADE
            );

            -- Module-level information
            CREATE TABLE IF NOT EXISTS modules (
                file_path TEXT PRIMARY KEY,
                module_name TEXT,
                exports TEXT,
                imports TEXT,
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            -- Indexes for fast lookups
            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path);
            CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
            CREATE INDEX IF NOT EXISTS idx_symbols_parent ON symbols(parent_id);
            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_name);
            CREATE INDEX IF NOT EXISTS idx_edges_kind ON edges(kind);
            CREATE INDEX IF NOT EXISTS idx_files_hash ON files(content_hash);

            -- Embeddings for semantic search
            CREATE TABLE IF NOT EXISTS embeddings (
                symbol_id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                dimension INTEGER NOT NULL,
                vector TEXT NOT NULL,  -- JSON array of floats
                created_at INTEGER DEFAULT (unixepoch()),
                FOREIGN KEY (symbol_id) REFERENCES symbols(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_embeddings_provider ON embeddings(provider);

            -- Full-text search index for semantic search
            CREATE VIRTUAL TABLE IF NOT EXISTS symbol_fts USING fts5(
                id,
                name,
                kind,
                signature,
                brief,
                docstring,
                content='symbols',
                content_rowid='rowid'
            );

            -- Triggers to keep FTS index in sync
            CREATE TRIGGER IF NOT EXISTS symbols_ai AFTER INSERT ON symbols BEGIN
                INSERT INTO symbol_fts(rowid, id, name, kind, signature, brief, docstring)
                VALUES (NEW.rowid, NEW.id, NEW.name, NEW.kind, NEW.signature, NEW.brief, NEW.docstring);
            END;

            CREATE TRIGGER IF NOT EXISTS symbols_ad AFTER DELETE ON symbols BEGIN
                INSERT INTO symbol_fts(symbol_fts, rowid, id, name, kind, signature, brief, docstring)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.name, OLD.kind, OLD.signature, OLD.brief, OLD.docstring);
            END;

            CREATE TRIGGER IF NOT EXISTS symbols_au AFTER UPDATE ON symbols BEGIN
                INSERT INTO symbol_fts(symbol_fts, rowid, id, name, kind, signature, brief, docstring)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.name, OLD.kind, OLD.signature, OLD.brief, OLD.docstring);
                INSERT INTO symbol_fts(rowid, id, name, kind, signature, brief, docstring)
                VALUES (NEW.rowid, NEW.id, NEW.name, NEW.kind, NEW.signature, NEW.brief, NEW.docstring);
            END;
            "#,
        )
    }

    /// Begin a transaction.
    #[allow(dead_code)]
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        self.conn.transaction()
    }

    /// Get the content hash for a file.
    pub fn get_file_hash(&self, path: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT content_hash FROM files WHERE path = ?",
                [path],
                |row| row.get(0),
            )
            .optional()
    }

    /// Check if a file needs reindexing based on hash.
    pub fn needs_update(&self, path: &str, new_hash: &str) -> Result<bool> {
        match self.get_file_hash(path)? {
            Some(stored_hash) => Ok(stored_hash != new_hash),
            None => Ok(true),
        }
    }

    /// Insert or update a file record.
    pub fn upsert_file(&self, file: &FileRecord, source: Option<&[u8]>) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO files (path, content_hash, size_bytes, language, last_indexed, source)
            VALUES (?, ?, ?, ?, unixepoch(), ?)
            "#,
            params![
                file.path,
                file.content_hash,
                file.size_bytes,
                file.language,
                source
            ],
        )?;
        Ok(())
    }

    /// Delete a file and all associated data.
    pub fn delete_file(&self, path: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM files WHERE path = ?", [path])?;
        Ok(())
    }

    /// Delete all symbols for a file.
    pub fn delete_symbols_for_file(&self, file_path: &str) -> Result<()> {
        // Delete edges first (foreign key constraint)
        self.conn.execute(
            "DELETE FROM edges WHERE source_id IN (SELECT id FROM symbols WHERE file_path = ?)",
            [file_path],
        )?;
        self.conn
            .execute("DELETE FROM symbols WHERE file_path = ?", [file_path])?;
        Ok(())
    }

    /// Insert a symbol.
    pub fn insert_symbol(&self, symbol: &Symbol) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO symbols (
                id, file_path, name, qualified_name, kind, visibility,
                signature, brief, docstring, line_start, line_end,
                col_start, col_end, parent_id, source
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                symbol.id,
                symbol.file_path,
                symbol.name,
                symbol.qualified_name,
                symbol.kind.as_str(),
                symbol.visibility.as_str(),
                symbol.signature,
                symbol.brief,
                symbol.docstring,
                symbol.line_start,
                symbol.line_end,
                symbol.col_start,
                symbol.col_end,
                symbol.parent_id,
                symbol.source,
            ],
        )?;
        Ok(())
    }

    /// Insert an edge.
    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO edges (source_id, target_id, target_name, kind, line, col, context)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                edge.source_id,
                edge.target_id,
                edge.target_name,
                edge.kind.as_str(),
                edge.line,
                edge.col,
                edge.context,
            ],
        )?;
        Ok(())
    }

    /// Insert module information.
    pub fn upsert_module(&self, module: &ModuleInfo) -> Result<()> {
        let exports_json = serde_json::to_string(&module.exports).unwrap_or_default();
        let imports_json = serde_json::to_string(&module.imports).unwrap_or_default();

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO modules (file_path, module_name, exports, imports)
            VALUES (?, ?, ?, ?)
            "#,
            params![
                module.file_path,
                module.module_name,
                exports_json,
                imports_json,
            ],
        )?;
        Ok(())
    }

    /// Find a symbol by ID.
    pub fn get_symbol(&self, id: &str) -> Result<Option<Symbol>> {
        self.conn
            .query_row(
                r#"
                SELECT id, file_path, name, qualified_name, kind, visibility,
                       signature, brief, docstring, line_start, line_end,
                       col_start, col_end, parent_id, source
                FROM symbols WHERE id = ?
                "#,
                [id],
                |row| Ok(symbol_from_row(row)),
            )
            .optional()
    }

    /// Find symbols by name (exact or pattern).
    pub fn find_symbols(&self, pattern: &str, limit: i32) -> Result<Vec<Symbol>> {
        self.find_symbols_filtered(pattern, limit, None, None)
    }

    /// Find symbols by name with optional file path and kind filters.
    ///
    /// - `pattern`: Name pattern to search for
    /// - `limit`: Maximum number of results
    /// - `file_pattern`: Optional file path filter (supports glob syntax: `*`, `**`)
    /// - `kind_filter`: Optional symbol kind filter (function, method, struct, etc.)
    ///
    /// Results are ordered by match quality: exact name match first, then prefix match,
    /// then substring match.
    pub fn find_symbols_filtered(
        &self,
        pattern: &str,
        limit: i32,
        file_pattern: Option<&str>,
        kind_filter: Option<&str>,
    ) -> Result<Vec<Symbol>> {
        // Escape SQL LIKE special characters in the pattern
        let escaped_pattern = escape_like_pattern(pattern);
        let like_pattern = format!("%{}%", escaped_pattern);
        let starts_with_pattern = format!("{}%", escaped_pattern);

        // Convert glob-style file pattern to SQL LIKE pattern
        let file_like = file_pattern.map(glob_to_like_pattern);

        // Build dynamic SQL based on filters
        // We use separate parameters for exact match (?1), like match (?2), and starts_with (?3)
        let mut sql = String::from(
            r#"
            SELECT id, file_path, name, qualified_name, kind, visibility,
                   signature, brief, docstring, line_start, line_end,
                   col_start, col_end, parent_id, source
            FROM symbols
            WHERE (name LIKE ?2 OR qualified_name LIKE ?2)
            "#,
        );

        // Track next parameter position
        let mut next_param = 4; // ?1=pattern, ?2=like_pattern, ?3=starts_with

        // Add file pattern filter if provided
        let file_param_pos = if file_pattern.is_some() {
            let pos = next_param;
            sql.push_str(&format!(" AND file_path LIKE ?{}", pos));
            next_param += 1;
            Some(pos)
        } else {
            None
        };

        // Add kind filter if provided
        let kind_param_pos = if kind_filter.is_some() {
            let pos = next_param;
            sql.push_str(&format!(" AND kind = ?{}", pos));
            next_param += 1;
            Some(pos)
        } else {
            None
        };

        // Add ORDER BY with proper exact match detection
        // ?1 is the original pattern (for exact match)
        // ?3 is the starts_with pattern (for prefix match)
        sql.push_str(&format!(
            r#"
            ORDER BY 
                CASE WHEN name = ?1 THEN 0 
                     WHEN name LIKE ?3 THEN 1 
                     ELSE 2 END,
                name
            LIMIT ?{}
            "#,
            next_param
        ));

        let mut stmt = self.conn.prepare(&sql)?;

        // Execute with appropriate parameters based on which filters are active
        let rows: Vec<Result<Symbol>> = match (file_like.as_deref(), kind_filter, file_param_pos, kind_param_pos) {
            (Some(fp), Some(kf), Some(_), Some(_)) => stmt
                .query_map(params![pattern, like_pattern, starts_with_pattern, fp, kf, limit], |row| {
                    Ok(symbol_from_row(row))
                })?
                .collect(),
            (Some(fp), None, Some(_), None) => stmt
                .query_map(params![pattern, like_pattern, starts_with_pattern, fp, limit], |row| {
                    Ok(symbol_from_row(row))
                })?
                .collect(),
            (None, Some(kf), None, Some(_)) => stmt
                .query_map(params![pattern, like_pattern, starts_with_pattern, kf, limit], |row| {
                    Ok(symbol_from_row(row))
                })?
                .collect(),
            (None, None, None, None) => stmt
                .query_map(params![pattern, like_pattern, starts_with_pattern, limit], |row| {
                    Ok(symbol_from_row(row))
                })?
                .collect(),
            // Handle impossible cases (filter provided but no param pos)
            _ => unreachable!("Filter and param position should match"),
        };

        rows.into_iter().collect()
    }

    /// Get the source code for a symbol.
    pub fn get_source(&self, symbol_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT source FROM symbols WHERE id = ?",
                [symbol_id],
                |row| row.get(0),
            )
            .optional()
    }

    /// Get all symbols in a file.
    #[allow(dead_code)]
    pub fn get_file_symbols(&self, file_path: &str) -> Result<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, file_path, name, qualified_name, kind, visibility,
                   signature, brief, docstring, line_start, line_end,
                   col_start, col_end, parent_id, source
            FROM symbols
            WHERE file_path = ?
            ORDER BY line_start
            "#,
        )?;

        let rows = stmt.query_map([file_path], |row| Ok(symbol_from_row(row)))?;
        rows.collect()
    }

    /// Get edges from a symbol.
    pub fn get_outgoing_edges(&self, symbol_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT source_id, target_id, target_name, kind, line, col, context
            FROM edges
            WHERE source_id = ?
            ORDER BY line
            "#,
        )?;

        let rows = stmt.query_map([symbol_id], |row| Ok(edge_from_row(row)))?;
        rows.collect()
    }

    /// Get edges to a symbol (callers).
    pub fn get_incoming_edges(&self, target_name: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT source_id, target_id, target_name, kind, line, col, context
            FROM edges
            WHERE target_name = ? OR target_id = ?
            ORDER BY source_id
            "#,
        )?;

        let rows = stmt.query_map([target_name, target_name], |row| Ok(edge_from_row(row)))?;
        rows.collect()
    }

    /// Get codebase statistics.
    pub fn get_stats(&self) -> Result<CodebaseStats> {
        let files: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
        let symbols: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))?;
        let edges: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        let functions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE kind IN ('function', 'method')",
            [],
            |row| row.get(0),
        )?;
        let structs: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE kind IN ('struct', 'class')",
            [],
            |row| row.get(0),
        )?;
        let enums: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE kind = 'enum'",
            [],
            |row| row.get(0),
        )?;
        let traits: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE kind IN ('trait', 'interface')",
            [],
            |row| row.get(0),
        )?;

        Ok(CodebaseStats {
            files,
            symbols,
            edges,
            functions,
            structs,
            enums,
            traits,
        })
    }

    /// Get all indexed file paths.
    pub fn get_indexed_files(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM files ORDER BY path")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// Semantic search using FTS5 full-text search.
    /// Searches across name, signature, brief, and docstring fields.
    pub fn semantic_search(&self, query: &str, limit: i32) -> Result<Vec<(Symbol, f64)>> {
        // Preprocess query: split into keywords, handle natural language
        let keywords = preprocess_search_query(query);

        if keywords.is_empty() {
            return Ok(Vec::new());
        }

        // Build FTS5 query with OR logic for broader matches
        let fts_query = keywords.join(" OR ");

        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                s.id, s.file_path, s.name, s.qualified_name, s.kind, s.visibility,
                s.signature, s.brief, s.docstring, s.line_start, s.line_end,
                s.col_start, s.col_end, s.parent_id, s.source,
                bm25(symbol_fts) as rank
            FROM symbol_fts
            JOIN symbols s ON symbol_fts.id = s.id
            WHERE symbol_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map(params![fts_query, limit], |row| {
            let symbol = symbol_from_row(row);
            let rank: f64 = row.get(15)?;
            // Convert BM25 score (negative, lower is better) to a 0-1 relevance score
            let relevance = 1.0 / (1.0 - rank);
            Ok((symbol, relevance))
        })?;

        rows.collect()
    }

    /// Hybrid search combining exact match with semantic search.
    pub fn hybrid_search(&self, query: &str, limit: i32) -> Result<Vec<(Symbol, f64, String)>> {
        let mut results: std::collections::HashMap<String, (Symbol, f64, String)> =
            std::collections::HashMap::new();

        // 1. Exact name matches (highest priority)
        let exact_matches = self.find_symbols(query, limit / 2)?;
        for symbol in exact_matches {
            let score = if symbol.name.eq_ignore_ascii_case(query) {
                1.0 // Exact match
            } else if symbol
                .name
                .to_lowercase()
                .starts_with(&query.to_lowercase())
            {
                0.9 // Prefix match
            } else {
                0.7 // Contains match
            };
            results.insert(symbol.id.clone(), (symbol, score, "exact".to_string()));
        }

        // 2. Semantic matches (FTS5)
        if let Ok(semantic_matches) = self.semantic_search(query, limit / 2) {
            for (symbol, relevance) in semantic_matches {
                results
                    .entry(symbol.id.clone())
                    .and_modify(|(_, existing_score, _)| {
                        *existing_score = existing_score.max(relevance);
                    })
                    .or_insert((symbol, relevance, "semantic".to_string()));
            }
        }

        // Sort by score and return
        let mut results: Vec<_> = results.into_values().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit as usize);

        Ok(results)
    }

    /// Rebuild the FTS index (useful after schema changes).
    #[allow(dead_code)]
    pub fn rebuild_fts_index(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            INSERT INTO symbol_fts(symbol_fts) VALUES('rebuild');
            "#,
        )?;
        Ok(())
    }

    // ========== Embedding Operations ==========

    /// Store an embedding for a symbol.
    pub fn store_embedding(
        &self,
        symbol_id: &str,
        provider: &str,
        model: &str,
        vector: &[f32],
    ) -> Result<()> {
        let vector_json = serde_json::to_string(vector)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO embeddings (symbol_id, provider, model, dimension, vector)
            VALUES (?, ?, ?, ?, ?)
            "#,
            params![symbol_id, provider, model, vector.len(), vector_json],
        )?;
        Ok(())
    }

    /// Get the embedding for a symbol.
    #[allow(dead_code)]
    pub fn get_embedding(&self, symbol_id: &str) -> Result<Option<Vec<f32>>> {
        let result = self.conn.query_row(
            "SELECT vector FROM embeddings WHERE symbol_id = ?",
            [symbol_id],
            |row| {
                let json: String = row.get(0)?;
                Ok(json)
            },
        );

        match result {
            Ok(json) => {
                let vector: Vec<f32> = serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Some(vector))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all embeddings with their symbol metadata.
    pub fn get_all_embeddings(
        &self,
    ) -> Result<Vec<(String, String, String, String, u32, Vec<f32>)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT e.symbol_id, s.name, s.kind, s.file_path, s.line_start, e.vector
            FROM embeddings e
            JOIN symbols s ON e.symbol_id = s.id
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let symbol_id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let kind: String = row.get(2)?;
            let file_path: String = row.get(3)?;
            let line: u32 = row.get(4)?;
            let json: String = row.get(5)?;
            Ok((symbol_id, name, kind, file_path, line, json))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (symbol_id, name, kind, file_path, line, json) = row?;
            let vector: Vec<f32> = serde_json::from_str(&json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            results.push((symbol_id, name, kind, file_path, line, vector));
        }

        Ok(results)
    }

    /// Count symbols that have embeddings.
    pub fn count_embeddings(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))
    }

    /// Get symbols that don't have embeddings yet.
    pub fn get_symbols_without_embeddings(&self, limit: i64) -> Result<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.id, s.file_path, s.name, s.qualified_name, s.kind, s.visibility,
                   s.signature, s.brief, s.docstring, s.line_start, s.line_end,
                   s.col_start, s.col_end, s.parent_id, s.source
            FROM symbols s
            LEFT JOIN embeddings e ON s.id = e.symbol_id
            WHERE e.symbol_id IS NULL
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(Symbol {
                id: row.get(0)?,
                file_path: row.get(1)?,
                name: row.get(2)?,
                qualified_name: row.get(3)?,
                kind: SymbolKind::from_str(&row.get::<_, String>(4)?)
                    .unwrap_or(SymbolKind::Function),
                visibility: Visibility::from_str(&row.get::<_, String>(5)?),
                signature: row.get(6)?,
                brief: row.get(7)?,
                docstring: row.get(8)?,
                line_start: row.get(9)?,
                line_end: row.get(10)?,
                col_start: row.get(11)?,
                col_end: row.get(12)?,
                parent_id: row.get(13)?,
                source: row.get(14)?,
            })
        })?;

        rows.collect()
    }

    /// Delete embeddings for a specific provider/model.
    pub fn delete_embeddings(&self, provider: &str, model: Option<&str>) -> Result<usize> {
        if let Some(model) = model {
            self.conn.execute(
                "DELETE FROM embeddings WHERE provider = ? AND model = ?",
                params![provider, model],
            )
        } else {
            self.conn
                .execute("DELETE FROM embeddings WHERE provider = ?", [provider])
        }
    }

    /// Resolve target_id for edges that only have target_name.
    /// 
    /// This performs cross-file symbol resolution by matching target_name to symbols
    /// in the database. Resolution priority:
    /// 1. Context match: the call context contains the type name (e.g., "TypeScriptParser::new()")
    /// 2. Unique: only one symbol with that name exists in the codebase
    /// 3. Same file unique: only one symbol with that name exists in the same file
    /// 
    /// We intentionally avoid aggressive same-file matching because calls like
    /// `Vec::new()` would incorrectly match a local `new` function.
    /// 
    /// Returns the number of edges that were resolved.
    pub fn resolve_edge_targets(&self) -> Result<usize> {
        // Step 1: Resolve edges where the context contains the qualified name
        // e.g., context "TypeScriptParser::new()" matches symbol with qualified_name "TypeScriptParser::new"
        // Only resolve if exactly one symbol matches to avoid ambiguous resolution
        let context_resolved = self.conn.execute(
            r#"
            UPDATE edges
            SET target_id = (
                SELECT t.id 
                FROM symbols t
                WHERE t.name = edges.target_name
                  AND t.kind IN ('function', 'method')
                  AND t.qualified_name IS NOT NULL
                  AND edges.context LIKE '%' || t.qualified_name || '%'
            )
            WHERE target_id IS NULL
              AND context IS NOT NULL
              AND (
                SELECT COUNT(*) 
                FROM symbols t
                WHERE t.name = edges.target_name
                  AND t.kind IN ('function', 'method')
                  AND t.qualified_name IS NOT NULL
                  AND edges.context LIKE '%' || t.qualified_name || '%'
              ) = 1
            "#,
            [],
        )?;

        // Step 2: Resolve edges where target name is unique across the codebase
        let unique_resolved = self.conn.execute(
            r#"
            UPDATE edges
            SET target_id = (
                SELECT id FROM symbols 
                WHERE name = edges.target_name
                  AND kind IN ('function', 'method')
            )
            WHERE target_id IS NULL
              AND (
                SELECT COUNT(*) FROM symbols 
                WHERE name = edges.target_name
                  AND kind IN ('function', 'method')
              ) = 1
            "#,
            [],
        )?;

        // Step 3: Resolve edges where target is unique in the same file
        // Only match if:
        // - There's exactly one function with that name in the same file
        // - The context doesn't suggest an external type (no :: prefix before the name)
        // - The context doesn't suggest an external type/receiver call
        //   (no ::, ., or -> before the function name)
        let same_file_unique = self.conn.execute(
            r#"
            UPDATE edges
            SET target_id = (
                SELECT t.id 
                FROM symbols t
                JOIN symbols s ON s.id = edges.source_id
                WHERE t.name = edges.target_name
                  AND t.file_path = s.file_path
                  AND t.kind IN ('function', 'method')
            )
            WHERE target_id IS NULL
              AND (
                SELECT COUNT(*) 
                FROM symbols t
                JOIN symbols s ON s.id = edges.source_id
                WHERE t.name = edges.target_name
                  AND t.file_path = s.file_path
                  AND t.kind IN ('function', 'method')
              ) = 1
              -- Exclude if context suggests an external type call (has :: before the function name)
              -- e.g., "Vec::new()" or "Parser::new()" should NOT match local "new" functions
              AND (
                context IS NULL 
                OR (
                    context NOT LIKE '%::' || target_name || '(%'
                    AND context NOT LIKE '%.' || target_name || '(%'
                    AND context NOT LIKE '%->' || target_name || '(%'
                )
                OR context LIKE '%' || (
                        SELECT t.qualified_name 
                        FROM symbols t
                        JOIN symbols s ON s.id = edges.source_id
                        WHERE t.name = edges.target_name
                          AND t.file_path = s.file_path
                          AND t.kind IN ('function', 'method')
                        LIMIT 1
                    ) || '(%'
              )
            "#,
            [],
        )?;

        Ok(context_resolved + unique_resolved + same_file_unique)
    }

    /// Find duplicate code blocks using content hashing and similarity.
    pub fn find_duplicates(
        &self,
        similarity_threshold: u32,
        min_lines: u32,
    ) -> Result<Vec<DuplicateResult>> {
        // Get all function/method symbols with their source code
        // Use DISTINCT and unique id to avoid duplicate rows
        let mut stmt = self.conn.prepare(
            r#"
            SELECT DISTINCT id, name, file_path, line_start, line_end, source
            FROM symbols
            WHERE kind IN ('function', 'method')
              AND source IS NOT NULL
              AND (line_end - line_start) >= ?
            ORDER BY id
            "#,
        )?;

        let symbols: Vec<(String, String, String, u32, u32, String)> = stmt
            .query_map([min_lines], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut duplicates = Vec::new();
        let threshold = similarity_threshold as f64 / 100.0;
        
        // Track seen pairs to avoid duplicates
        let mut seen_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

        // Compare each pair of symbols
        for i in 0..symbols.len() {
            for j in (i + 1)..symbols.len() {
                let (id1, name1, file1, line1, end1, source1) = &symbols[i];
                let (id2, name2, file2, line2, end2, source2) = &symbols[j];

                // Skip if same symbol (by id or by file+line)
                if id1 == id2 || (file1 == file2 && line1 == line2) {
                    continue;
                }
                
                // Create canonical pair key (smaller id first) to avoid duplicates
                let pair_key = if id1 < id2 {
                    (id1.clone(), id2.clone())
                } else {
                    (id2.clone(), id1.clone())
                };
                
                // Skip if we've already seen this pair
                if seen_pairs.contains(&pair_key) {
                    continue;
                }

                // Normalize and compare source code
                let norm1 = normalize_code(source1);
                let norm2 = normalize_code(source2);

                let similarity = calculate_similarity(&norm1, &norm2);

                if similarity >= threshold {
                    // Mark this pair as seen
                    seen_pairs.insert(pair_key);
                    
                    // Create a content hash for grouping
                    let hash = format!("{:x}", md5_hash(&norm1));

                    duplicates.push(DuplicateResult {
                        name1: name1.clone(),
                        file1: file1.clone(),
                        line1: *line1,
                        name2: name2.clone(),
                        file2: file2.clone(),
                        line2: *line2,
                        similarity: similarity * 100.0,
                        lines: ((end1 - line1) + (end2 - line2)) / 2,
                        hash,
                    });
                }
            }
        }

        // Sort by similarity (highest first)
        duplicates.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(duplicates)
    }
}

/// Escape SQL LIKE special characters in a pattern.
/// 
/// SQLite LIKE uses `%` for any sequence and `_` for single character.
/// This function escapes these so they match literally.
fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")  // Escape backslash first
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Convert a glob-style pattern to a SQL LIKE pattern.
///
/// Supports:
/// - `*` -> `%` (match any sequence - note: in SQL LIKE this also matches `/`)
/// - `**` -> `%` (match any sequence including path separators)
/// - `**/` -> consumed, following pattern matches from any depth
/// - `?` -> `_` (match single character)
/// - Escapes literal `%` and `_` in the pattern
///
/// Limitations:
/// - SQL LIKE `%` matches across path separators, so `src/*.rs` will also match
///   `src/foo/bar.rs`. Use substring patterns like `*parser*` for simple filtering,
///   or rely on the prefix to narrow results.
///
/// Examples:
/// - `**/*.rs` -> `%.rs` (any .rs file at any depth)
/// - `src/**/*.rs` -> `src/%.rs` (any .rs file under src/)
/// - `*parser*` -> `%parser%` (any path containing "parser")
fn glob_to_like_pattern(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                // Check for **
                if chars.peek() == Some(&'*') {
                    chars.next();
                    // Check for **/ - this should match zero or more path segments
                    if chars.peek() == Some(&'/') {
                        chars.next();
                        // **/ means "any path prefix including empty"
                        // Don't add % here - the following pattern (likely *.ext) will add it
                        // This allows **/*.rs to become %.rs instead of %%.rs
                        // But if there's nothing after, or it's not a *, we need the %
                        if chars.peek().is_none() || chars.peek() == Some(&'*') {
                            // **/ at end or followed by another * - don't add redundant %
                            continue;
                        }
                        // **/ followed by something else - add % to match the path prefix
                        result.push('%');
                    } else {
                        // ** without trailing / - just match anything
                        result.push('%');
                    }
                } else {
                    // Single * - match anything (in SQL LIKE, same as %)
                    result.push('%');
                }
            }
            '?' => result.push('_'),
            '%' => result.push_str("\\%"),
            '_' => result.push_str("\\_"),
            '\\' => result.push_str("\\\\"),
            _ => result.push(c),
        }
    }

    result
}

/// Normalize code for comparison (remove whitespace, comments, variable names).
fn normalize_code(code: &str) -> String {
    code.lines()
        .map(|line| {
            // Remove leading/trailing whitespace
            let trimmed = line.trim();
            // Remove single-line comments
            let without_comment = if let Some(idx) = trimmed.find("//") {
                &trimmed[..idx]
            } else if let Some(idx) = trimmed.find('#') {
                &trimmed[..idx]
            } else {
                trimmed
            };
            without_comment.trim()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Calculate similarity between two strings using Jaccard similarity on tokens.
fn calculate_similarity(s1: &str, s2: &str) -> f64 {
    let tokens1: std::collections::HashSet<&str> = s1
        .split(|c: char| {
            c.is_whitespace()
                || c == '('
                || c == ')'
                || c == '{'
                || c == '}'
                || c == ';'
                || c == ','
        })
        .filter(|t| !t.is_empty())
        .collect();

    let tokens2: std::collections::HashSet<&str> = s2
        .split(|c: char| {
            c.is_whitespace()
                || c == '('
                || c == ')'
                || c == '{'
                || c == '}'
                || c == ';'
                || c == ','
        })
        .filter(|t| !t.is_empty())
        .collect();

    if tokens1.is_empty() && tokens2.is_empty() {
        return 1.0;
    }
    if tokens1.is_empty() || tokens2.is_empty() {
        return 0.0;
    }

    let intersection = tokens1.intersection(&tokens2).count();
    let union = tokens1.union(&tokens2).count();

    intersection as f64 / union as f64
}

/// Simple MD5-like hash for grouping duplicates (not cryptographically secure).
fn md5_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Preprocess a search query into keywords.
fn preprocess_search_query(query: &str) -> Vec<String> {
    // Common words to filter out
    let stop_words: std::collections::HashSet<&str> = [
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "dare", "and", "or", "but", "if", "then", "else", "when", "where", "why",
        "how", "what", "which", "who", "whom", "this", "that", "these", "those", "i", "you", "he",
        "she", "it", "we", "they", "me", "him", "her", "us", "them", "my", "your", "his", "its",
        "our", "their", "for", "to", "from", "with", "at", "by", "on", "in", "of", "about", "into",
        "through", "during", "before", "after", "above", "below", "find", "get", "search", "look",
        "all", "any", "each", "every",
    ]
    .iter()
    .copied()
    .collect();

    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| {
            let word = word.trim();
            !word.is_empty() && word.len() > 1 && !stop_words.contains(word)
        })
        .map(|s| {
            // Add wildcard suffix for prefix matching
            format!("{}*", s)
        })
        .collect()
}

/// Helper to convert a row to a Symbol.
fn symbol_from_row(row: &rusqlite::Row) -> Symbol {
    let kind_str: String = row.get(4).unwrap_or_default();
    let visibility_str: String = row.get(5).unwrap_or_default();

    Symbol {
        id: row.get(0).unwrap_or_default(),
        file_path: row.get(1).unwrap_or_default(),
        name: row.get(2).unwrap_or_default(),
        qualified_name: row.get(3).ok(),
        kind: SymbolKind::from_str(&kind_str).unwrap_or(SymbolKind::Function),
        visibility: Visibility::from_str(&visibility_str),
        signature: row.get(6).ok(),
        brief: row.get(7).ok(),
        docstring: row.get(8).ok(),
        line_start: row.get(9).unwrap_or(0),
        line_end: row.get(10).unwrap_or(0),
        col_start: row.get(11).unwrap_or(0),
        col_end: row.get(12).unwrap_or(0),
        parent_id: row.get(13).ok(),
        source: row.get(14).ok(),
    }
}

/// Helper to convert a row to an Edge.
fn edge_from_row(row: &rusqlite::Row) -> Edge {
    let kind_str: String = row.get(3).unwrap_or_default();

    Edge {
        source_id: row.get(0).unwrap_or_default(),
        target_id: row.get(1).ok(),
        target_name: row.get(2).unwrap_or_default(),
        kind: EdgeKind::from_str(&kind_str).unwrap_or(EdgeKind::Calls),
        line: row.get(4).ok(),
        col: row.get(5).ok(),
        context: row.get(6).ok(),
    }
}

/// Result of duplicate detection.
#[derive(Debug, Clone)]
pub struct DuplicateResult {
    pub name1: String,
    pub file1: String,
    pub line1: u32,
    pub name2: String,
    pub file2: String,
    pub line2: u32,
    pub similarity: f64,
    pub lines: u32,
    pub hash: String,
}

/// Extension trait for optional query results.
trait ResultExt<T> {
    fn optional(self) -> Result<Option<T>>;
}

impl<T> ResultExt<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_database() {
        let db = Database::open_in_memory().unwrap();
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.symbols, 0);
    }

    #[test]
    fn test_insert_and_find_symbol() {
        let db = Database::open_in_memory().unwrap();

        // Insert a file first (foreign key)
        let file = FileRecord {
            path: "src/main.rs".to_string(),
            content_hash: "abc123".to_string(),
            size_bytes: 100,
            language: Some("rust".to_string()),
            last_indexed: 0,
        };
        db.upsert_file(&file, None).unwrap();

        // Insert a symbol
        let symbol = Symbol {
            id: "src/main.rs::main".to_string(),
            file_path: "src/main.rs".to_string(),
            name: "main".to_string(),
            qualified_name: None,
            kind: SymbolKind::Function,
            visibility: Visibility::Private,
            signature: Some("fn main()".to_string()),
            brief: Some("Entry point".to_string()),
            docstring: None,
            line_start: 1,
            line_end: 5,
            col_start: 0,
            col_end: 1,
            parent_id: None,
            source: Some("fn main() {\n    println!(\"Hello\");\n}".to_string()),
        };
        db.insert_symbol(&symbol).unwrap();

        // Find it
        let found = db.get_symbol("src/main.rs::main").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "main");

        // Search for it
        let results = db.find_symbols("main", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_find_symbols_exact_match_ordering() {
        let db = Database::open_in_memory().unwrap();

        // Insert a file
        let file = FileRecord {
            path: "src/test.rs".to_string(),
            content_hash: "abc123".to_string(),
            size_bytes: 100,
            language: Some("rust".to_string()),
            last_indexed: 0,
        };
        db.upsert_file(&file, None).unwrap();

        // Insert symbols with similar names - order matters for testing
        // We insert in reverse order to ensure ordering comes from query, not insertion
        let symbols = vec![
            ("new_large", "src/test.rs::new_large"),      // substring match
            ("new_item", "src/test.rs::new_item"),        // prefix match
            ("renew", "src/test.rs::renew"),              // substring match (contains "new")
            ("new", "src/test.rs::new"),                  // exact match
            ("new_thing", "src/test.rs::new_thing"),      // prefix match
        ];

        for (name, id) in &symbols {
            let symbol = Symbol {
                id: id.to_string(),
                file_path: "src/test.rs".to_string(),
                name: name.to_string(),
                qualified_name: None,
                kind: SymbolKind::Function,
                visibility: Visibility::Public,
                signature: None,
                brief: None,
                docstring: None,
                line_start: 1,
                line_end: 5,
                col_start: 0,
                col_end: 1,
                parent_id: None,
                source: None,
            };
            db.insert_symbol(&symbol).unwrap();
        }

        // Search for "new" - should get exact match first, then prefix matches, then substring
        let results = db.find_symbols("new", 10).unwrap();

        assert!(results.len() >= 4, "Should find at least 4 symbols");

        // First result should be exact match "new"
        assert_eq!(results[0].name, "new", "Exact match 'new' should be first");

        // Next results should be prefix matches (new_*), alphabetically sorted
        // new_item, new_large, new_thing should come before renew
        let prefix_matches: Vec<&str> = results[1..4]
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(
            prefix_matches.iter().all(|n| n.starts_with("new")),
            "Positions 2-4 should be prefix matches, got: {:?}",
            prefix_matches
        );

        // "renew" should be last (substring but not prefix)
        let renew_pos = results.iter().position(|s| s.name == "renew");
        assert!(
            renew_pos.is_some() && renew_pos.unwrap() >= 4,
            "'renew' should be after prefix matches"
        );
    }

    #[test]
    fn test_find_symbols_with_file_filter() {
        let db = Database::open_in_memory().unwrap();

        // Insert two files
        for path in &["src/parser/rust.rs", "src/embeddings/local.rs"] {
            let file = FileRecord {
                path: path.to_string(),
                content_hash: "abc123".to_string(),
                size_bytes: 100,
                language: Some("rust".to_string()),
                last_indexed: 0,
            };
            db.upsert_file(&file, None).unwrap();
        }

        // Insert "new" in both files
        for (path, id) in &[
            ("src/parser/rust.rs", "src/parser/rust.rs::new"),
            ("src/embeddings/local.rs", "src/embeddings/local.rs::new"),
        ] {
            let symbol = Symbol {
                id: id.to_string(),
                file_path: path.to_string(),
                name: "new".to_string(),
                qualified_name: None,
                kind: SymbolKind::Method,
                visibility: Visibility::Public,
                signature: None,
                brief: None,
                docstring: None,
                line_start: 1,
                line_end: 5,
                col_start: 0,
                col_end: 1,
                parent_id: None,
                source: None,
            };
            db.insert_symbol(&symbol).unwrap();
        }

        // Without filter, should find both
        let all_results = db.find_symbols_filtered("new", 10, None, None).unwrap();
        assert_eq!(all_results.len(), 2);

        // With file filter for parser, should find only one
        let filtered = db
            .find_symbols_filtered("new", 10, Some("*parser*"), None)
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].file_path.contains("parser"));

        // With file filter for embeddings
        let filtered = db
            .find_symbols_filtered("new", 10, Some("*embeddings*"), None)
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].file_path.contains("embeddings"));
    }

    #[test]
    fn test_escape_like_pattern() {
        // Normal patterns should pass through
        assert_eq!(escape_like_pattern("new"), "new");
        assert_eq!(escape_like_pattern("foo_bar"), "foo\\_bar");

        // Special SQL LIKE characters should be escaped
        assert_eq!(escape_like_pattern("100%"), "100\\%");
        assert_eq!(escape_like_pattern("a_b"), "a\\_b");
        assert_eq!(escape_like_pattern("a%b_c"), "a\\%b\\_c");
    }

    #[test]
    fn test_glob_to_like_pattern() {
        // * becomes %
        assert_eq!(glob_to_like_pattern("*.rs"), "%.rs");
        assert_eq!(glob_to_like_pattern("src/*"), "src/%");

        // **/ is consumed when followed by *, preventing double %
        // This ensures **/*.rs becomes %.rs (not %%.rs)
        assert_eq!(glob_to_like_pattern("**/*.rs"), "%.rs");
        assert_eq!(glob_to_like_pattern("src/**/*.rs"), "src/%.rs");

        // ** without trailing / just becomes %
        assert_eq!(glob_to_like_pattern("src/**"), "src/%");

        // **/ followed by non-* adds the %
        assert_eq!(glob_to_like_pattern("**/foo.rs"), "%foo.rs");

        // ? becomes _
        assert_eq!(glob_to_like_pattern("file?.txt"), "file_.txt");

        // Literal % and _ are escaped
        assert_eq!(glob_to_like_pattern("100%"), "100\\%");
        assert_eq!(glob_to_like_pattern("a_b"), "a\\_b");
    }

    #[test]
    fn test_glob_pattern_matches_files() {
        let db = Database::open_in_memory().unwrap();

        // Insert files at different depths
        for path in &[
            "main.rs",
            "src/lib.rs",
            "src/parser/mod.rs",
            "src/parser/rust.rs",
            "tests/test.rs",
        ] {
            let file = FileRecord {
                path: path.to_string(),
                content_hash: "abc".to_string(),
                size_bytes: 100,
                language: Some("rust".to_string()),
                last_indexed: 0,
            };
            db.upsert_file(&file, None).unwrap();

            let symbol = Symbol {
                id: format!("{}::main", path),
                file_path: path.to_string(),
                name: "main".to_string(),
                qualified_name: None,
                kind: SymbolKind::Function,
                visibility: Visibility::Public,
                signature: None,
                brief: None,
                docstring: None,
                line_start: 1,
                line_end: 5,
                col_start: 0,
                col_end: 1,
                parent_id: None,
                source: None,
            };
            db.insert_symbol(&symbol).unwrap();
        }

        // **/*.rs should match ALL .rs files at any depth
        let results = db
            .find_symbols_filtered("main", 20, Some("**/*.rs"), None)
            .unwrap();
        assert_eq!(results.len(), 5, "**/*.rs should match all .rs files");

        // src/**/*.rs should match all .rs files under src/
        let results = db
            .find_symbols_filtered("main", 20, Some("src/**/*.rs"), None)
            .unwrap();
        assert_eq!(
            results.len(),
            3,
            "src/**/*.rs should match src/lib.rs, src/parser/mod.rs, src/parser/rust.rs"
        );

        // Note: src/*.rs also matches nested files because SQL LIKE % matches /
        // This is a documented limitation. For precise matching, use substring patterns.
        let results = db
            .find_symbols_filtered("main", 20, Some("src/*.rs"), None)
            .unwrap();
        assert_eq!(
            results.len(),
            3,
            "src/*.rs matches all under src/ (SQL LIKE limitation)"
        );

        // *parser* should match files with "parser" in the path
        let results = db
            .find_symbols_filtered("main", 20, Some("*parser*"), None)
            .unwrap();
        assert_eq!(results.len(), 2, "*parser* should match parser directory files");
    }
}
