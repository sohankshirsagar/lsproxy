use glob::glob;
use lsp_types::TextDocumentItem;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use url::Url;
pub async fn get_files_for_workspace(
    repo_path: &str,
) -> Result<Vec<TextDocumentItem>, Box<dyn std::error::Error>> {
    let tsconfig_path = Path::new(repo_path).join("tsconfig.json");
    let tsconfig_content = fs::read_to_string(tsconfig_path).unwrap_or_else(|_| "{}".to_string());
    let tsconfig: Value = serde_json::from_str(&tsconfig_content)?;

    let mut included = Vec::new();
    let include_patterns = tsconfig["include"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"]);
    let exclude_patterns = tsconfig["exclude"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["**/node_modules/**", "**/dist/**", "**/build/**", ".git/**"]);

    let files = get_typescript_files(repo_path, &include_patterns, &exclude_patterns)?;

    for file_path in files {
        let content = fs::read_to_string(&file_path)?;
        included.push(TextDocumentItem {
            uri: Url::from_file_path(&file_path).unwrap(),
            language_id: "typescript".to_string(),
            version: 1,
            text: content,
        });
    }

    Ok(included)
}

fn get_typescript_files(
    repo_path: &str,
    include_patterns: &[&str],
    exclude_patterns: &[&str],
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    for pattern in include_patterns {
        let glob_pattern = format!("{}/{}", repo_path, pattern);
        for entry in glob(&glob_pattern)? {
            let path = entry?;
            if !is_excluded(&path, repo_path, exclude_patterns) && path.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}

fn is_excluded(path: &Path, repo_path: &str, exclude_patterns: &[&str]) -> bool {
    let relative_path = path.strip_prefix(repo_path).unwrap_or(path);
    exclude_patterns.iter().any(|pattern| {
        let glob_pattern = format!("{}/{}", repo_path, pattern);
        glob::Pattern::new(&glob_pattern)
            .map(|p| p.matches_path(relative_path))
            .unwrap_or(false)
    })
}
