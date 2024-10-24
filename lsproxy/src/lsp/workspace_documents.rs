use crate::utils::file_utils::search_files;
use log::{debug, error, warn};
use lsp_types::Range;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent};
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::read_to_string,
    sync::{
        broadcast::{channel, Receiver, Sender},
        RwLock,
    },
};

#[async_trait::async_trait]
pub trait WorkspaceDocuments: Send + Sync {
    async fn read_text_document(
        &self,
        full_file_path: &PathBuf,
        range: Option<Range>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn list_files(&self) -> Vec<PathBuf>;
    async fn update_patterns(&self, include_patterns: Vec<String>, exclude_patterns: Vec<String>);
}

pub struct WorkspaceDocumentsHandler {
    cache: Arc<RwLock<HashMap<PathBuf, Option<String>>>>,
    event_sender: Sender<DebouncedEvent>,
    patterns: Arc<RwLock<(Vec<String>, Vec<String>)>>,
    root_path: PathBuf,
}

impl WorkspaceDocumentsHandler {
    pub fn new(
        root_path: &Path,
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) -> Self {
        let (tx, mut rx) = channel(100);
        let event_sender = tx.clone();
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let patterns = Arc::new(RwLock::new((include_patterns, exclude_patterns)));
        let root_path = root_path.to_path_buf();

        // Initialize debouncer with a 2-second debounce duration
        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        let _ = tx.send(event.clone());
                    }
                }
                Err(e) => error!("Debounce error: {:?}", e),
            },
        )
        .expect("Failed to create debouncer");

        // Watch the root path recursively
        debouncer
            .watcher()
            .watch(&root_path, RecursiveMode::Recursive)
            .expect("Failed to watch path");

        let cache_clone = Arc::clone(&cache);
        let patterns_clone = Arc::clone(&patterns);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if WorkspaceDocumentsHandler::matches_patterns(&event.path, &patterns_clone).await {
                    cache_clone.write().await.clear();
                    debug!("Cache cleared for {:?}", event.path);
                }
            }
        });

        Self {
            cache,
            patterns,
            root_path,
            event_sender,
        }
    }

    async fn matches_patterns(
        path: &PathBuf,
        patterns: &Arc<RwLock<(Vec<String>, Vec<String>)>>,
    ) -> bool {
        let patterns_guard = patterns.read().await;
        let (include, exclude) = &*patterns_guard;
        let path_str = path.to_string_lossy();

        include
            .iter()
            .any(|pat| glob::Pattern::new(pat).unwrap().matches(&path_str))
            && !exclude
                .iter()
                .any(|pat| glob::Pattern::new(pat).unwrap().matches(&path_str))
    }

    async fn get_content(
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

    #[allow(unused)] // TODO: use this in client to notify servers
    pub fn subscribe_to_file_changes(&self) -> Receiver<DebouncedEvent> {
        self.event_sender.subscribe()
    }

    fn extract_range(content: &str, range: Range) -> Result<String, Box<dyn Error + Send + Sync>> {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Handle empty content case
        if total_lines == 0 {
            return Ok(String::new());
        }

        let start_line = range.start.line as usize;
        let mut end_line = range.end.line as usize;

        if end_line >= total_lines {
            warn!(
                "End line exceeds total lines: {} >= {}. Adjusting to include up to and including the last line.",
                end_line, total_lines
            );
            end_line = total_lines.saturating_sub(1);
        }

        // If start line is greater than end line, return empty string
        if start_line > end_line {
            warn!("Invalid range: start_line > end_line");
            return Ok(String::new());
        }

        let extracted: Vec<&str> = lines[start_line..=end_line]
            .iter()
            .enumerate()
            .map(|(i, &line)| {
                let line_len = line.chars().count();
                match (i, start_line == end_line) {
                    (0, true) => {
                        let start_char = range.start.character.min(line_len as u32) as usize;
                        let end_char = range.end.character.min(line_len as u32) as usize;
                        &line[..line_len].get(start_char..end_char).unwrap_or("")
                    }
                    (0, false) => {
                        let start_char = range.start.character.min(line_len as u32) as usize;
                        &line[..line_len].get(start_char..).unwrap_or("")
                    }
                    (n, _) if n == end_line - start_line => {
                        let end_char = range.end.character.min(line_len as u32) as usize;
                        &line[..line_len].get(..end_char).unwrap_or("")
                    }
                    _ => line,
                }
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
        let content = self.get_content(full_file_path).await?;
        match range {
            Some(range) => Self::extract_range(&content, range),
            None => Ok(content),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Range;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_text_document() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory and file
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, world!\nThis is a test.")?;

        // Initialize WorkspaceDocumentsHandler
        let handler = WorkspaceDocumentsHandler::new(dir.path(), vec!["*.txt".to_string()], vec![]);

        // Test reading the entire document
        let content = handler.read_text_document(&file_path, None).await?;
        assert_eq!(content, "Hello, world!\nThis is a test.");

        // Test reading a specific range
        let range = Range {
            start: lsp_types::Position {
                line: 0,
                character: 7,
            },
            end: lsp_types::Position {
                line: 0,
                character: 12,
            },
        };
        let extracted = handler.read_text_document(&file_path, Some(range)).await?;
        assert_eq!(extracted, "world");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_files() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory and files
        let dir = tempdir()?;
        fs::write(dir.path().join("file1.rs"), "fn main() {}")?;
        fs::write(dir.path().join("file2.txt"), "Hello")?;

        // Initialize WorkspaceDocumentsHandler with include and exclude patterns
        let handler = WorkspaceDocumentsHandler::new(
            dir.path(),
            vec!["*.rs".to_string()],
            vec!["file2.txt".to_string()],
        );

        // Test listing files based on patterns
        let files = handler.list_files().await;
        assert_eq!(files.len(), 1);
        assert!(files.contains(&dir.path().join("file1.rs")));

        fs::write(dir.path().join("file3.rs"), "fn main() {}")?;
        // Wait for the watcher to update the cache
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let files = handler.list_files().await;
        println!("Files: {:?}", files);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&dir.path().join("file1.rs")));
        assert!(files.contains(&dir.path().join("file3.rs")));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_patterns() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory and files
        let dir = tempdir()?;
        fs::write(dir.path().join("file1.rs"), "fn main() {}")?;
        fs::write(dir.path().join("file2.txt"), "Hello")?;

        // Initialize WorkspaceDocumentsHandler with initial patterns
        let handler = WorkspaceDocumentsHandler::new(dir.path(), vec!["*.txt".to_string()], vec![]);

        // Verify initial file listing
        let initial_files = handler.list_files().await;
        assert_eq!(initial_files.len(), 1);
        assert!(initial_files.contains(&dir.path().join("file2.txt")));

        // Update patterns to include Rust files
        handler
            .update_patterns(vec!["*.rs".to_string()], vec![])
            .await;

        // Verify updated file listing
        let updated_files = handler.list_files().await;
        assert_eq!(updated_files.len(), 1);
        assert!(updated_files.contains(&dir.path().join("file1.rs")));

        Ok(())
    }

    #[tokio::test]
    async fn test_read_text_document_out_of_bounds() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory and file
        let dir = tempdir()?;
        let file_path = dir.path().join("test_out_of_bounds.txt");
        fs::write(&file_path, "Line 1\nLine 2")?;

        // Initialize WorkspaceDocumentsHandler
        let handler = WorkspaceDocumentsHandler::new(dir.path(), vec!["*.txt".to_string()], vec![]);

        // Test reading with a range beyond the number of lines
        let range = Range {
            start: lsp_types::Position {
                line: 5,
                character: 0,
            },
            end: lsp_types::Position {
                line: 6,
                character: 10,
            },
        };
        let extracted = handler.read_text_document(&file_path, Some(range)).await?;
        assert_eq!(extracted, "");

        Ok(())
    }

    #[tokio::test]
    async fn test_read_text_document_invalid_characters() -> Result<(), Box<dyn Error + Send + Sync>>
    {
        // Setup temporary directory and file
        let dir = tempdir()?;
        let file_path = dir.path().join("test_invalid_chars.txt");
        fs::write(&file_path, "Short line")?;

        // Initialize WorkspaceDocumentsHandler
        let handler = WorkspaceDocumentsHandler::new(dir.path(), vec!["*.txt".to_string()], vec![]);

        // Test reading with character positions exceeding line length
        let range = Range {
            start: lsp_types::Position {
                line: 0,
                character: 100,
            },
            end: lsp_types::Position {
                line: 0,
                character: 200,
            },
        };
        let extracted = handler.read_text_document(&file_path, Some(range)).await?;
        assert_eq!(extracted, "");

        Ok(())
    }

    #[tokio::test]
    async fn test_read_text_document_empty_file() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory and empty file
        let dir = tempdir()?;
        let file_path = dir.path().join("empty.txt");
        fs::write(&file_path, "")?;

        // Initialize WorkspaceDocumentsHandler
        let handler = WorkspaceDocumentsHandler::new(dir.path(), vec!["*.txt".to_string()], vec![]);

        // Test reading the entire empty document
        let content = handler.read_text_document(&file_path, None).await?;
        assert_eq!(content, "");

        // Test reading with any range on empty file
        let range = Range {
            start: lsp_types::Position {
                line: 0,
                character: 0,
            },
            end: lsp_types::Position {
                line: 0,
                character: 10,
            },
        };
        let extracted = handler.read_text_document(&file_path, Some(range)).await?;
        assert_eq!(extracted, "");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_files_no_matching_files() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory without matching files
        let dir = tempdir()?;
        fs::write(dir.path().join("file1.rs"), "fn main() {}")?;

        // Initialize WorkspaceDocumentsHandler with patterns that do not match
        let handler = WorkspaceDocumentsHandler::new(
            dir.path(),
            vec!["*.txt".to_string()],
            vec!["*.md".to_string()],
        );

        // Test listing files with no matches
        let files = handler.list_files().await;
        assert!(files.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_patterns_empty_include_exclude() -> Result<(), Box<dyn Error + Send + Sync>>
    {
        // Setup temporary directory and files
        let dir = tempdir()?;
        fs::write(dir.path().join("file1.rs"), "fn main() {}")?;
        fs::write(dir.path().join("file2.txt"), "Hello")?;

        // Initialize WorkspaceDocumentsHandler with initial patterns
        let handler = WorkspaceDocumentsHandler::new(
            dir.path(),
            vec!["*.rs".to_string()],
            vec!["file2.txt".to_string()],
        );

        // Update patterns with empty include and exclude
        handler.update_patterns(vec![], vec![]).await;

        // Test listing files after updating patterns
        let files = handler.list_files().await;
        // Assuming empty include patterns match nothing
        assert!(files.is_empty());

        Ok(())
    }
}
