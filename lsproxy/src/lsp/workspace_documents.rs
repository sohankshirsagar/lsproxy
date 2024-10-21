use crate::utils::file_utils::search_files;
use log::{debug, error, warn};
use lsp_types::Range;
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs::read_to_string;
use tokio::sync::RwLock;

#[async_trait::async_trait]
pub trait WorkspaceDocuments: Send + Sync {
    async fn read_text_document(
        &self,
        full_file_path: &PathBuf,
        range: Option<Range>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn invalidate_cache(&self, full_file_path: &PathBuf);
    async fn list_files(&self) -> Vec<PathBuf>;
    async fn update_patterns(&self, include_patterns: Vec<String>, exclude_patterns: Vec<String>);
}

pub struct WorkspaceDocumentsHandler {
    cache: Arc<RwLock<HashMap<PathBuf, Option<String>>>>,
    patterns: Arc<RwLock<(Vec<String>, Vec<String>)>>,
    root_path: PathBuf,
}

impl WorkspaceDocumentsHandler {
    pub fn new(
        root_path: &Path,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            patterns: Arc::new(RwLock::new((include_patterns, exclude_patterns))),
            root_path: root_path.to_path_buf(),
        }
    }

    async fn get_or_insert_content(
        &self,
        full_file_path: &PathBuf,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut cache = self.cache.write().await;
        match cache.get(full_file_path) {
            Some(Some(content)) => Ok(content.clone()),
            _ => {
                debug!("Cache miss for {:?}", full_file_path);
                let content = read_to_string(full_file_path).await?;
                cache.insert(full_file_path.clone(), Some(content.clone()));
                Ok(content)
            }
        }
    }

    fn extract_range(content: &str, range: Range) -> Result<String, Box<dyn Error + Send + Sync>> {
        let lines: Vec<&str> = content.split('\n').collect();
        let total_lines = lines.len();

        if range.start.line as usize >= total_lines {
            warn!(
                "Start line out of bounds: {}  ({} total lines)",
                range.start.line, total_lines
            );
            return Ok(String::new());
        }

        let start_line = range.start.line as usize;
        let end_line = (range.end.line as usize).min(total_lines - 1);

        if start_line > end_line {
            warn!(
                "Start line is greater than end line: {} > {}",
                start_line, end_line
            );
            return Ok(String::new());
        }

        let start_char = range.start.character as usize;
        let end_char = range.end.character as usize;

        let extracted: Vec<&str> = lines[start_line..=end_line]
            .iter()
            .enumerate()
            .map(|(i, &line)| match (i, start_line == end_line) {
                (0, true) => &line[start_char.min(line.len())..end_char.min(line.len())],
                (0, false) => &line[start_char.min(line.len())..],
                (n, _) if n == end_line - start_line => &line[..end_char.min(line.len())],
                _ => line,
            })
            .collect();

        debug!("Extracted range lines: {:?}", extracted);
        Ok(extracted.join("\n"))
    }
}

#[async_trait::async_trait]
impl WorkspaceDocuments for WorkspaceDocumentsHandler {
    async fn read_text_document(
        &self,
        full_file_path: &PathBuf,
        range: Option<Range>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let content = self.get_or_insert_content(full_file_path).await?;
        match range {
            Some(range) => Self::extract_range(&content, range),
            None => Ok(content),
        }
    }

    async fn invalidate_cache(&self, full_file_path: &PathBuf) {
        self.cache.write().await.remove(full_file_path);
    }

    async fn list_files(&self) -> Vec<PathBuf> {
        let cache_read = self.cache.read().await;
        if cache_read.is_empty() {
            drop(cache_read);
            let (include_patterns, exclude_patterns) = self.patterns.read().await.clone();
            let file_paths = search_files(&self.root_path, include_patterns, exclude_patterns)
                .unwrap_or_else(|err| {
                    error!("Error searching files: {}", err);
                    Vec::new()
                });
            let mut cache_write = self.cache.write().await;
            for file_path in file_paths {
                cache_write.insert(file_path, None);
            }
            cache_write.keys().cloned().collect()
        } else {
            cache_read.keys().cloned().collect()
        }
    }

    async fn update_patterns(&self, include_patterns: Vec<String>, exclude_patterns: Vec<String>) {
        *self.patterns.write().await = (include_patterns, exclude_patterns);
        self.cache.write().await.clear();
    }
}
