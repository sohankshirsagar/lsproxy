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
    async fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client = Self {
            tags: TagDatabase::new()?,
        };
        client.generate(file_path).await?;
        client.load(Path::new(file_path).join("tags"))?;
        Ok(client)
    }

    async fn generate(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Run ctags command to generate tags file
        let output_file = Path::new(file_path).join("tags");
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
            if let Some(file_str) = file.to_str() {
                command.arg(file_str);
            }
        }

        let output = command.output()?;

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
        let mut files = Vec::new();
        let mut lines = Vec::new();
        let mut columns = Vec::new();

        // Read the tags file
        let file = File::open(path).expect("couldn't find tag file");
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
                    .unwrap_or(1) - 1;

                // Find column number using the line content from the tags file
                let column_number = line_content.find(tag_name).unwrap_or(0) as u32;

                names.push(tag_name.to_string());
                files.push(file_name.to_string());
                lines.push(line_number);
                columns.push(column_number);
            }
        }

        self.tags
            .add_tags_by_columns(names, files, lines, columns)?;
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
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_tags() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup_no_manager(&python_sample_path());
        let client = CtagsClient::new(&python_sample_path()).await?;
        let symbols = client.get_file_symbols("main.py")?;
        let expected = vec![
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
        ];
        assert_eq!(symbols, expected);
        Ok(())
    }
}
