use serde::{Serialize, Deserialize};
use utoipa::ToSchema;
use strum_macros::{EnumString, Display};
use lsp_types::{GotoDefinitionResponse, Location, Url};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct RepoKey {
    pub id: String,
    pub github_url: String,
    pub branch: Option<String>,
    pub commit: String,
}

#[derive(Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLSPs {
    Python,
    TypeScript,
    Rust,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct UniqueDefinition {
    pub uri: Url,
    pub range_start: (u32, u32),
    pub range_end: (u32, u32),
    pub original: GotoDefinitionResponse,
}

impl From<GotoDefinitionResponse> for UniqueDefinition {
    fn from(response: GotoDefinitionResponse) -> Self {
        match response.clone() {
            GotoDefinitionResponse::Scalar(location) => Self::from_location(location, response),
            GotoDefinitionResponse::Array(locations) if !locations.is_empty() => {
                Self::from_location(locations[0].clone(), response)
            },
            GotoDefinitionResponse::Link(links) if !links.is_empty() => {
                let location = Location::new(links[0].target_uri.clone(), links[0].target_range);
                Self::from_location(location, response)
            },
            _ => panic!("Unexpected empty GotoDefinitionResponse"),
        }
    }
}

impl UniqueDefinition {
    fn from_location(location: Location, original: GotoDefinitionResponse) -> Self {
        UniqueDefinition {
            uri: location.uri,
            range_start: (location.range.start.line, location.range.start.character),
            range_end: (location.range.end.line, location.range.end.character),
            original,
        }
    }
}
