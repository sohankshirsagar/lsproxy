use log::{debug, error};
use notify_debouncer_mini::DebouncedEvent;
use tokio::sync::broadcast::Receiver;
use tokio::sync::RwLock;

use super::tag_db::TagDatabase;
use crate::api_types::{get_mount_dir, Symbol};
use crate::utils::file_utils::search_files;
use crate::utils::workspace_documents::{
    DEFAULT_EXCLUDE_PATTERNS, PYRIGHT_FILE_PATTERNS, RUST_ANALYZER_FILE_PATTERNS,
    TYPESCRIPT_FILE_PATTERNS,
};
use std::io::BufRead;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
pub struct CtagsClient {
    tags: Arc<RwLock<TagDatabase>>,
}

impl CtagsClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Arc::new(RwLock::new(TagDatabase::new()?));

        let ctags = Self::generate(root_path).await?;
        Self::load(db.clone(), ctags).await?;
        tokio::spawn(Self::handle_watch_events(
            root_path.to_string(),
            db.clone(),
            watch_events_rx,
        ));
        Ok(Self { tags: db })
    }

    async fn generate(root_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Run ctags command to generate tags file
        let files = search_files(
            Path::new(root_path),
            PYRIGHT_FILE_PATTERNS
                .iter()
                .chain(TYPESCRIPT_FILE_PATTERNS.iter())
                .chain(RUST_ANALYZER_FILE_PATTERNS.iter())
                .map(|&s| s.to_string())
                .collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
        )?;

        // Build command with base args
        let mut command = Command::new("ctags");
        command.args(&[
            "--fields=+n", // Include line numbers
            "--quiet",     // don't print warnings
            "-f -",        // output to stdout
        ]);

        // Add all discovered files to the command
        for file in files {
            command.arg(file);
        }

        let output = command
            .output()
            .map_err(|e| format!("Failed to execute ctags command: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "ctags command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        let output_string = String::from_utf8(output.stdout)?;
        Ok(output_string)
    }

    async fn load(
        db: Arc<RwLock<TagDatabase>>,
        ctags: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Prepare vectors for column-based storage
        let mut names = Vec::new();
        let mut files = Vec::new();
        let mut lines = Vec::new();
        let mut columns = Vec::new();

        // Process each line
        for line in ctags.lines() {
            // Skip comment lines
            if line.starts_with('!') {
                continue;
            }

            // Parse tag line
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let tag_name = parts[0];
                let file_path = Path::new(parts[1]);
                let file_name = file_path
                    .strip_prefix(get_mount_dir())
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or(parts[1]);
                let line_content = parts[2].trim_start_matches("/^").trim_end_matches("$/");

                // Parse the line number from the extensions
                let line_number = parts
                    .iter()
                    .find(|&&part| part.starts_with("line:"))
                    .and_then(|part| part.trim_start_matches("line:").parse::<u32>().ok())
                    .unwrap_or(1)
                    - 1;

                // Find column number using the line content from the tags file
                let column_number = line_content.find(tag_name).unwrap_or(0) as u32;

                names.push(tag_name.to_string());
                files.push(file_name.to_string());
                lines.push(line_number);
                columns.push(column_number);
            }
        }
        db.write()
            .await
            .add_tags_by_columns(names, files, lines, columns)
    }

    async fn handle_watch_events(
        root_path: String,
        db: Arc<RwLock<TagDatabase>>,
        mut watch_events_rx: Receiver<DebouncedEvent>,
    ) {
        while let Ok(event) = watch_events_rx.recv().await {
            if Self::event_matches(&event) {
                db.write().await.clear();
                let ctags = Self::generate(&root_path).await.unwrap_or_else(|e| {
                    error!("Failed to generate tags: {}", e);
                    String::new()
                });
                Self::load(db.clone(), ctags).await.unwrap_or_else(|e| {
                    error!("Failed to load tags: {}", e);
                });
                debug!("Tags successfully regenerated and loaded.");
            }
        }
    }

    fn event_matches(event: &DebouncedEvent) -> bool {
        let path_str = event.path.to_string_lossy();
        let include_patterns: Vec<String> = PYRIGHT_FILE_PATTERNS
            .iter()
            .chain(TYPESCRIPT_FILE_PATTERNS.iter())
            .chain(RUST_ANALYZER_FILE_PATTERNS.iter())
            .map(|&s| s.to_string())
            .collect();
        let exclude_patterns: Vec<String> = DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect();

        let included = include_patterns
            .iter()
            .any(|pat| glob::Pattern::new(pat).unwrap().matches(&path_str));
        let excluded = exclude_patterns
            .iter()
            .any(|pat| glob::Pattern::new(pat).unwrap().matches(&path_str));
        included && !excluded
    }

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<Symbol>, Box<dyn std::error::Error>> {
        let symbols = self.tags.read().await.get_file_symbols(file_name)?;
        Ok(symbols)
    }
}

#[cfg(test)]
mod test {
    use fs_extra::dir::{copy, CopyOptions};
    use std::fs::remove_file;
    use tokio::sync::broadcast::{channel, Sender};
    use tokio::time::{sleep, Duration};

    use super::*;
    use crate::api_types::{FilePosition, Position, Symbol};
    use crate::test_utils::{python_sample_path, TestContext};

    fn create_test_watcher_channels() -> (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) {
        channel(100)
    }

    #[tokio::test]
    async fn test_python_tags() -> Result<(), Box<dyn std::error::Error>> {
        let (_, rx) = create_test_watcher_channels();
        let _context = TestContext::setup_no_manager(&python_sample_path());
        let client = CtagsClient::new(&python_sample_path(), rx).await?;
        let symbols = client.get_file_symbols("main.py").await?;
        let expected = vec![
            Symbol {
                name: String::from("plt"),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 0,
                        character: 28,
                    },
                },
            },
            Symbol {
                name: String::from("graph"),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 5,
                        character: 0,
                    },
                },
            },
            Symbol {
                name: String::from("result"),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 0,
                    },
                },
            },
            Symbol {
                name: String::from("cost"),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 8,
                    },
                },
            },
        ];
        assert_eq!(symbols, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_watch_event_deletion() -> Result<(), Box<dyn std::error::Error>> {
        // 1. Copy the python project to a temp dir
        let temp_dir = tempfile::tempdir()?;
        copy(
            &python_sample_path(),
            temp_dir.path(),
            &CopyOptions::new().overwrite(true),
        )?;

        // 2. Check the symbols in main
        let (tx, rx) = create_test_watcher_channels();
        let client = CtagsClient::new(temp_dir.path().to_str().unwrap(), rx).await?;
        let symbols_before = client.get_file_symbols("main.py").await?;
        assert!(!symbols_before.is_empty());

        // 3. Delete main from temp dir
        remove_file(temp_dir.path().join("main.py"))?;

        // 4. Send a watch event
        tx.send(DebouncedEvent {
            path: temp_dir.path().join("main.py"),
            kind: notify_debouncer_mini::DebouncedEventKind::Any,
        })?;

        // 5. Sleep to allow event processing
        sleep(Duration::from_millis(100)).await;

        // 6. Check the symbols in main
        let symbols_after = client.get_file_symbols("main.py").await?;

        // 7. Should be empty
        assert!(symbols_after.is_empty());

        Ok(())
    }
}
