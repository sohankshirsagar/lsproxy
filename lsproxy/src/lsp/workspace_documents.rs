use std::{collections::HashMap, error::Error, path::Path, sync::Arc};
use tokio::sync::Mutex; // Use Tokio's async Mutex

use lsp_types::Range;
use tokio::fs::read_to_string;

use crate::utils::file_utils::search_files;

#[async_trait::async_trait]
pub trait WorkspaceDocuments: Send + Sync {
    async fn read_text_document(
        &mut self,
        full_file_path: &str,
        range: Option<Range>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;

    #[allow(unused)] // TODO handle syncronization
    async fn invalidate_cache(&self, full_file_path: &str);

    async fn list_files(&mut self) -> Vec<String>;

    async fn update_patterns(
        &mut self,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    );
}

pub struct WorkspaceDocumentsHandler {
    cache: Arc<Mutex<HashMap<String, Option<String>>>>,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    root_path: String,
}

impl WorkspaceDocumentsHandler {
    pub fn new(
        root_path: &str,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            include_patterns,
            exclude_patterns,
            root_path: root_path.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl WorkspaceDocuments for WorkspaceDocumentsHandler {
    async fn read_text_document(
        &mut self,
        full_file_path: &str,
        range: Option<Range>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut cache = self.cache.lock().await;
        let file_content = match cache.get(full_file_path) {
            Some(entry) => match entry {
                Some(existing_content) => existing_content.clone(),
                None => {
                    let content = read_to_string(full_file_path).await?;
                    cache.insert(full_file_path.to_string(), Some(content.clone()));
                    content
                }
            },
            None => {
                let content = read_to_string(full_file_path).await?;
                cache.insert(full_file_path.to_string(), Some(content.clone()));
                content
            }
        };
        match range {
            Some(range) => {
                let start_line = range.start.line as usize;
                let end_line = range.end.line as usize;
                let start_character = range.start.character as usize;
                let end_character = range.end.character as usize;
                let lines: Vec<&str> = file_content.split('\n').collect();
                Ok(lines[start_line..=end_line]
                    .iter()
                    .enumerate()
                    .map(|(i, &line)| {
                        if i == 0 && i == end_line - start_line {
                            &line[start_character..end_character]
                        } else if i == 0 {
                            &line[start_character..]
                        } else if i == end_line - start_line {
                            &line[..end_character]
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<&str>>()
                    .join("\n"))
            }
            None => Ok(file_content.clone()),
        }
    }

    async fn invalidate_cache(&self, full_file_path: &str) {
        let mut cache = self.cache.lock().await;
        cache.insert(full_file_path.to_string(), None);
    }

    async fn list_files(&mut self) -> Vec<String> {
        let mut cache = self.cache.lock().await;
        if cache.is_empty() {
            let file_paths = search_files(
                &Path::new(&self.root_path),
                self.include_patterns.clone(),
                self.exclude_patterns.clone(),
            )
            .unwrap();
            for file_path in file_paths {
                cache.insert(file_path.to_string_lossy().into_owned(), None);
            }
        }
        cache.keys().cloned().collect()
    }

    async fn update_patterns(
        &mut self,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) {
        self.include_patterns = include_patterns;
        self.exclude_patterns = exclude_patterns;
        // bust cache
        self.cache.lock().await.clear();
    }
}
