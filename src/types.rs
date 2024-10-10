use serde::{Serialize, Deserialize};
use utoipa::ToSchema;
use strum_macros::{EnumString, Display};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct RepoKey {
    pub id: String,
    pub github_url: String,
    pub branch: Option<String>,
    pub commit: String,
}

#[derive(Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLSPs {
    Python,
    TypeScript,
    Rust,
}
