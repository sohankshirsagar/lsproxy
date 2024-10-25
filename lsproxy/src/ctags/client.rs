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
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
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

        Self::generate(root_path).await?;
        Self::load(db.clone(), Path::new(root_path).join(".tags")).await?;
        tokio::spawn(Self::handle_watch_events(
            root_path.to_string(),
            db.clone(),
            watch_events_rx,
        ));
        Ok(Self { tags: db })
    }

    async fn generate(root_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Run ctags command to generate tags file
        let output_file = Path::new(root_path).join(".tags");
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
            "--output-format=u-ctags",
            "-f",
            output_file
                .to_str()
                .expect("Output path contains invalid UTF-8"),
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
        Ok(())
    }

    async fn load(
        db: Arc<RwLock<TagDatabase>>,
        path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Prepare vectors for column-based storage
        let mut names = Vec::new();
        let mut files = Vec::new();
        let mut lines = Vec::new();
        let mut columns = Vec::new();

        // Read the tags file
        let file = File::open(path).expect("Failed to open tags file at the specified path");
        let reader = BufReader::new(file);

        // Process each line
        for line in reader.lines() {
            let line = line?;

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
                if let Err(e) = Self::generate(&root_path).await {
                    error!("Failed to generate tags: {}", e);
                    continue;
                }
                if let Err(e) = Self::load(db.clone(), Path::new(&root_path).join(".tags")).await {
                    error!("Failed to load tags: {}", e);
                } else {
                    debug!("Tags successfully regenerated and loaded.");
                }
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
    use std::fs::{remove_file};
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
