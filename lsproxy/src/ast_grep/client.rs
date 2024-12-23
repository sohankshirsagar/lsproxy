use tokio::process::Command;

use crate::api_types::FilePosition;
use super::types::AstGrepMatch;

pub struct AstGrepClient {
    pub symbol_config_path: String,
    pub reference_config_path: String,
}

impl AstGrepClient {

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        self.scan_file(&self.symbol_config_path, file_name).await
    }

    pub async fn get_references_contained_in_symbol(
        &self,
        identifier_position: FilePosition,
        file_name: &str,
    ) -> Result<Vec<FilePosition>, Box<dyn std::error::Error>> {
        // Get all symbols in the file
        let file_symbols = self.scan_file(&self.symbol_config_path, file_name).await?;
        
        // Find the symbol that contains our identifier position
        let containing_symbol = file_symbols.iter().find(|symbol| {
            symbol.range.start.line == identifier_position.position.line 
            && symbol.range.start.column == identifier_position.position.character
        });

        // Get all references
        let matches = self.scan_file(&self.reference_config_path, file_name).await?;

        // If we found a containing symbol, filter references to only those within its range
        if let Some(symbol) = containing_symbol {
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

            // Convert matches to FilePositions and filter to those within the symbol's range
            let contained_references = matches.into_iter()
                .map(|m| FilePosition {
                    path: m.file,
                    position: Position {
                        line: m.range.start.line as u32,
                        character: m.range.start.column as u32,
                    },
                })
                .filter(|pos| symbol_range.contains(&pos.position))
                .collect();

            Ok(contained_references)
        } else {
            Ok(Vec::new())
        }
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

        let mut symbols: Vec<AstGrepMatch> = serde_json::from_str(&output)
            .map_err(|e| format!("Failed to parse JSON: {}\nJSON: {}", e, output))?;
        symbols.sort_by_key(|s| s.range.start.line);
        Ok(symbols)
    }
}
