use log::error;
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
        drop(cache); // Release the lock early

        match range {
            Some(range) => {
                let lines: Vec<&str> = file_content.split('\n').collect();
                let total_lines = lines.len();

                let start_line = range.start.line as usize;
                let end_line = range.end.line as usize;

                if start_line >= total_lines || end_line >= total_lines {
                    error!(
                        "Range out of bounds: start_line={}, end_line={}, total_lines={}",
                        start_line, end_line, total_lines
                    );
                    return Err("Specified range is out of bounds".into());
                }

                let start_character = range.start.character as usize;
                let end_character = range.end.character as usize;

                let selected_lines = &lines[start_line..=end_line];
                let mut extracted = Vec::with_capacity(selected_lines.len());

                for (i, line) in selected_lines.iter().enumerate() {
                    let current_line_number = start_line + i;
                    let line_length = line.len();

                    let extracted_line = if i == 0 && start_character < line_length {
                        &line[start_character..]
                    } else if i == selected_lines.len() - 1 && end_character <= line_length {
                        &line[..end_character]
                    } else {
                        line
                    };

                    if extracted_line.len() < end_character {
                        error!(
                            "Character range out of bounds: line={}, end_character={}, line_length={}",
                            current_line_number, end_character, line_length
                        );
                        return Err("Specified character range is out of bounds".into());
                    }

                    extracted.push(extracted_line);
                }

                let result = extracted.join("\n");
                Ok(result)
            }
            None => Ok(file_content),
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
            .unwrap_or_else(|err| {
                error!("Error searching files: {}", err);
                Vec::new()
            });
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
