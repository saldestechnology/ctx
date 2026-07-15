//! End-to-end CLI tests for the LSP extraction backend.
//!
//! The ctx test binary doubles as a scripted mock language server: when
//! `CTX_INTERNAL_MOCK_LSP` points at a scenario JSON file it speaks
//! Content-Length framed JSON-RPC over stdio instead of running a command
//! (see `src/lsp/mock.rs`). Each test registers that mock in
//! `.ctx/config.toml` and drives a real `ctx index` run.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn ctx(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ctx"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run ctx binary")
}

fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Write a `[lsp.<language>]` block that spawns the ctx binary in mock mode.
fn write_mock_lsp_config(
    root: &Path,
    language: &str,
    backend: &str,
    extensions: &str,
    scenario: &Path,
) {
    // TOML literal strings ('...') keep Windows backslashes intact.
    let config = format!(
        r#"
[lsp.{language}]
command = '{command}'
{extensions}
backend = "{backend}"
env = {{ CTX_INTERNAL_MOCK_LSP = '{scenario}' }}
"#,
        command = env!("CARGO_BIN_EXE_ctx"),
        scenario = scenario.display(),
    );
    write(root, ".ctx/config.toml", &config);
}

const KOTLIN_SOURCE: &str = "class Greeter {\n    fun greet(who: String): String {\n        return \"hi \" + who\n    }\n}\nfun topLevel() {}\n";

/// documentSymbol scenario response matching `KOTLIN_SOURCE`.
fn kotlin_document_symbols() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "Greeter",
            "kind": 5,
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 4, "character": 1 } },
            "selectionRange": { "start": { "line": 0, "character": 6 }, "end": { "line": 0, "character": 13 } },
            "children": [
                {
                    "name": "greet",
                    "detail": "fun greet(who: String): String",
                    "kind": 6,
                    "range": { "start": { "line": 1, "character": 4 }, "end": { "line": 3, "character": 5 } },
                    "selectionRange": { "start": { "line": 1, "character": 8 }, "end": { "line": 1, "character": 13 } }
                }
            ]
        },
        {
            "name": "topLevel",
            "kind": 12,
            "range": { "start": { "line": 5, "character": 0 }, "end": { "line": 5, "character": 18 } },
            "selectionRange": { "start": { "line": 5, "character": 4 }, "end": { "line": 5, "character": 12 } }
        }
    ])
}

