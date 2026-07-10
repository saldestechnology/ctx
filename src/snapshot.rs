//! Per-commit metric snapshots (`ctx snapshot`).
//!
//! Exports one Parquet partition per commit (`.ctx/snapshots/sha=<sha>/`)
//! with per-file and per-symbol metrics, near-duplicate pairs, and metadata,
//! for longitudinal quality analysis via `ctx sql --snapshots`.
