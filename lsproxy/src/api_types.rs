use lsp_types::{Location, LocationLink};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use strum_macros::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};

use crate::utils::file_utils::uri_to_relative_path_string;

static GLOBAL_MOUNT_DIR: LazyLock<Arc<RwLock<PathBuf>>> =
    LazyLock::new(|| Arc::new(RwLock::new(PathBuf::from("/mnt/workspace"))));

thread_local! {
    static THREAD_LOCAL_MOUNT_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub fn get_mount_dir() -> PathBuf {
    THREAD_LOCAL_MOUNT_DIR.with(|local| {
        local
            .borrow()
            .clone()
            .unwrap_or_else(|| GLOBAL_MOUNT_DIR.read().unwrap().clone())
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

/// Response returned when an API error occurs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Description of the error that occurred
    pub error: String,
}

/// Response returned by the health check endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Current status of the service ("ok" or error description)
    pub status: String,
    /// Version of the service
    pub version: String,
    /// Map of supported languages and whether they are currently available
    pub languages: HashMap<SupportedLanguages, bool>,
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
    #[serde(rename = "cpp")]
    CPP,
    #[serde(rename = "csharp")]
    CSharp,
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "golang")]
    Golang,
    #[serde(rename = "php")]
    PHP,
    #[serde(rename = "ruby")]
    Ruby,
}

/// A position within a text document, using 0-based indexing
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Position {
    /// 0-indexed line number.
    #[schema(example = 10)]
    pub line: u32,
    /// 0-indexed character index within the line.
    #[schema(example = 5)]
    pub character: u32,
}

/// A position within a specific file in the workspace
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilePosition {
    /// Path to the file, relative to the workspace root
    #[schema(example = "src/main.py")]
    pub path: String,
    /// Position within the file
    pub position: Position,
}

/// A range within a specific file, defined by start and end positions
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileRange {
    /// The path to the file.
    #[schema(example = "src/main.py")]
    pub path: String,
    /// The range within the file
    pub range: Range,
}

impl FileRange {
    pub fn contains(&self, position: FilePosition) -> bool {
        let pos = &position.position;
        self.path == position.path
            && self.range.start.line <= pos.line
            && self.range.end.line >= pos.line
            && (self.range.start.line != pos.line || self.range.start.character <= pos.character)
            && (self.range.end.line != pos.line || self.range.end.character >= pos.character)
    }
}

impl From<FileRange> for lsp_types::Range {
    fn from(range: FileRange) -> Self {
        lsp_types::Range::new(
            lsp_types::Position::from(range.range.start),
            lsp_types::Position::from(range.range.end),
        )
    }
}

impl From<Position> for lsp_types::Position {
    fn from(position: Position) -> Self {
        lsp_types::Position {
            line: position.line,
            character: position.character,
        }
    }
}

impl From<lsp_types::Position> for Position {
    fn from(position: lsp_types::Position) -> Self {
        Position {
            line: position.line,
            character: position.character,
        }
    }
}

