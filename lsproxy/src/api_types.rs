use lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, GotoDefinitionResponse, Location as LspLocation,
    LocationLink, OneOf, SymbolInformation, SymbolKind, Url, WorkspaceSymbol,
    WorkspaceSymbolResponse,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{to_value, Value};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};

pub const MOUNT_DIR: &str = "/mnt/repo";

#[derive(
    Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLanguages {
    Python,
    TypeScriptJavaScript,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilePosition {
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub identifier_start_position: FilePosition,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetDefinitionRequest {
    pub position: FilePosition,
}

#[derive(ToSchema, IntoParams)]
pub struct GetReferencesRequest {
    pub symbol_identifier_position: FilePosition,
    pub include_declaration: Option<bool>,
}

impl<'de> Deserialize<'de> for GetReferencesRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            symbol_identifier_position: String,
            include_declaration: Option<bool>,
        }

        let helper = Helper::deserialize(deserializer)?;

        let file_position: FilePosition = serde_json::from_str(&helper.symbol_identifier_position)
            .map_err(serde::de::Error::custom)?;

        Ok(GetReferencesRequest {
            symbol_identifier_position: file_position,
            include_declaration: helper.include_declaration,
        })
    }
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct FileSymbolsRequest {
    pub file_path: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct WorkspaceSymbolsRequest {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DefinitionResponse {
    raw_response: Value,
    definitions: Vec<FilePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceResponse {
    raw_response: Value,
    references: Vec<FilePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SymbolResponse {
    raw_response: Value,
    symbols: Vec<Symbol>,
}

impl From<LspLocation> for FilePosition {
    fn from(location: LspLocation) -> Self {
        FilePosition {
            path: uri_to_path_str(location.uri),
            line: location.range.start.line,
            character: location.range.start.character,
        }
    }
}

impl From<LocationLink> for FilePosition {
    fn from(link: LocationLink) -> Self {
        FilePosition {
            path: uri_to_path_str(link.target_uri),
            line: link.target_range.start.line,
            character: link.target_range.start.character,
        }
    }
}

impl From<SymbolInformation> for Symbol {
    fn from(symbol: SymbolInformation) -> Self {
        Symbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            identifier_start_position: FilePosition::from(symbol.location),
        }
    }
}

impl From<WorkspaceSymbol> for Symbol {
    fn from(symbol: WorkspaceSymbol) -> Self {
        let (path, identifier_start_line, identifier_start_character) = match symbol.location {
            OneOf::Left(location) => (
                uri_to_path_str(location.uri),
                location.range.start.line,
                location.range.start.character,
            ),
            OneOf::Right(workspace_location) => (uri_to_path_str(workspace_location.uri), 0, 0),
        };

        Symbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            identifier_start_position: FilePosition {
                path,
                line: identifier_start_line,
                character: identifier_start_character,
            },
        }
    }
}

impl From<GotoDefinitionResponse> for DefinitionResponse {
    fn from(response: GotoDefinitionResponse) -> Self {
        let raw_response = to_value(&response).unwrap_or_default();
        let definitions = match response {
            GotoDefinitionResponse::Scalar(location) => {
                vec![FilePosition::from(location)]
            }
            GotoDefinitionResponse::Array(locations) => {
                locations.into_iter().map(FilePosition::from).collect()
            }
            GotoDefinitionResponse::Link(links) => {
                links.into_iter().map(FilePosition::from).collect()
            }
        };
        DefinitionResponse {
            raw_response,
            definitions,
        }
    }
}

impl From<Vec<LspLocation>> for ReferenceResponse {
    fn from(locations: Vec<LspLocation>) -> Self {
        let raw_response = to_value(&locations).unwrap_or_default();
        let references = locations.into_iter().map(FilePosition::from).collect();
        ReferenceResponse {
            raw_response,
            references,
        }
    }
}

impl From<Vec<WorkspaceSymbolResponse>> for SymbolResponse {
    fn from(responses: Vec<WorkspaceSymbolResponse>) -> Self {
        let raw_response = to_value(&responses).unwrap_or_default();
        let symbols: Vec<Symbol> = responses
            .into_iter()
            .flat_map(|response| match response {
                WorkspaceSymbolResponse::Flat(symbols) => {
                    symbols.into_iter().map(Symbol::from).collect::<Vec<_>>()
                }
                WorkspaceSymbolResponse::Nested(symbols) => {
                    symbols.into_iter().map(Symbol::from).collect()
                }
            })
            .collect();

        SymbolResponse {
            raw_response,
            symbols,
        }
    }
}

impl SymbolResponse {
    pub fn new(response: DocumentSymbolResponse, file_path: &str) -> Self {
        let raw_response = to_value(&response).unwrap_or_default();
        let symbols = match response {
            DocumentSymbolResponse::Flat(symbols) => symbols
                .into_iter()
                .map(|symbol| Symbol {
                    name: symbol.name,
                    kind: symbol_kind_to_string(&symbol.kind).to_string(),
                    identifier_start_position: FilePosition {
                        path: file_path.to_string(),
                        line: symbol.location.range.start.line,
                        character: symbol.location.range.start.character,
                    },
                })
                .collect(),
            DocumentSymbolResponse::Nested(symbols) => flatten_nested_symbols(symbols, file_path),
        };
        SymbolResponse {
            raw_response,
            symbols,
        }
    }
}

fn uri_to_path_str(uri: Url) -> String {
    let path = uri
        .to_file_path()
        .unwrap_or_else(|_| PathBuf::from(uri.path()));

    let mount_dir = Path::new(MOUNT_DIR);
    path.strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>, file_path: &str) -> Vec<Symbol> {
    fn recursive_flatten(symbol: DocumentSymbol, file_path: &str, result: &mut Vec<Symbol>) {
        result.push(Symbol {
            name: symbol.name,
            kind: symbol_kind_to_string(&symbol.kind).to_string(),
            identifier_start_position: FilePosition {
                path: file_path.to_string(),
                line: symbol.selection_range.start.line,
                character: symbol.selection_range.start.character,
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
