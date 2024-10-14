use lsp_types::{GotoDefinitionResponse, Location, Url};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum_macros::{Display, EnumString};

pub const MOUNT_DIR: &str = "/mnt/repo";

#[derive(
    Debug,
    EnumString,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLSP {
    Python,
    TypeScriptJavaScript,
    Rust,
}
