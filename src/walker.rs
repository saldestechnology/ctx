use std::io;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;

use crate::default_ignores::DEFAULT_IGNORES;

/// Represents a discovered file with its metadata.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub absolute_path: PathBuf,
    pub relative_path: PathBuf,
    pub size: u64,
}

/// Configuration for the file walker.
pub struct WalkerConfig {
    pub use_gitignore: bool,
    pub use_default_ignores: bool,
    pub custom_ignores: Vec<String>,
    pub include_patterns: Vec<String>,
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            use_default_ignores: true,
            custom_ignores: Vec::new(),
            include_patterns: Vec::new(),
        }
    }
}

/// Check if a pattern is a glob pattern (contains wildcards).
fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

/// Build a GlobSet from include patterns.
fn build_include_globset(patterns: &[String]) -> Result<Option<GlobSet>, globset::Error> {
    let glob_patterns: Vec<&String> = patterns.iter().filter(|p| is_glob_pattern(p)).collect();

    if glob_patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in glob_patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(Some(builder.build()?))
}

/// Get literal paths from patterns (non-glob patterns).
fn get_literal_paths(patterns: &[String]) -> Vec<PathBuf> {
    patterns
        .iter()
        .filter(|p| !is_glob_pattern(p))
        .map(PathBuf::from)
        .collect()
}

/// Walk directories and discover files based on configuration.
pub fn discover_files(root: &Path, config: &WalkerConfig) -> io::Result<Vec<FileEntry>> {
    let root = root.canonicalize()?;
    let mut entries = Vec::new();

    // Determine starting paths
    let starting_paths = if config.include_patterns.is_empty() {
        vec![root.clone()]
    } else {
        let literal_paths = get_literal_paths(&config.include_patterns);
        if literal_paths.is_empty() {
            vec![root.clone()]
        } else {
            literal_paths
                .into_iter()
                .map(|p| if p.is_absolute() { p } else { root.join(p) })
                .collect()
        }
    };

    // Build include glob set for filtering
    let include_globset = build_include_globset(&config.include_patterns)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Process each starting path
    for start_path in &starting_paths {
        if !start_path.exists() {
            eprintln!("Warning: path does not exist: {}", start_path.display());
            continue;
        }

        let mut builder = WalkBuilder::new(start_path);

        // Configure gitignore handling
        builder.git_ignore(config.use_gitignore);
        builder.git_global(config.use_gitignore);
        builder.git_exclude(config.use_gitignore);

        // Look for .contextignore file
        builder.add_custom_ignore_filename(".contextignore");

        // Build overrides for custom ignores and default ignores
        let mut override_builder = OverrideBuilder::new(&root);

        // Add default ignores (as negative patterns)
        if config.use_default_ignores {
            for pattern in DEFAULT_IGNORES {
                // Convert to negation pattern for ignore crate
                let neg_pattern = format!("!{}", pattern);
                if override_builder.add(&neg_pattern).is_err() {
                    // Try alternative format
                    let alt_pattern = format!("!**/{}", pattern.trim_end_matches('/'));
                    let _ = override_builder.add(&alt_pattern);
                }
            }
        }

        // Add custom ignores
        for pattern in &config.custom_ignores {
            let neg_pattern = format!("!{}", pattern);
            if override_builder.add(&neg_pattern).is_err() {
                let alt_pattern = format!("!**/{}", pattern);
                let _ = override_builder.add(&alt_pattern);
            }
        }

        if let Ok(overrides) = override_builder.build() {
            builder.overrides(overrides);
        }

        // Walk the directory
        for result in builder.build() {
            let entry = match result {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Warning: {}", err);
                    continue;
                }
            };

            // Skip directories
            let file_type = match entry.file_type() {
                Some(ft) => ft,
                None => continue,
            };
            if file_type.is_dir() {
                continue;
            }

            let abs_path = entry.path().to_path_buf();

            // Calculate relative path from root
            let rel_path = match abs_path.strip_prefix(&root) {
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            };

            // Apply include glob filter if present
            if let Some(ref globset) = include_globset {
                if !globset.is_match(&rel_path) {
                    continue;
                }
            }

            // Skip binary files (check for null bytes)
            if is_binary_file(&abs_path) {
                continue;
            }

            // Get file size
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            entries.push(FileEntry {
                absolute_path: abs_path,
                relative_path: rel_path,
                size,
            });
        }
    }

    // Sort by relative path for consistent output
    entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    // Remove duplicates (can happen with overlapping patterns)
    entries.dedup_by(|a, b| a.absolute_path == b.absolute_path);

    Ok(entries)
}

/// Check if a file is likely binary by looking for null bytes.
fn is_binary_file(path: &Path) -> bool {
    use std::fs::File;
    use std::io::Read;

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut buffer = [0u8; 8192];
    let bytes_read = match file.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return false,
    };

    buffer[..bytes_read].contains(&0)
}

/// Format file size in human-readable format.
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}
