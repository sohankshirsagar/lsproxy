use ignore::WalkBuilder;
use log::debug;
use std::path::{Path, PathBuf};

pub fn search_files(
    path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let walk = build_walk(path, exclude_patterns);

    for result in walk {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if !include_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches_path(&path))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                if path.is_file() {
                    files.push(path.to_path_buf());
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }

    Ok(files)
}

pub fn search_directories(
    root_path: &std::path::Path,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let walk = build_walk(root_path, exclude_patterns);
    for result in walk {
        match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                if !include_patterns.iter().any(|pattern| {
                    glob::Pattern::new(pattern)
                        .map(|p| p.matches_path(&path))
                        .unwrap_or(false)
                }) {
                    continue;
                }
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    dirs.push(path.parent().unwrap().to_path_buf());
                }
            }
            Err(err) => eprintln!("Error: {}", err),
        }
    }
    debug!("dirs: {:?}", dirs);
    Ok(dirs)
}

fn build_walk(path: &Path, exclude_patterns: Vec<String>) -> ignore::Walk {
    let walk = WalkBuilder::new(path)
        .filter_entry(move |entry| {
            let path = entry.path();
            debug!("Checking path: {:?}", path);

            let is_excluded = exclude_patterns.iter().any(|pattern| {
                let matches = glob::Pattern::new(pattern)
                    .map(|p| p.matches_path(path))
                    .unwrap_or(false);
                if matches {
                    debug!("Excluded: {:?} matches pattern {:?}", path, pattern);
                }
                matches
            });
            !is_excluded
        })
        .build();
    walk
}
