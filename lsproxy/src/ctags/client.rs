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
        // Build command with base args
        let mut command = Command::new("ctags");
        command.args(&[
            "--map-typescript=+.tsx", // Enable typescript tsx files
            "--fields=+neKl", // Include line numbers, long kind names, and language
            "--python-kinds=-iIx", // Remove imports
            "--rust-kinds=-n", // Remove modules
            "--output-format=u-ctags",
            "--quiet",           // don't print warnings
            "-f -",
        ]);

        // Find all the workspace files
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
        let mut kinds = Vec::new();
        let mut languages = Vec::new();
        let mut files = Vec::new();
        let mut start_lines = Vec::new();
        let mut start_characters = Vec::new();
        let mut end_lines = Vec::new();

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
                let kind = parts[3];
                let file_name = file_path
                    .strip_prefix(get_mount_dir())
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or(parts[1]);
                let line_content = parts[2].trim_start_matches("/^").trim_end_matches("$/");

                // Parse the language
                let language = parts
                    .iter()
                    .find(|&&part| part.starts_with("language:"))
                    .and_then(|part| part.trim_start_matches("language:").parse::<String>().ok())
                    .unwrap_or(String::from("unknown"));

                // Parse the start line number
                let start_line = parts
                    .iter()
                    .find(|&&part| part.starts_with("line:"))
                    .and_then(|part| part.trim_start_matches("line:").parse::<u32>().ok())
                    .unwrap_or(1)
                    - 1;

                // Find start character using the line content from the tags file
                let start_character = line_content.find(tag_name).unwrap_or(0) as u32;

                // Parse the end line number
                // WE ARE ADDING 1 HERE TO MAKE THE RANGE INCLUSIVE
                // WITHOUT KNOWING HOW LONG THE END LINE IS
                // IF THERE IS NO END WE ASSUME IT IS THE SAME AS THE START LINE
                let end_line = parts
                    .iter()
                    .find(|&&part| part.starts_with("end:"))
                    .and_then(|part| part.trim_start_matches("end:").parse::<u32>().ok())
                    .unwrap_or(start_line + 1);

                names.push(tag_name.to_string());
                kinds.push(kind.to_string());
                languages.push(language);
                files.push(file_name.to_string());
                start_lines.push(start_line);
                start_characters.push(start_character);
                end_lines.push(end_line);
            }
        }
        db.write().await.add_tags_by_columns(
            names,
            kinds,
            languages,
            files,
            start_lines,
            start_characters,
            end_lines,
        )
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

    use tokio::sync::broadcast::{channel, Sender};
    use tokio::time::{sleep, Duration};

    use super::*;
    use crate::api_types::{FilePosition, Position, Symbol};
    use crate::test_utils::{python_sample_path, rust_sample_path, TestContext};

    fn create_test_watcher_channels() -> (Sender<DebouncedEvent>, Receiver<DebouncedEvent>) {
        channel(100)
    }

    #[tokio::test]
    async fn test_python_tags() -> Result<(), Box<dyn std::error::Error>> {
        let (_, rx) = create_test_watcher_channels();
        let _context = TestContext::setup_no_manager(&python_sample_path());
        let client = CtagsClient::new(&python_sample_path(), rx).await?;
        let symbols = client.get_file_symbols("graph.py")?;
        let expected = vec![
            Symbol {
                name: String::from("AstarGraph"),
                kind: String::from("class"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 0,
                        character: 6,
                    },
                },
            },
            Symbol {
                name: String::from("__init__"),
                kind: String::from("member"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 3,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("heuristic"),
                kind: String::from("member"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 22,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("get_vertex_neighbours"),
                kind: String::from("member"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 31,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("move_cost"),
                kind: String::from("member"),
                identifier_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 51,
                        character: 8,
                    },
                },
            },
        ];
        assert_eq!(symbols, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_rust_tags() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup_no_manager(&rust_sample_path());
        let (_, rx) = create_test_watcher_channels();
        let client = CtagsClient::new(&rust_sample_path(), rx).await?;
        let symbols = client.get_file_symbols("src/point.rs").await?;
        let expected = vec![
            Symbol {
                name: String::from("Point"),
                kind: String::from("struct"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 1,
                        character: 11,
                    },
                },
            },
            Symbol {
                name: String::from("x"),
                kind: String::from("field"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 2,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("y"),
                kind: String::from("field"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 3,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("Point"),
                kind: String::from("implementation"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 6,
                        character: 5,
                    },
                },
            },
            Symbol {
                name: String::from("new"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 7,
                        character: 11,
                    },
                },
            },
            Symbol {
                name: String::from("Point"),
                kind: String::from("implementation"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 12,
                        character: 23,
                    },
                },
            },
            Symbol {
                name: String::from("Output"),
                kind: String::from("typedef"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 13,
                        character: 9,
                    },
                },
            },
            Symbol {
                name: String::from("add"),
                kind: String::from("method"),
                identifier_position: FilePosition {
                    path: String::from("src/point.rs"),
                    position: Position {
                        line: 15,
                        character: 7,
                    },
                },
            },
        ];
        assert_eq!(symbols, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_watch_event_deletion() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = create_test_watcher_channels();
        let sample_path = python_sample_path();
        let _context = TestContext::setup_no_manager(&sample_path);
        let client = CtagsClient::new(&sample_path, rx).await?;
        // this is done after client is initialized, so ctags are already loaded
        let temp_file = tempfile::Builder::new()
            .prefix("test_file")
            .suffix(".py")
            .tempfile_in(&sample_path)?;
        tokio::fs::write(
            &temp_file.path(),
            "def test_func():\n    x = 1\n    return x",
        )
        .await?;

        let relative_file_path = temp_file.path().file_name().unwrap().to_str().unwrap();

        let symbols = client.get_file_symbols(relative_file_path).await?;
        assert!(symbols.is_empty()); // add a new temp f

        tx.send(DebouncedEvent {
            path: temp_file.path().to_path_buf(),
            kind: notify_debouncer_mini::DebouncedEventKind::Any,
        })?;

        sleep(Duration::from_millis(100)).await;

        let symbols_after = client.get_file_symbols(relative_file_path).await?;

        assert!(
            !symbols_after.is_empty(),
            "No symbols found in {}",
            relative_file_path
        );

        Ok(())
    }
}
