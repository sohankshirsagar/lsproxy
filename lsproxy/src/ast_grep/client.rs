use tokio::process::Command;
use std::io::{Error, ErrorKind};

use super::types::AstGrepMatch;
use crate::api_types::{FileRange, Position};

pub struct AstGrepClient {
    pub symbol_config_path: String,
    pub identifier_config_path: String,
    pub reference_config_path: String,
}

impl AstGrepClient {
    pub async fn get_symbol_from_position(
        &self,
        file_name: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<AstGrepMatch, Box<dyn std::error::Error>> {
        // Get all symbols in the file
        let file_symbols = self.scan_file(&self.symbol_config_path, file_name).await?;

        // Find the symbol that matches our identifier position
        let symbol_result = file_symbols.into_iter().find(|symbol| {
            symbol.range.start.line == identifier_position.line
                && symbol.range.start.column == identifier_position.character
        });
        match symbol_result {
            Some(matched_symbol) => Ok(matched_symbol),
            None => Err(Box::new(Error::new(ErrorKind::NotFound, "No symbol found for position"))),

        }
    }

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.symbol_config_path, file_name).await
    }

    pub async fn get_file_identifiers(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.identifier_config_path, file_name).await
    }

    pub async fn get_references_contained_in_symbol(
        &self,
        file_name: &str,
        identifier_position: &lsp_types::Position,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {

        // Get all references
        let matches = self
            .scan_file(&self.reference_config_path, file_name)
            .await?;

        let symbol = self.get_symbol_from_position(file_name, identifier_position).await?;

        // Filter references to only those within its range
        let symbol_range = FileRange {
            path: symbol.file.clone(),
            start: Position {
                line: symbol.meta_variables.single.context.range.start.line as u32,
                character: symbol.meta_variables.single.context.range.start.column as u32,
            },
            end: Position {
                line: symbol.meta_variables.single.context.range.end.line as u32,
                character: symbol.meta_variables.single.context.range.end.column as u32,
            },
        };

        // Filter matches to those within the symbol's range
        let contained_references = matches
            .into_iter()
            .filter(|m| {
                symbol_range.contains(&Position {
                    line: m.range.start.line as u32,
                    character: m.range.start.column as u32,
                })
            })
            .collect();

            Ok(contained_references)
    }

    async fn scan_file(
        &self,
        config_path: &str,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("ast-grep")
            .arg("scan")
            .arg("--config")
            .arg(config_path)
            .arg("--json")
            .arg(file_name)
            .output()
            .await?;

        if !command_result.status.success() {
            let error = String::from_utf8_lossy(&command_result.stderr);
            return Err(format!("sg command failed: {}", error).into());
        }

        let output = String::from_utf8(command_result.stdout)?;

        let mut symbols: Vec<AstGrepMatch> =
            serde_json::from_str(&output).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        symbols = symbols
            .into_iter()
            .filter(|s| s.rule_id != "all-identifiers")
            .collect();
        symbols.sort_by_key(|s| s.range.start.line);
        Ok(symbols)
    }

    pub async fn get_file_identifiers(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("ast-grep")
            .arg("scan")
            .arg("--config")
            .arg(&self.config_path)
            .arg("--json")
            .arg(file_name)
            .output()
            .await?;

        if !command_result.status.success() {
            let error = String::from_utf8_lossy(&command_result.stderr);
            return Err(format!("sg command failed: {}", error).into());
        }

        let output = String::from_utf8(command_result.stdout)?;
        let mut identifiers: Vec<AstGrepMatch> =
            serde_json::from_str(&output).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        identifiers = identifiers
            .into_iter()
            .filter(|s| s.rule_id == "all-identifiers")
            .collect();

        identifiers.sort_by_key(|s| s.range.start.line);
        Ok(identifiers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_references() -> Result<(), Box<dyn std::error::Error>> {
        let client = AstGrepClient {
            symbol_config_path: String::from("/usr/src/ast_grep/symbol-config.yml"),
            reference_config_path: String::from("/usr/src/ast_grep/reference-config.yml"),
        };

        let path = "/mnt/lsproxy_root/sample_project/python/graph.py";
        let position = lsp_types::Position {
            line: 6, // Line with @log_execution_time decorator
            character: 6,
        };

        let references = client
            .get_references_contained_in_symbol(path, &position)
            .await?;
        let match_positions: Vec<lsp_types::Position> = references
            .iter()
            .map(lsp_types::Position::from)
            .collect();
        let expected = vec![
            lsp_types::Position {
                line: 6,
                character: 17,
            },
            lsp_types::Position {
                line: 8,
                character: 24,
            },
            lsp_types::Position {
                line: 8,
                character: 29,
            },
            lsp_types::Position {
                line: 8,
                character: 34,
            },
            lsp_types::Position {
                line: 8,
                character: 40,
            },
            lsp_types::Position {
                line: 8,
                character: 45,
            },
            lsp_types::Position {
                line: 9,
                character: 23,
            },
            lsp_types::Position {
                line: 16,
                character: 5,
            },
            lsp_types::Position {
                line: 18,
                character: 20,
            },
            lsp_types::Position {
                line: 20,
                character: 5,
            },
            lsp_types::Position {
                line: 26,
                character: 13,
            },
            lsp_types::Position {
                line: 27,
                character: 13,
            },
            lsp_types::Position {
                line: 28,
                character: 46,
            },
            lsp_types::Position {
                line: 30,
                character: 5,
            },
            lsp_types::Position {
                line: 48,
                character: 14,
            },
            lsp_types::Position {
                line: 52,
                character: 28,
            },
        ];
        assert_eq!(match_positions, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_contained_references() -> Result<(), Box<dyn std::error::Error>> {
        let client = AstGrepClient {
            symbol_config_path: String::from("/usr/src/ast_grep/symbol-config.yml"),
            reference_config_path: String::from("/usr/src/ast_grep/reference-config.yml"),
        };

        let path = "/mnt/lsproxy_root/sample_project/python/graph.py";
        let position = lsp_types::Position {
            line: 51, // Line with @log_execution_time decorator
            character: 8,
        };

        let references = client
            .get_references_contained_in_symbol(path, &position)
            .await
            .unwrap();
        let match_positions: Vec<lsp_types::Position> = references
            .iter()
            .map(|ast_match: &AstGrepMatch| lsp_types::Position {
                line: ast_match.range.start.line as u32,
                character: ast_match.range.start.column as u32,
            })
            .collect();
        let expected = vec![lsp_types::Position {
            line: 52,
            character: 28,
        }];
        assert_eq!(match_positions, expected);
        Ok(())
    }
}
