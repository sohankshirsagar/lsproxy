use ignore::WalkBuilder;
use log::debug;
use lsp_types::TextDocumentItem;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;

pub async fn get_files_for_workspace_typescript(
    repo_path: &str,
) -> Result<Vec<TextDocumentItem>, Box<dyn std::error::Error>> {
    let tsconfig_path = Path::new(repo_path).join("tsconfig.json");
    let tsconfig_content = fs::read_to_string(tsconfig_path).unwrap_or_else(|_| "{}".to_string());
    let tsconfig: Value = serde_json::from_str(&tsconfig_content)?;

    let include_patterns = tsconfig["include"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"]);
    let exclude_patterns = tsconfig["exclude"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["**/node_modules/**", "**/dist/**", "**/build/**", ".git/**"]);

    let files = search_files(
        Path::new(repo_path),
        include_patterns.into_iter().map(String::from).collect(),
        exclude_patterns.into_iter().map(String::from).collect(),
    )?;

    files
        .into_iter()
        .map(|file_path| {
            let content = fs::read_to_string(&file_path)?;
            Ok(TextDocumentItem {
                uri: Url::from_file_path(&file_path).map_err(|_| "Invalid file path")?,
                language_id: "typescript".to_string(),
                version: 1,
                text: content,
            })
        })
        .collect()
}

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
