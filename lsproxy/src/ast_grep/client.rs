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
        let file_symbols = self.scan_file(&self.symbol_config_path, file_name);
        let matches = self.scan_file(&self.reference_config_path, file_name).await?;
        // Replace with code that finds the symbol that has a matching identifier position of the
        // one passed in and and then gets all the matches that are within that symbol's range
        Ok(Vec::new())
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
