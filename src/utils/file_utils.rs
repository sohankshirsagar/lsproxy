use glob::glob;
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

    let files = get_typescript_files(repo_path, &include_patterns, &exclude_patterns)?;

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

fn get_typescript_files(
    repo_path: &str,
    include_patterns: &[&str],
    exclude_patterns: &[&str],
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    include_patterns
        .iter()
        .try_fold(Vec::new(), |mut acc, pattern| {
            let paths = glob(&format!("{}/{}", repo_path, pattern))?
                .filter_map(Result::ok)
                .filter(|path| !is_excluded(path, repo_path, exclude_patterns) && path.is_file());
            acc.extend(paths);
            Ok(acc)
        })
}

fn is_excluded(path: &Path, repo_path: &str, exclude_patterns: &[&str]) -> bool {
    let relative_path = path.strip_prefix(repo_path).unwrap_or(path);
    exclude_patterns.iter().any(|pattern| {
        glob::Pattern::new(&format!("{}/{}", repo_path, pattern))
            .map(|p| p.matches_path(relative_path))
            .unwrap_or(false)
    })
}

pub fn is_hidden(entry: &std::fs::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

pub fn search_directories(path: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && !is_hidden(&entry) {
            dirs.push(path.clone());
            dirs.extend(search_directories(&path)?);
        }
    }
    Ok(dirs)
}
