use crate::utils::file_utils::search_files;
use log::{debug, error, warn};
use lsp_types::Range;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
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
    event_sender: Sender<Event>,
    patterns: Arc<RwLock<(Vec<String>, Vec<String>)>>,
    root_path: PathBuf,
    watcher: RecommendedWatcher,
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

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )
        .unwrap();

        watcher
            .watch(&root_path, RecursiveMode::Recursive)
            .unwrap_or_else(|e| error!("Failed to watch {:?}: {:?}", root_path, e));

        let cache_clone = Arc::clone(&cache);
        let patterns_clone = Arc::clone(&patterns);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                debug!("Received event: {:?}", event);
                for path in event.paths {
                    if WorkspaceDocumentsHandler::matches_patterns(&path, &patterns_clone).await {
                        cache_clone.write().await.clear();
                        debug!("Cache cleared for {:?}", path);
                    }
                }
            }
        });

        Self {
            cache,
            patterns,
            root_path,
            event_sender,
            watcher,
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

    pub fn subscribe_to_file_changes(&self) -> Receiver<Event> {
        self.event_sender.subscribe()
    }

    fn extract_range(content: &str, range: Range) -> Result<String, Box<dyn Error + Send + Sync>> {
        let lines: Vec<&str> = content.lines().collect();
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
                (0, true) => {
                    let line_start = start_char.min(line.len());
                    let line_end = end_char.min(line.len());
                    if line_start != start_char || line_end != end_char {
                        warn!(
                            "Adjusted range for single-line extraction: {}..{} to {}..{} on line {}",
                            start_char, end_char, line_start, line_end, i + start_line
                        );
                    }
                    &line[line_start..line_end]
                }
                (0, false) => {
                    let line_start = start_char.min(line.len());
                    if line_start != start_char {
                        warn!(
                            "Adjusted start character: {} to {} on line {}",
                            start_char, line_start, i + start_line
                        );
                    }
                    &line[line_start..]
                }
                (n, _) if n == end_line - start_line => {
                    let line_end = end_char.min(line.len());
                    if line_end != end_char {
                        warn!(
                            "Adjusted end character: {} to {} on line {}",
                            end_char, line_end, i + start_line
                        );
                    }
                    &line[..line_end]
                }
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
    use std::fs;
    use tempfile::tempdir;
    use lsp_types::Range;

    #[tokio::test]
    async fn test_read_text_document() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Setup temporary directory and file
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, world!\nThis is a test.")?;

        // Initialize WorkspaceDocumentsHandler
        let handler = WorkspaceDocumentsHandler::new(
            dir.path(),
            vec!["*.txt".to_string()],
            vec![],
        );

        // Test reading the entire document
        let content = handler.read_text_document(&file_path, None).await?;
        assert_eq!(content, "Hello, world!\nThis is a test.");

        // Test reading a specific range
        let range = Range {
            start: lsp_types::Position { line: 0, character: 7 },
            end: lsp_types::Position { line: 0, character: 12 },
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
        let handler = WorkspaceDocumentsHandler::new(
            dir.path(),
            vec!["*.txt".to_string()],
            vec![],
        );

        // Verify initial file listing
        let initial_files = handler.list_files().await;
        assert_eq!(initial_files.len(), 1);
        assert!(initial_files.contains(&dir.path().join("file2.txt")));

        // Update patterns to include Rust files
        handler.update_patterns(
            vec!["*.rs".to_string()],
            vec![],
        ).await;

        // Verify updated file listing
        let updated_files = handler.list_files().await;
        assert_eq!(updated_files.len(), 1);
        assert!(updated_files.contains(&dir.path().join("file1.rs")));

        Ok(())
    }
}