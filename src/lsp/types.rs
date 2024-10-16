use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumString};
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, SymbolKind, DocumentSymbol, LocationLink, Location, WorkspaceSymbolResponse, WorkspaceSymbol, OneOf, SymbolInformation, Url};

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

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SimplifiedLocation {
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

 #[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
 pub struct SimplifiedDocumentSymbol {
     pub name: String,
     pub kind: String,
     pub line: u32,
     pub character: u32,
 }

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SimplifiedWorkspaceSymbol {
    pub name: String,
    pub kind: String,
    pub uri: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomGotoDefinitionResponse {
    raw_response: serde_json::Value,
    simplified: Vec<SimplifiedLocation>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomWorkspaceSymbolResponse {
    raw_response: serde_json::Value,
    simplified: Vec<SimplifiedWorkspaceSymbol>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomReferenceResponse {
    raw_response: serde_json::Value,
    simplified: Vec<SimplifiedLocation>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CustomDocumentSymbolResponse {
    raw_response: serde_json::Value,
    simplified: Vec<SimplifiedDocumentSymbol>,
}

impl From<Location> for SimplifiedLocation {
    fn from(location: Location) -> Self {
        SimplifiedLocation {
            uri: simplify_uri(location.uri),
            line: location.range.start.line,
            character: location.range.start.character,
        }
    }
}

impl From<LocationLink> for SimplifiedLocation {
    fn from(link: LocationLink) -> Self {
        SimplifiedLocation {
            uri: simplify_uri(link.target_uri),
            line: link.target_range.start.line,
            character: link.target_range.start.character,
        }
    }
}

impl From<SymbolInformation> for SimplifiedWorkspaceSymbol {
    fn from(symbol: SymbolInformation) -> Self {
        SimplifiedWorkspaceSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            uri: symbol.location.uri.to_string(),
            line: symbol.location.range.start.line,
            character: symbol.location.range.start.character,
        }
    }
}

impl From<WorkspaceSymbol> for SimplifiedWorkspaceSymbol {
    fn from(symbol: WorkspaceSymbol) -> Self {
        let (uri, line, character) = match symbol.location {
            OneOf::Left(location) => {
                (location.uri.to_string(), location.range.start.line, location.range.start.character)
            },
            OneOf::Right(workspace_location) => {
                (workspace_location.uri.to_string(), 0, 0) // Default to 0 for line and character
            },
        };

        SimplifiedWorkspaceSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            uri,
            line,
            character,
        }
    }
}

impl From<GotoDefinitionResponse> for CustomGotoDefinitionResponse {
    fn from(response: GotoDefinitionResponse) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let simplified = match response {
            GotoDefinitionResponse::Scalar(location) => vec![SimplifiedLocation::from(location)],
            GotoDefinitionResponse::Array(locations) => locations.into_iter().map(SimplifiedLocation::from).collect(),
            GotoDefinitionResponse::Link(links) => links.into_iter().map(SimplifiedLocation::from).collect(),
        };
        CustomGotoDefinitionResponse {
            raw_response,
            simplified,
        }
    }
}

impl From<Vec<WorkspaceSymbolResponse>> for CustomWorkspaceSymbolResponse {
    fn from(responses: Vec<WorkspaceSymbolResponse>) -> Self {
        let raw_response = serde_json::to_value(&responses).unwrap_or_default();
        let simplified: Vec<SimplifiedWorkspaceSymbol> = responses.into_iter().flat_map(|response| {
            match response {
                WorkspaceSymbolResponse::Flat(symbols) => {
                    symbols.into_iter().map(SimplifiedWorkspaceSymbol::from).collect::<Vec<_>>()
                },
                WorkspaceSymbolResponse::Nested(symbols) => {
                    symbols.into_iter().map(SimplifiedWorkspaceSymbol::from).collect::<Vec<_>>()
                },
            }
        }).collect();

        CustomWorkspaceSymbolResponse {
            raw_response,
            simplified,
        }
    }
}

impl From<Vec<Location>> for CustomReferenceResponse {
    fn from(locations: Vec<Location>) -> Self {
        let raw_response = serde_json::to_value(&locations).unwrap_or_default();
        let simplified = locations.into_iter().map(SimplifiedLocation::from).collect();
        CustomReferenceResponse {
            raw_response,
            simplified
        }
    }
}

impl From<DocumentSymbolResponse> for CustomDocumentSymbolResponse {
    fn from(response: DocumentSymbolResponse) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let simplified = match response {
            DocumentSymbolResponse::Flat(symbols) => symbols
                .into_iter()
                .map(|symbol| SimplifiedDocumentSymbol {
                    name: symbol.name,
                    kind: symbol_kind_to_string(&symbol.kind).to_string(),
                    line: symbol.location.range.start.line,
                    character: symbol.location.range.start.character,
                })
                .collect(),
            DocumentSymbolResponse::Nested(symbols) => flatten_nested_symbols(symbols)
        };
        CustomDocumentSymbolResponse {
            raw_response,
            simplified,
        }
    }
}

fn simplify_uri(uri: Url) -> String {
    let path = uri.to_file_path().unwrap_or_else(|_| PathBuf::from(uri.path()));
    let current_dir = std::env::current_dir().unwrap_or_default();

    let simplified = path
        .strip_prefix(&current_dir)
        .map(|p| p.to_path_buf())
        .unwrap_or(path);

    let mount_dir = Path::new(MOUNT_DIR);
    simplified
        .strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| simplified.to_string_lossy().into_owned())
}

fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>) -> Vec<SimplifiedDocumentSymbol> {
    fn recursive_flatten(symbol: DocumentSymbol, result: &mut Vec<SimplifiedDocumentSymbol>) {
        result.push(SimplifiedDocumentSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            line: symbol.selection_range.start.line,
            character: symbol.selection_range.start.character,
        });

        for child in symbol.children.unwrap_or_default() {
            recursive_flatten(child, result);
        }
    }

    let mut flattened = Vec::new();
    for symbol in symbols {
        recursive_flatten(symbol, &mut flattened);
    }
    flattened
}

fn symbol_kind_to_string(kind: &SymbolKind) -> &str {
    match kind {
        &SymbolKind::FILE => "file",
        &SymbolKind::MODULE => "module",
        &SymbolKind::NAMESPACE => "namespace",
        &SymbolKind::PACKAGE => "package",
        &SymbolKind::CLASS => "class",
        &SymbolKind::METHOD => "method",
        &SymbolKind::PROPERTY => "property",
        &SymbolKind::FIELD => "field",
        &SymbolKind::CONSTRUCTOR => "constructor",
        &SymbolKind::ENUM => "enum",
        &SymbolKind::INTERFACE => "interface",
        &SymbolKind::FUNCTION => "function",
        &SymbolKind::VARIABLE => "variable",
        &SymbolKind::CONSTANT => "constant",
        &SymbolKind::STRING => "string",
        &SymbolKind::NUMBER => "number",
        &SymbolKind::BOOLEAN => "boolean",
        &SymbolKind::ARRAY => "array",
        &SymbolKind::OBJECT => "object",
        &SymbolKind::KEY => "key",
        &SymbolKind::NULL => "null",
        &SymbolKind::ENUM_MEMBER => "enum_member",
        &SymbolKind::STRUCT => "struct",
        &SymbolKind::EVENT => "event",
        &SymbolKind::OPERATOR => "operator",
        &SymbolKind::TYPE_PARAMETER => "type_parameter",
        _ => "unknown", // Default case for any future additions
    }
}
