use log::{debug, warn};
use lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, GotoDefinitionResponse, Location, LocationLink,
    SymbolInformation, SymbolKind, Url,
};
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Value};
use std::cell::RefCell;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};
use std::sync::{Arc, RwLock, LazyLock};

 static GLOBAL_MOUNT_DIR: LazyLock<Arc<RwLock<PathBuf>>> = LazyLock::new(|| {
     Arc::new(RwLock::new(PathBuf::from("/mnt/workspace")))
 });

 thread_local! {
     static THREAD_LOCAL_MOUNT_DIR: RefCell<Option<PathBuf>> = RefCell::new(None);
 }

 pub fn get_mount_dir() -> PathBuf {
     THREAD_LOCAL_MOUNT_DIR.with(|local| {
         local.borrow().clone().unwrap_or_else(|| GLOBAL_MOUNT_DIR.read().unwrap().clone())
     })
 }

 pub fn set_thread_local_mount_dir(path: impl AsRef<Path>) {
     THREAD_LOCAL_MOUNT_DIR.with(|local| {
         *local.borrow_mut() = Some(path.as_ref().to_path_buf());
     });
 }

 pub fn unset_thread_local_mount_dir() {
     THREAD_LOCAL_MOUNT_DIR.with(|local| {
         *local.borrow_mut() = None;
     });
 }

 pub fn set_global_mount_dir(path: impl AsRef<Path>) {
     let mut global_dir = GLOBAL_MOUNT_DIR.write().unwrap();
     *global_dir = path.as_ref().to_path_buf();
 }

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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Position {
    /// 0-indexed line number.
    #[schema(example = 10)]
    pub line: u32,
    /// 0-indexed character index.
    #[schema(example = 5)]
    pub character: u32,
}

