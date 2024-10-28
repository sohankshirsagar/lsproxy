use tokio::process::Command;


use super::types::AstGrepMatch;

pub struct AstGrepClient {
    pub root_path: String,
    pub config_path: String,
}

impl AstGrepClient {

    pub async fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<AstGrepMatch>, Box<dyn std::error::Error>> {
        let command_result = Command::new("sg")
            .arg("--config")
            .arg(&self.config_path)
            .arg("--json")
            .arg(file_name)
            .output()
            .await?;
        let output = String::from_utf8(command_result.stdout)?;
        let symbols: Vec<AstGrepMatch> = serde_json::from_str(&output)?;
        Ok(symbols)
    }
}