/// A reference to a symbol along with its definition(s) found in the workspace
///
/// e.g. for a reference to `User` in `main.py`:
/// ```python
/// user = User("John", 30)
/// _______^
/// ```
/// This would contain:
/// - The reference location and name ("User" at line 0)
/// - The symbol definition(s) (e.g. "class User" in models.py)
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferenceWithSymbolDefinitions {
    pub reference: Identifier,
    pub definitions: Vec<Symbol>,
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
    pub identifier_position: FilePosition,

    /// The full range of the symbol.
    pub file_range: FileRange,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Identifier {
    pub name: String,
    pub file_range: FileRange,
    pub kind: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetDefinitionRequest {
    pub position: FilePosition,

    /// Whether to include the source code around the symbol's identifier in the response.
    /// Defaults to false.
    /// TODO: Implement this
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
    pub identifier_position: FilePosition,

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

/// Request to get all symbols that are referenced from a symbol at the given position, either
/// focusing on function calls, or more permissively finding all references
///
/// The input position must point to a symbol (e.g. function name, class name, variable name).
/// The response will include all symbols that are referenced from that input symbol.
/// For example, if the position points to a function name, the response will include
/// all symbols referenced within that function's implementation.
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct GetReferencedSymbolsRequest {
    /// Whether to use the more permissive rules to find referenced symbols. This will be not just
    /// code that is executed but also things like type hints and chained indirection.
    /// Defaults to false.
    #[serde(default)]
    #[schema(example = false)]
    pub full_scan: bool,

    /// The identifier position of the symbol to find references within
    pub identifier_position: FilePosition,
}

/// Request to get the symbols in a file.
#[derive(Deserialize, ToSchema, IntoParams)]
pub struct FileSymbolsRequest {
    /// The path to the file to get the symbols for, relative to the root of the workspace.
    #[schema(example = "src/main.py")]
    pub file_path: String,
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
    /// The identifier that was "clicked-on" to get the definition.
    pub selected_identifier: Identifier,
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
    /// The identifier that was "clicked-on" to get the references.
    pub selected_identifier: Identifier,
}

/// Response containing symbols referenced from the requested position
///
/// The symbols are categorized into:
/// - workspace_symbols: References to symbols that were found and have definitions in the workspace
/// - external_symbols: References to symbols from outside the workspace (built-in functions, external libraries)
/// - not_found: References where the symbol definition could not be found
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReferencedSymbolsResponse {
    pub workspace_symbols: Vec<ReferenceWithSymbolDefinitions>,
    pub external_symbols: Vec<Identifier>,
    pub not_found: Vec<Identifier>,
}

pub type SymbolResponse = Vec<Symbol>;

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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FindIdentifierRequest {
    /// The name of the identifier to search for.
    #[schema(example = "User")]
    pub name: String,
    /// The path to the file to search for identifiers.
    #[schema(example = "src/main.py")]
    pub path: String,
    /// The position hint to search for identifiers. If not provided.
    pub position: Option<Position>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IdentifierResponse {
    pub identifiers: Vec<Identifier>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, ToSchema)]
pub struct Range {
    /// The start position of the range.
    pub start: Position,
    /// The end position of the range.
    pub end: Position,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReadSourceCodeRequest {
    /// Path to the file, relative to the workspace root
    #[schema(example = "src/main.py")]
    pub path: String,
    /// Optional range within the file to read
    pub range: Option<Range>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OpenJavaFilesRequest {
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OpenJavaFilesResponse {
    pub message: String,
    pub opened_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_multi_line_range() {
        let range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 12,
                    character: 10,
                },
            },
        };

        // Test positions within the range
        assert!(
            range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 11,
                    character: 0
                }
            }),
            "middle line should be contained"
        );
        assert!(
            range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 10,
                    character: 5
                }
            }),
            "start position should be contained"
        );
        assert!(
            range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 12,
                    character: 10
                }
            }),
            "end position should be contained"
        );
    }

    #[test]
    fn test_contains_multi_line_range_outside_positions() {
        let range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 12,
                    character: 10,
                },
            },
        };

        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 9,
                    character: 0
                }
            }),
            "line before start should not be contained"
        );
        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 13,
                    character: 0
                }
            }),
            "line after end should not be contained"
        );
        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 10,
                    character: 4
                }
            }),
            "position before start on first line should not be contained"
        );
        assert!(
            !range.contains(FilePosition {
                path: range.path.clone(),
                position: Position {
                    line: 12,
                    character: 11
                }
            }),
            "position after end on last line should not be contained"
        );
    }

    #[test]
    fn test_contains_single_line_range() {
        let single_line_range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 10,
                },
            },
        };

        assert!(
            single_line_range.contains(FilePosition {
                path: single_line_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 7
                }
            }),
            "position within single line range should be contained"
        );
        assert!(
            !single_line_range.contains(FilePosition {
                path: single_line_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 4
                }
            }),
            "position before single line range should not be contained"
        );
        assert!(
            !single_line_range.contains(FilePosition {
                path: single_line_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 11
                }
            }),
            "position after single line range should not be contained"
        );
    }

    #[test]
    fn test_contains_zero_width_range() {
        let zero_width_range = FileRange {
            path: "test.rs".to_string(),
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 5,
                },
            },
        };

        assert!(
            zero_width_range.contains(FilePosition {
                path: zero_width_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 5
                }
            }),
            "position at zero-width range should be contained"
        );
        assert!(
            !zero_width_range.contains(FilePosition {
                path: zero_width_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 4
                }
            }),
            "position before zero-width range should not be contained"
        );
        assert!(
            !zero_width_range.contains(FilePosition {
                path: zero_width_range.path.clone(),
                position: Position {
                    line: 10,
                    character: 6
                }
            }),
            "position after zero-width range should not be contained"
        );
    }
}
