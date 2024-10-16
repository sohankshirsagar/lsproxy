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
pub struct SimpleLocation {
    pub path: String,
    pub identifier_start_line: u32,
    pub identifier_start_character: u32,
}

 #[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
 pub struct SimpleSymbol {
     pub name: String,
     pub kind: String,
     pub location: SimpleLocation,
 }

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SimpleGotoDefinitionResponse{
    raw_response: serde_json::Value,
    definitions: Vec<SimpleLocation>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SimpleReferenceResponse {
    raw_response: serde_json::Value,
    references: Vec<SimpleLocation>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SimpleSymbolResponse {
    raw_response: serde_json::Value,
    symbols: Vec<SimpleSymbol>,
}

impl From<Location> for SimpleLocation {
    fn from(location: Location) -> Self {
        SimpleLocation {
            path: uri_to_path_str(location.uri),
            identifier_start_line: location.range.start.line,
            identifier_start_character: location.range.start.character,
        }
    }
}

impl From<LocationLink> for SimpleLocation {
    fn from(link: LocationLink) -> Self {
        SimpleLocation {
            path: uri_to_path_str(link.target_uri),
            identifier_start_line: link.target_range.start.line,
            identifier_start_character: link.target_range.start.character,
        }
    }
}

impl From<SymbolInformation> for SimpleSymbol {
    fn from(symbol: SymbolInformation) -> Self {
        SimpleSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            location: SimpleLocation {
                path: uri_to_path_str(symbol.location.uri),
                identifier_start_line: symbol.location.range.start.line,
                identifier_start_character: symbol.location.range.start.character,
            },
        }
    }
}

impl From<WorkspaceSymbol> for SimpleSymbol {
    fn from(symbol: WorkspaceSymbol) -> Self {
        let (path, identifier_start_line, identifier_start_character) = match symbol.location {
            OneOf::Left(location) => {
                (uri_to_path_str(location.uri), location.range.start.line, location.range.start.character)
            },
            OneOf::Right(workspace_location) => {
                (uri_to_path_str(workspace_location.uri), 0, 0) // Default to 0 for line and character
            },
        };

        SimpleSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            location: SimpleLocation {
                path,
                identifier_start_line,
                identifier_start_character,
            },
        }
    }
}

impl From<GotoDefinitionResponse> for SimpleGotoDefinitionResponse{
    fn from(response: GotoDefinitionResponse) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let definitions = match response {
            GotoDefinitionResponse::Scalar(location) => vec![SimpleLocation::from(location)],
            GotoDefinitionResponse::Array(locations) => locations.into_iter().map(SimpleLocation::from).collect(),
            GotoDefinitionResponse::Link(links) => links.into_iter().map(SimpleLocation::from).collect(),
        };
        SimpleGotoDefinitionResponse
    {
            raw_response,
            definitions,
        }
    }
}

impl From<Vec<WorkspaceSymbolResponse>> for SimpleSymbolResponse {
    fn from(responses: Vec<WorkspaceSymbolResponse>) -> Self {
        let raw_response = serde_json::to_value(&responses).unwrap_or_default();
        let symbols: Vec<SimpleSymbol> = responses.into_iter().flat_map(|response| {
            match response {
                WorkspaceSymbolResponse::Flat(symbols) => {
                    symbols.into_iter().map(SimpleSymbol::from).collect::<Vec<_>>()
                },
                WorkspaceSymbolResponse::Nested(symbols) => {
                    symbols.into_iter().map(SimpleSymbol::from).collect::<Vec<_>>()
                },
            }
        }).collect();

        SimpleSymbolResponse {
            raw_response,
            symbols,
        }
    }
}

impl From<Vec<Location>> for SimpleReferenceResponse {
    fn from(locations: Vec<Location>) -> Self {
        let raw_response = serde_json::to_value(&locations).unwrap_or_default();
        let references = locations.into_iter().map(SimpleLocation::from).collect();
        SimpleReferenceResponse {
            raw_response,
            references,
        }
    }
}

impl SimpleSymbolResponse {
    pub fn new(response: DocumentSymbolResponse, file_path: &str) -> Self {
        let raw_response = serde_json::to_value(&response).unwrap_or_default();
        let symbols = match response {
            DocumentSymbolResponse::Flat(symbols) => symbols
                .into_iter()
                .map(|symbol| SimpleSymbol {
                    name: symbol.name,
                    kind: symbol_kind_to_string(&symbol.kind).to_string(),
                    location: SimpleLocation {
                        path: file_path.to_string(),
                        identifier_start_line: symbol.location.range.start.line,
                        identifier_start_character: symbol.location.range.start.character,
                    }
                })
                .collect(),
            DocumentSymbolResponse::Nested(symbols) => flatten_nested_symbols(symbols, file_path),
        };
        SimpleSymbolResponse {
            raw_response,
            symbols,
        }
    }
}

fn uri_to_path_str(uri: Url) -> String {
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

fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>, file_path: &str) -> Vec<SimpleSymbol> {
    fn recursive_flatten(symbol: DocumentSymbol, file_path: &str, result: &mut Vec<SimpleSymbol>) {
        result.push(SimpleSymbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            location: SimpleLocation {
                path: file_path.to_string(),
                identifier_start_line: symbol.selection_range.start.line,
                identifier_start_character: symbol.selection_range.start.character,
            },
        });

        for child in symbol.children.unwrap_or_default() {
            recursive_flatten(child, file_path, result);
        }
    }

    let mut flattened = Vec::new();
    for symbol in symbols {
        recursive_flatten(symbol, file_path, &mut flattened);
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
