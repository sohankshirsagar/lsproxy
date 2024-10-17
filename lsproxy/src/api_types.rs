use log::warn;
use lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, GotoDefinitionResponse, Location, LocationLink, OneOf,
    SymbolInformation, SymbolKind, Url, WorkspaceLocation, WorkspaceSymbol,
    WorkspaceSymbolResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Value};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};

pub const MOUNT_DIR: &str = "/mnt/workspace";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(
    Debug, EnumString, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum SupportedLanguages {
    #[serde(rename = "python")]
    Python,
    /// TypeScript and JavaScript are handled by the same langserver
    #[serde(rename = "typescript_javascript")]
    TypeScriptJavaScript,
    #[serde(rename = "rust")]
    Rust,
}

/// Specific position within a file.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilePosition {
    #[schema(example = "src/main.py")]
    pub path: String,
    #[schema(example = 10)]
    pub line: u32,
    #[schema(example = 5)]
    pub character: u32,
}

/// Represents a symbol within the codebase.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Symbol {
    /// The name of the symbol.
    #[schema(example = "User")]
    pub name: String,
    /// The kind of the symbol (e.g., function, class).
    #[schema(example = "class")]
    pub kind: String,
    /// The starting position of the symbol's identifier, and the file_path from workspace root
    ///
    /// e.g. for the definition of `User` on line 0 of `src/main.py` with the code:
    /// ```
    /// 0: class User:
    /// _________^
    /// 1:     def __init__(self, name, age):
    /// 2:         self.name = name
    /// 3:         self.age = age
    /// ```
    pub identifier_start_position: FilePosition,
}

