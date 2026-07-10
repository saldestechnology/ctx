//! Gate-evaluation logging.
//!
//! When the `CTX_GATE_LOG` environment variable is set, `ctx score` appends
//! one JSON line per gate evaluation to a local log file (default
//! `.ctx/gate-log.jsonl`). Opt-in, local-only; ctx ships no telemetry.