/// Specific position within a file.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilePosition {
    #[schema(example = "src/main.py")]
    pub path: String,
    pub position: Position,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileRange {
    /// The path to the file.
    #[schema(example = "src/main.py")]
    pub path: String,
    /// The start position of the range.
    pub start: Position,
    /// The end position of the range.
    pub end: Position,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct CodeContext {
    pub range: FileRange,
    pub source_code: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Symbol {
    /// The name of the symbol.
    #[schema(example = "User")]
    pub name: String,
    /// The kind of the symbol (e.g., function, class).
    #[schema(example = "class")]
    pub kind: String,

    /// The start position of the symbol's identifier.
    pub start_position: FilePosition,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetDefinitionRequest {
    pub position: FilePosition,

    /// Whether to include the source code around the symbol's identifier in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_source_code: bool,

    /// Whether to include the raw response from the langserver in the response.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub include_raw_response: bool,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetReferencesRequest {
    pub start_position: FilePosition,

    /// Whether to include the declaration (definition) of the symbol in the response.
    /// Defaults to false.
    #[serde(default)]
    pub include_declaration: bool,

    /// Whether to include the source code of the symbol in the response.
    /// Defaults to none.
    #[serde(default)]
    #[schema(example = 5)]
    pub include_code_context_lines: Option<u32>,

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
#[allow(unused)] // TODO re-implement using textDocument/symbol
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
///
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
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct DefinitionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition
    pub raw_response: Option<Value>,
    pub definitions: Vec<FilePosition>,
    /// The source code of symbol definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code_context: Option<Vec<CodeContext>>,
}

/// Response to a references request.
///
/// Points to the start position of the symbol's identifier.
///
/// e.g. for the references of `User` on line 0 character 6 of `src/main.py` with the code:
/// ```
/// 0: class User:
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
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferencesResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_references
    pub raw_response: Option<Value>,

    pub references: Vec<FilePosition>,

    /// The source code around the references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<CodeContext>>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct SymbolResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The raw response from the langserver.
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#workspace_symbol
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#document_symbol
    pub raw_response: Option<Value>,
    pub symbols: Vec<Symbol>,
}

impl From<(GotoDefinitionResponse, Option<Vec<CodeContext>>, bool)> for DefinitionResponse {
    fn from(
        (response, source_code_context, include_raw): (
            GotoDefinitionResponse,
            Option<Vec<CodeContext>>,
            bool,
        ),
    ) -> Self {
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
            source_code_context,
        }
    }
}

impl From<(Vec<Location>, Option<Vec<CodeContext>>, bool)> for ReferencesResponse {
    fn from(
        (locations, source_code_context, include_raw): (
            Vec<Location>,
            Option<Vec<CodeContext>>,
            bool,
        ),
    ) -> Self {
        let raw_response = if include_raw {
            Some(to_value(&locations).unwrap_or_default())
        } else {
            None
        };
        let references = locations.into_iter().map(FilePosition::from).collect();
        ReferencesResponse {
            raw_response,
            references,
            context: source_code_context,
        }
    }
}

impl From<Location> for FilePosition {
    fn from(location: Location) -> Self {
        FilePosition {
            path: uri_to_relative_path_string(&location.uri),
            position: Position {
                line: location.range.start.line,
                character: location.range.start.character,
            },
        }
    }
}

impl From<LocationLink> for FilePosition {
    fn from(link: LocationLink) -> Self {
        FilePosition {
            path: uri_to_relative_path_string(&link.target_uri),
            position: Position {
                line: link.target_range.start.line,
                character: link.target_range.start.character,
            },
        }
    }
}

impl From<SymbolInformation> for Symbol {
    fn from(symbol: SymbolInformation) -> Self {
        Symbol {
            name: symbol.name,
            kind: symbol_kind_to_string(symbol.kind).to_owned(),
            start_position: FilePosition::from(symbol.location),
        }
    }
}

impl Symbol {
    fn new(symbol: &DocumentSymbol, file_path: &str) -> Self {
        Symbol {
            name: symbol.name.to_string(),
            kind: symbol_kind_to_string(symbol.kind).to_owned(),
            start_position: FilePosition {
                path: file_path.to_owned(),
                position: Position {
                    line: symbol.selection_range.start.line,
                    character: symbol.selection_range.start.character,
                },
            },
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
                    start_position: FilePosition::from(symbol.location),
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

pub fn uri_to_relative_path_string(uri: &Url) -> String {
    let path = uri.to_file_path().unwrap_or_else(|e| {
        warn!("Failed to convert URI to file path: {:?}", e);
        PathBuf::from(uri.path())
    });

    absolute_path_to_relative_path_string(&path)
}

pub fn absolute_path_to_relative_path_string(path: &PathBuf) -> String {
    let mount_dir = get_mount_dir();
    debug!("mount_dir is {:?}", mount_dir);
    debug!("path is {:?}", path);
    path.strip_prefix(mount_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|e| {
            warn!("Failed to strip prefix: {:?}", e);
            path.to_string_lossy().into_owned()
        })
}

fn flatten_nested_symbols(symbols: Vec<DocumentSymbol>, file_path: &str) -> Vec<Symbol> {
    fn recursive_flatten(symbol: DocumentSymbol, file_path: &str, result: &mut Vec<Symbol>) {
        result.push(Symbol::new(&symbol, file_path));

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


#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{Range, SymbolKind};

    #[test]
    fn test_symbol_new_from_document_symbol() {
        let document_symbol = DocumentSymbol {
            name: "test_function".to_string(),
            detail: None,
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 15,
                    character: 1,
                },
            },
            selection_range: Range {
                start: lsp_types::Position {
                    line: 10,
                    character: 4,
                },
                end: lsp_types::Position {
                    line: 10,
                    character: 17,
                },
            },
            children: None,
        };

        let file_path = "src/main.rs";
        let symbol = Symbol::new(&document_symbol, file_path);

        assert_eq!(symbol.name, "test_function");
        assert_eq!(symbol.kind, "function");
        assert_eq!(symbol.start_position.path, file_path);
        assert_eq!(symbol.start_position.position.line, 10);
        assert_eq!(symbol.start_position.position.character, 4);
    }
}
