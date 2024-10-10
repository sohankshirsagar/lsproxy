use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct RepoKey {
    pub id: String,
    pub github_url: String,
    pub branch: Option<String>,
    pub commit: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLSPs {
    Python,
    TypeScript,
    Rust,
}