/// Request to get the definition of a symbol at a given position in a file.
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetDefinitionRequest {
    /// The position within the file to get the definition for. This should point to the identifier
    /// of the symbol you want to get the definition for.
    ///
    /// e.g. for getting the definition of `User` on line 10 of `src/main.py` with the code:
    /// ```
    /// 0: class User:
    /// 1:     def __init__(self, name, age):
    /// 2:         self.name = name
    /// 3:         self.age = age
    /// 4:
    /// 5: user = User("John", 30)
    /// __________^^^
    /// ```
    /// The (line, char) should be anywhere in (5, 7)-(5, 11).
    pub position: FilePosition,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

/// Request to get the references of a symbol in the workspace.
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetReferencesRequest {
    /// The position within the file to get the references for. This should point to the identifier of the definition.
    ///
    /// e.g. for getting the references of `User` on line 0 of `src/main.py` with the code:
    /// ```
    /// 0: class User:
    /// _________^^^^
    /// 1:     def __init__(self, name, age):
    /// 2:         self.name = name
    /// 3:         self.age = age
    /// 4:
    /// 5: user = User("John", 30)
    /// ```
    pub symbol_identifier_position: FilePosition,

    /// Whether to include the declaration (definition) of the symbol in the response.
    /// Defaults to false.
    #[serde(default)]
    pub include_declaration: bool,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

/// Request to get the symbols in a file.
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct FileSymbolsRequest {
    /// The path to the file to get the symbols for, relative to the root of the workspace.
    #[schema(example = "src/main.py")]
    pub file_path: String,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

/// Request to get the symbols in the workspace.
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct WorkspaceSymbolsRequest {
    /// The query to search for.
    #[schema(example = "User")]
    pub query: String,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

/// Response to a definition request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DefinitionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
    pub raw_response: Option<Value>,
    /// The definition(s) of the symbol.
    /// Points to the start position of the symbol's identifier.
    ///
    /// e.g. for the definition of `User` on line 5 of `src/main.py` with the code:
    /// ```
    /// 0: class User:
    /// _________^
    /// 1:     def __init__(self, name, age):
    /// 2:         self.name = name
    /// 3:         self.age = age
    /// 4:
    /// 5: user = User("John", 30)
    /// __________^
    /// ```
    /// The definition(s) will be `[{"path": "src/main.py", "line": 0, "character": 6}]`.
    pub definitions: Vec<FilePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references
    pub raw_response: Option<Value>,
    /// The references to the symbol.
    /// Points to the start position of the symbol's identifier.
    ///
    /// e.g. for the references of `User` on line 0 character 6 of `src/main.py` with the code:
    /// ```
    /// 0: class User:
    /// _________^
    /// 1:     def __init__(self, name, age):
    /// 2:         self.name = name
    /// 3:         self.age = age
    /// 4:
    /// 5: user = User("John", 30)
    /// _________^
    /// 6:
    /// 7: print(user.name)
    /// ```
    /// The references will be `[{"path": "src/main.py", "line": 5, "character": 7}]`.
    ///
    pub references: Vec<FilePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SymbolResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#document_symbol
    pub raw_response: Option<Value>,
    pub symbols: Vec<Symbol>,
}

impl From<(GotoDefinitionResponse, bool)> for DefinitionResponse {
    fn from((response, include_raw): (GotoDefinitionResponse, bool)) -> Self {
        let raw_response = if include_raw {
            Some(to_value(&response).unwrap_or_else(|e| {
                warn!("Serialization failed: {:?}", e);
                Value::Null
            }))
        } else {
            None
        };
        let definitions = match response {
            GotoDefinitionResponse::Scalar(location) => vec![FilePosition::from(location)],
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

impl From<(Vec<Location>, bool)> for ReferenceResponse {
    fn from((locations, include_raw): (Vec<Location>, bool)) -> Self {
        let raw_response = if include_raw {
            Some(to_value(&locations).unwrap_or_default())
        } else {
            None
        };
        let references = locations.into_iter().map(FilePosition::from).collect();
        ReferenceResponse {
            raw_response,
            references,
        }
    }
}

impl From<Location> for FilePosition {
    fn from(location: Location) -> Self {
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
            kind: symbol_kind_to_string(symbol.kind).to_owned(),
            identifier_start_position: FilePosition::from(symbol.location),
        }
    }
}

impl From<WorkspaceLocation> for FilePosition {
    fn from(location: WorkspaceLocation) -> Self {
        warn!("WorkspaceLocation does not contain line and character information and will not be shown");
        FilePosition {
            path: uri_to_path_str(location.uri),
            line: 0,
            character: 0,
        }
    }
}

impl From<WorkspaceSymbol> for Symbol {
    fn from(symbol: WorkspaceSymbol) -> Self {
        Symbol {
            name: symbol.name,
            kind: symbol_kind_to_string(symbol.kind).to_owned(),
            identifier_start_position: match symbol.location {
                OneOf::Left(location) => FilePosition::from(location),
                OneOf::Right(workspace_location) => {
                    warn!("WorkspaceLocation does not contain line and character information and will not be shown");
                    FilePosition::from(workspace_location)
                }
            },
        }
    }
}

impl From<(Vec<WorkspaceSymbolResponse>, bool)> for SymbolResponse {
    fn from((responses, include_raw): (Vec<WorkspaceSymbolResponse>, bool)) -> Self {
        let raw_response = if include_raw {
            Some(to_value(&responses).unwrap_or_default())
        } else {
            None
        };
        let symbols: Vec<Symbol> = responses
            .into_iter()
            .flat_map(|response| match response {
                WorkspaceSymbolResponse::Flat(symbols) => {
                    symbols.into_iter().map(Symbol::from).collect::<Vec<_>>()
                }
                WorkspaceSymbolResponse::Nested(symbols) =>{
                    warn!("Nested symbols are missing line and character information and will not be shown");
                    symbols.into_iter().map(Symbol::from).collect::<Vec<_>>()
                }
            })
            .collect();

        SymbolResponse {
            raw_response,
            symbols,
        }
    }
}

impl From<(DocumentSymbolResponse, String, bool)> for SymbolResponse {
    fn from((response, file_path, include_raw): (DocumentSymbolResponse, String, bool)) -> Self {
        let raw_response = include_raw.then(|| to_value(&response).unwrap_or_default());
        let symbols = match response {
            DocumentSymbolResponse::Flat(symbols) => symbols
                .into_iter()
                .map(|symbol| Symbol {
                    name: symbol.name,
                    kind: symbol_kind_to_string(symbol.kind).to_owned(),
                    identifier_start_position: FilePosition::from(symbol.location),
                })
                .collect(),
            DocumentSymbolResponse::Nested(symbols) => flatten_nested_symbols(symbols, &file_path),
        };
        SymbolResponse {
            raw_response,
            symbols,
        }
    }
}

fn uri_to_path_str(uri: Url) -> String {
    let path = uri.to_file_path().unwrap_or_else(|e| {
        warn!("Failed to convert URI to file path: {:?}", e);
        PathBuf::from(uri.path())
    });

    let mount_dir = Path::new(MOUNT_DIR);
    path.strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|e| {
            warn!("Failed to strip prefix: {:?}", e);
            path.to_string_lossy().into_owned()
        })
}

fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>, file_path: &str) -> Vec<Symbol> {
    fn recursive_flatten(symbol: DocumentSymbol, file_path: &str, result: &mut Vec<Symbol>) {
        result.push(Symbol {
            name: symbol.name,
            kind: symbol_kind_to_string(symbol.kind).to_owned(),
            identifier_start_position: FilePosition {
                path: file_path.to_owned(),
                line: symbol.selection_range.start.line,
                character: symbol.selection_range.start.character,
            },
        });

        if let Some(children) = symbol.children {
            for child in children {
                recursive_flatten(child, file_path, result);
            }
        }
    }

    let mut flattened = Vec::new();

    for symbol in symbols {
        recursive_flatten(symbol, file_path, &mut flattened);
    }
    flattened
}

fn symbol_kind_to_string(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::FILE => "file",
        SymbolKind::MODULE => "module",
        SymbolKind::NAMESPACE => "namespace",
        SymbolKind::PACKAGE => "package",
        SymbolKind::CLASS => "class",
        SymbolKind::METHOD => "method",
        SymbolKind::PROPERTY => "property",
        SymbolKind::FIELD => "field",
        SymbolKind::CONSTRUCTOR => "constructor",
        SymbolKind::ENUM => "enum",
        SymbolKind::INTERFACE => "interface",
        SymbolKind::FUNCTION => "function",
        SymbolKind::VARIABLE => "variable",
        SymbolKind::CONSTANT => "constant",
        SymbolKind::STRING => "string",
        SymbolKind::NUMBER => "number",
        SymbolKind::BOOLEAN => "boolean",
        SymbolKind::ARRAY => "array",
        SymbolKind::OBJECT => "object",
        SymbolKind::KEY => "key",
        SymbolKind::NULL => "null",
        SymbolKind::ENUM_MEMBER => "enum_member",
        SymbolKind::STRUCT => "struct",
        SymbolKind::EVENT => "event",
        SymbolKind::OPERATOR => "operator",
        SymbolKind::TYPE_PARAMETER => "type_parameter",
        _ => "unknown",
    }
}