/// Scratch area outside the indexed repo for scenario + hits files.
fn scenario_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn dynamic_language_indexes_via_mock_server() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let scratch = scenario_dir();
    let scenario_path = scratch.path().join("scenario.json");
    let hits_path = scratch.path().join("hits.log");

    write(root, "src/main.kt", KOTLIN_SOURCE);
    fs::write(
        &scenario_path,
        serde_json::json!({
            "server_name": "ctx-mock-kotlin",
            "hits_file": hits_path,
            "document_symbols": { "src/main.kt": kotlin_document_symbols() },
        })
        .to_string(),
    )
    .unwrap();
    write_mock_lsp_config(
        root,
        "kotlin",
        "lsp",
        "extensions = [\"kt\"]",
        &scenario_path,
    );

    // Exit code 0 and the kotlin file is indexed.
    let out = ctx(root, &["index"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The mock server was actually spoken to.
    let hits = fs::read_to_string(&hits_path).unwrap_or_default();
    assert!(hits.contains("initialize"), "hits: {hits}");
    assert!(hits.contains("textDocument/documentSymbol"), "hits: {hits}");

    // `ctx query files` lists the dynamic-language file.
    let out = ctx(root, &["query", "files"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("src/main.kt"), "stdout: {stdout}");

    // `ctx query find` returns the LSP-extracted symbols.
    let out = ctx(root, &["query", "find", "greet"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("greet"), "stdout: {stdout}");
    assert!(stdout.contains("main.kt"), "stdout: {stdout}");

    // The stored file record carries the dynamic language name (files.language
    // is free-form TEXT, so kotlin flows through `ctx sql` unchanged).
    #[cfg(feature = "duckdb")]
    {
        let out = ctx(
            root,
            &[
                "sql",
                "--json",
                "SELECT language FROM v1.files WHERE path = 'src/main.kt'",
            ],
        );
        assert_eq!(out.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("kotlin"), "stdout: {stdout}");
    }

    // Library-level check: the LSP symbols are real index rows.
    let db = ctx::index::open_database(root).unwrap();
    let symbols = db.find_symbols("greet", 10).unwrap();
    assert!(
        symbols
            .iter()
            .any(|s| s.file_path == "src/main.kt" && s.name == "greet"),
        "symbols: {symbols:?}"
    );

    // The status sidecar reports the healthy server, and stays out of the
    // index itself.
    let status = fs::read_to_string(root.join(".ctx/lsp_status.json")).unwrap();
    let status: serde_json::Value = serde_json::from_str(&status).unwrap();
    assert_eq!(status["servers"][0]["language"], "kotlin");
    assert_eq!(status["servers"][0]["state"], "healthy");
    assert_eq!(status["servers"][0]["server_name"], "ctx-mock-kotlin");
    let out = ctx(root, &["query", "files"]);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("lsp_status.json"));
}

#[test]
fn hybrid_backend_resolves_cross_file_call_via_definition() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let scratch = scenario_dir();
    let scenario_path = scratch.path().join("scenario.json");

    // `helper` is defined in two files, so the SQL resolution passes leave
    // the bare-name call from app.py unresolved (ambiguous). The mock's
    // definition response points at util.py.
    write(root, "app.py", "def main():\n    return helper()\n");
    write(root, "util.py", "def helper():\n    return 1\n");
    write(root, "other.py", "def helper():\n    return 2\n");

    fs::write(
        &scenario_path,
        serde_json::json!({
            // Wildcard position match: any definition request in app.py
            // resolves to util.py's `helper` (0-based line 0).
            "definitions": {
                "app.py": { "path": "util.py", "line": 0, "character": 4 }
            },
        })
        .to_string(),
    )
    .unwrap();
    write_mock_lsp_config(root, "python", "hybrid", "", &scenario_path);

    let out = ctx(root, &["index"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The ambiguous call is now resolved to util.py's helper.
    let db = ctx::index::open_database(root).unwrap();
    let call_edge = db
        .get_incoming_edges("helper")
        .unwrap()
        .into_iter()
        .find(|e| e.kind == ctx::db::EdgeKind::Calls)
        .expect("expected a calls edge for helper");
    let target_id = call_edge
        .target_id
        .as_deref()
        .expect("the ambiguous cross-file call must be LSP-resolved, not NULL");
    assert!(
        target_id.starts_with("util.py::helper"),
        "expected util.py's helper, got {target_id}"
    );

    // Same result through the public SQL surface.
    #[cfg(feature = "duckdb")]
    {
        let out = ctx(
            root,
            &[
                "sql",
                "--json",
                "SELECT target_id FROM v1.edges WHERE target_name = 'helper' AND kind = 'calls'",
            ],
        );
        assert_eq!(out.status.code(), Some(0));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("util.py::helper"),
            "expected the edge to resolve to util.py::helper: {stdout}"
        );
    }

    // Tree-sitter still did the extraction (hybrid): symbols exist for all files.
    let out = ctx(root, &["query", "find", "helper"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("util.py"), "stdout: {stdout}");
    assert!(stdout.contains("other.py"), "stdout: {stdout}");
}

#[test]
fn missing_server_binary_falls_back_to_tree_sitter() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    write(
        root,
        "src/app.py",
        "def compute_total(x):\n    return x + 1\n",
    );
    // No scenario needed: the command never spawns.
    write(
        root,
        ".ctx/config.toml",
        r#"
[lsp.python]
command = "definitely-not-on-path-xyz"
backend = "lsp"
"#,
    );

    let out = ctx(root, &["index"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "LSP failures never break indexing"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("falling back"), "stderr: {stderr}");
    assert!(
        stderr.contains("definitely-not-on-path-xyz"),
        "stderr: {stderr}"
    );

    // Tree-sitter extracted the symbols instead.
    let out = ctx(root, &["query", "find", "compute_total"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("compute_total"), "stdout: {stdout}");

    // Status sidecar records the failure.
    let status = fs::read_to_string(root.join(".ctx/lsp_status.json")).unwrap();
    assert!(status.contains("\"state\": \"failed\""), "status: {status}");
}

#[test]
fn server_crash_after_initialize_falls_back_gracefully() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let scratch = scenario_dir();
    let scenario_path = scratch.path().join("scenario.json");

    write(
        root,
        "src/app.py",
        "def compute_total(x):\n    return x + 1\n",
    );
    fs::write(
        &scenario_path,
        serde_json::json!({ "exit_after_initialize": true }).to_string(),
    )
    .unwrap();
    write_mock_lsp_config(root, "python", "lsp", "", &scenario_path);

    let out = ctx(root, &["index"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("falling back"), "stderr: {stderr}");

    // Fallback produced tree-sitter symbols.
    let out = ctx(root, &["query", "find", "compute_total"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("compute_total"), "stdout: {stdout}");
}

#[test]
fn incremental_reindex_spawns_no_server() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let scratch = scenario_dir();
    let scenario_path = scratch.path().join("scenario.json");
    let hits_path = scratch.path().join("hits.log");

    write(root, "src/main.kt", KOTLIN_SOURCE);
    fs::write(
        &scenario_path,
        serde_json::json!({
            "hits_file": hits_path,
            "document_symbols": { "src/main.kt": kotlin_document_symbols() },
        })
        .to_string(),
    )
    .unwrap();
    write_mock_lsp_config(
        root,
        "kotlin",
        "lsp",
        "extensions = [\"kt\"]",
        &scenario_path,
    );

    // First run talks to the server.
    let out = ctx(root, &["index"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(fs::read_to_string(&hits_path)
        .unwrap_or_default()
        .contains("initialize"));

    // Second run: nothing changed, so no file is reindexed and the server is
    // never spawned again.
    fs::remove_file(&hits_path).unwrap();
    let out = ctx(root, &["index"]);
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Indexed 0 files"), "stderr: {stderr}");
    assert!(
        !hits_path.exists() || fs::read_to_string(&hits_path).unwrap().is_empty(),
        "no LSP traffic expected on an incremental no-op run"
    );

    // Editing the file re-runs Stage A for exactly that file.
    write(
        root,
        "src/main.kt",
        &KOTLIN_SOURCE.replace("topLevel", "topLevelV2"),
    );
    let out = ctx(root, &["index"]);
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Indexed 1 files"), "stderr: {stderr}");
    assert!(fs::read_to_string(&hits_path)
        .unwrap_or_default()
        .contains("textDocument/documentSymbol"));
}

/// Optional smoke test against a real language server. Run explicitly with
/// `cargo test --test lsp_cli -- --ignored` on a machine with
/// `pyright-langserver` installed; skips silently when the binary is absent.
#[test]
#[ignore = "requires pyright-langserver on PATH"]
fn real_pyright_extracts_python_symbols() {
    let has_pyright = Command::new("pyright-langserver")
        .arg("--version")
        .output()
        .is_ok();
    if !has_pyright {
        eprintln!("pyright-langserver not found; skipping");
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "app.py",
        "class Greeter:\n    def greet(self, who):\n        return f\"hi {who}\"\n",
    );
    write(
        root,
        ".ctx/config.toml",
        r#"
[lsp.python]
command = "pyright-langserver"
args = ["--stdio"]
backend = "lsp"
"#,
    );

    let out = ctx(root, &["index"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = ctx(root, &["query", "find", "greet"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("greet"), "stdout: {stdout}");
}
