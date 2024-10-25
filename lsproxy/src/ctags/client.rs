use super::tag_db::TagDatabase;
use crate::api_types::{get_mount_dir, Symbol};
use crate::utils::workspace_documents::{
    WorkspaceDocuments, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS, PYRIGHT_FILE_PATTERNS,
    RUST_ANALYZER_FILE_PATTERNS, TYPESCRIPT_FILE_PATTERNS,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CtagsClient {
    tags: TagDatabase,
}

impl CtagsClient {
    pub async fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client = Self {
            tags: TagDatabase::new()?,
        };
        client.generate(file_path).await?;
        client.load(Path::new(file_path).join("tags"))?;
        Ok(client)
    }

    async fn generate(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let output_file = Path::new(file_path).join("tags");

        // Build command with base args
        let mut command = Command::new("ctags");
        command.args(&[
            "--fields=+neKl", // Include line numbers, long kind names, and language
            "--python-kinds=-I",// Remove imports
            "--rust-kinds=-n",// Remove modules
            "--output-format=u-ctags",
            "-f",
            output_file
                .to_str()
                .expect("Output path contains invalid UTF-8"),
        ]);

        // Find all the workspace files
        let files = WorkspaceDocumentsHandler::new(
            Path::new(file_path),
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
        )
        .list_files()
        .await;

        // Add all discovered files to the command
        for file in files {
            if let Some(file_str) = file.to_str() {
                command.arg(file_str);
            }
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

    fn load(&mut self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Prepare vectors for column-based storage
        let mut names = Vec::new();
        let mut kinds = Vec::new();
        let mut languages = Vec::new();
        let mut files = Vec::new();
        let mut start_lines = Vec::new();
        let mut start_characters = Vec::new();
        let mut end_lines = Vec::new();

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
                let end_line = parts
                    .iter()
                    .find(|&&part| part.starts_with("end:"))
                    .and_then(|part| part.trim_start_matches("end:").parse::<u32>().ok())
                    .unwrap_or(1)
                    - 1;

                names.push(tag_name.to_string());
                kinds.push(kind.to_string());
                languages.push(language);
                files.push(file_name.to_string());
                start_lines.push(start_line);
                start_characters.push(start_character);
                end_lines.push(end_line);
            }
        }

        self.tags
            .add_tags_by_columns(names, kinds, languages, files, start_lines, start_characters, end_lines)?;
        Ok(())
    }

    pub fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<Symbol>, Box<dyn std::error::Error>> {
        let symbols = self.tags.get_file_symbols(file_name)?;
        Ok(symbols)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api_types::{FilePosition, Position, Symbol};
    use crate::test_utils::{python_sample_path, rust_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_tags() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup_no_manager(&python_sample_path());
        let client = CtagsClient::new(&python_sample_path()).await?;
        let symbols = client.get_file_symbols("main.py")?;
        let expected = vec![
            Symbol {
                name: String::from("graph"),
                kind: String::from("variable"),
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
                kind: String::from("variable"),
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
                kind: String::from("variable"),
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
    async fn test_rust_tags() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup_no_manager(&rust_sample_path());
        let client = CtagsClient::new(&rust_sample_path()).await?;
        let symbols = client.get_file_symbols("src/point.rs")?;
        let expected = vec![
            Symbol {
                name: String::from("Point"),
                kind: String::from("struct"),
                start_position: FilePosition {
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
                start_position: FilePosition {
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
                start_position: FilePosition {
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
                start_position: FilePosition {
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
                start_position: FilePosition {
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
                start_position: FilePosition {
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
                start_position: FilePosition {
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
                start_position: FilePosition {
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
}
