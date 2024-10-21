use crate::utils::file_utils::search_files;
use log::{debug, error, warn};
use lsp_types::Range;
use notify::{Config, Event, RecommendedWatcher, Watcher};
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

        let watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )
        .unwrap();

        let cache_clone = Arc::clone(&cache);
        let patterns_clone = Arc::clone(&patterns);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
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
