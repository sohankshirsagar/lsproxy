use crate::api_types::{get_mount_dir, SupportedLanguages};
use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    PyrightClient, RustAnalyzerClient, TypeScriptLanguageClient, PYRIGHT_FILE_PATTERNS,
    RUST_ANALYZER_FILE_PATTERNS, TYPESCRIPT_FILE_PATTERNS,
};
use crate::lsp::DEFAULT_EXCLUDE_PATTERNS;
use crate::utils::file_utils::search_files;
use log::{debug, warn};
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, Location, Position};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::workspace_documents::WorkspaceDocuments;

pub struct LspManager {
    clients: HashMap<SupportedLanguages, Arc<Mutex<Box<dyn LspClient>>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Detects the languages in the workspace by searching for files that match the language server's file patterns, before LSPs are started.
    fn detect_languages_in_workspace(&self, root_path: &str) -> Vec<SupportedLanguages> {
        let mut lsps = Vec::new();
        for lsp in [
            SupportedLanguages::Python,
            SupportedLanguages::TypeScriptJavaScript,
            SupportedLanguages::Rust,
        ] {
            let patterns = match lsp {
                SupportedLanguages::Python => PYRIGHT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::TypeScriptJavaScript => TYPESCRIPT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::Rust => RUST_ANALYZER_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
            };
            if search_files(
                Path::new(root_path),
                patterns,
                DEFAULT_EXCLUDE_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            )
            .map_err(|e| warn!("Error searching files: {}", e))
            .unwrap_or_default()
            .len()
                > 0
            {
                lsps.push(lsp);
            }
        }
        debug!("Starting LSPs: {:?}", lsps);
        lsps
    }

    pub async fn start_langservers(&mut self, workspace_path: &str) -> Result<(), String> {
        let lsps = self.detect_languages_in_workspace(workspace_path);
        for lsp in lsps {
            if self.get_client(lsp).is_some() {
                continue;
            }
            debug!("Starting {:?} LSP", lsp);
            let mut client: Box<dyn LspClient> = match lsp {
                SupportedLanguages::Python => Box::new(
                    PyrightClient::new(workspace_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::TypeScriptJavaScript => Box::new(
                    TypeScriptLanguageClient::new(workspace_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Rust => Box::new(
                    RustAnalyzerClient::new(workspace_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
            };
            client
                .initialize(workspace_path.to_string())
                .await
                .map_err(|e| e.to_string())?;
            client
                .setup_workspace(workspace_path)
                .await
                .map_err(|e| e.to_string())?;
            self.clients.insert(lsp, Arc::new(Mutex::new(client)));
        }
        Ok(())
    }

    pub async fn file_symbols(
        &self,
        file_path: &str,
    ) -> Result<DocumentSymbolResponse, LspManagerError> {
        // Check if the file_path is included in the workspace files
        let workspace_files = self.workspace_files().await?;
        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = self.detect_language(full_path_str)?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;
        locked_client
            .text_document_symbols(full_path_str)
            .await
            .map_err(|e| LspManagerError::InternalError(format!("Symbol retrieval failed: {}", e)))
    }

    pub async fn definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, LspManagerError> {
        let workspace_files = self.workspace_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()).into());
        }
        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = self.detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;
        locked_client
            .text_document_definition(full_path_str, position)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Definition retrieval failed: {}", e))
            })
    }

    pub fn get_client(
        &self,
        lsp_type: SupportedLanguages,
    ) -> Option<Arc<Mutex<Box<dyn LspClient>>>> {
        self.clients.get(&lsp_type).cloned()
    }

    pub async fn references(
        &self,
        file_path: &str,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, LspManagerError> {
        let workspace_files = self.workspace_files().await.map_err(|e| {
            LspManagerError::InternalError(format!("Workspace file retrieval failed: {}", e))
        })?;
        if !workspace_files.iter().any(|f| f == file_path) {
            return Err(LspManagerError::FileNotFound(file_path.to_string()));
        }
        let full_path = get_mount_dir().join(&file_path);
        let full_path_str = full_path.to_str().unwrap_or_default();
        let lsp_type = self.detect_language(full_path_str).map_err(|e| {
            LspManagerError::InternalError(format!("Language detection failed: {}", e))
        })?;
        let client = self
            .get_client(lsp_type)
            .ok_or(LspManagerError::LspClientNotFound(lsp_type))?;
        let mut locked_client = client.lock().await;

        locked_client
            .text_document_reference(full_path_str, position, include_declaration)
            .await
            .map_err(|e| {
                LspManagerError::InternalError(format!("Reference retrieval failed: {}", e))
            })
    }

    pub async fn workspace_files(&self) -> Result<Vec<String>, LspManagerError> {
        let mut files = Vec::new();
        let mount_dir = get_mount_dir().to_string_lossy().into_owned();
        for client in self.clients.values() {
            let mut locked_client = client.lock().await;
            files.extend(
                locked_client
                    .get_workspace_documents()
                    .list_files()
                    .await
                    .iter()
                    .filter_map(|f| {
                        f.strip_prefix(&mount_dir)
                            .map(|p| p.strip_prefix('/').unwrap_or(p))
                            .map(|p| p.to_string())
                    })
                    .collect::<Vec<String>>(),
            );
        }
        Ok(files)
    }

    fn detect_language(&self, file_path: &str) -> Result<SupportedLanguages, LspManagerError> {
        let path: PathBuf = PathBuf::from(file_path);
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("py") => Ok(SupportedLanguages::Python),
            Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
                Ok(SupportedLanguages::TypeScriptJavaScript)
            }
            Some("rs") => Ok(SupportedLanguages::Rust),
            _ => Err(LspManagerError::UnsupportedFileType(file_path.to_string())),
        }
    }
}

#[derive(Debug)]
pub enum LspManagerError {
    FileNotFound(String),
    LspClientNotFound(SupportedLanguages),
    InternalError(String),
    UnsupportedFileType(String),
}

impl fmt::Display for LspManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LspManagerError::FileNotFound(path) => {
                write!(f, "File '{}' not found in workspace", path)
            }
            LspManagerError::LspClientNotFound(lang) => {
                write!(f, "LSP client not found for {:?}", lang)
            }
            LspManagerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            LspManagerError::UnsupportedFileType(path) => {
                write!(f, "Unsupported file type: {}", path)
            }
        }
    }
}

impl std::error::Error for LspManagerError {}

#[cfg(test)]
mod tests {
    use crate::api_types::test_utils::set_mount_dir;
    use crate::api_types::{FilePosition, Position, Symbol, SymbolResponse};

    use lsp_types::{Range, Url};

    use super::*;

    fn python_sample_path() -> String {
        "/mnt/lsproxy_root/sample_project/python".to_string()
    }

    async fn start_python_manager() -> Result<LspManager, Box<dyn std::error::Error>> {
        let python_path = python_sample_path();
        set_mount_dir(&python_path);

        start_manager(&python_path).await
    }

    fn reset_mount_dir() {
        set_mount_dir("/mnt/workspace");
    }

    async fn start_manager(file_path: &str) -> Result<LspManager, Box<dyn std::error::Error>> {
        let mut lsp_manager = LspManager::new();
        lsp_manager.start_langservers(file_path).await?;

        Ok(lsp_manager)
    }

    #[tokio::test]
    async fn test_start_manager_python() {
        let result = start_python_manager().await;
        assert!(
            result.is_ok(),
            "Failed to start manager: {:?}",
            result.err()
        );

        reset_mount_dir();
    }

    #[tokio::test]
    async fn test_start_manager_python_no_config() {
        let result = start_python_manager().await;
        assert!(
            result.is_ok(),
            "Failed to start manager: {:?}",
            result.err()
        );
        reset_mount_dir();
    }

    #[tokio::test]
    async fn test_workspace_files() {
        let result = start_python_manager().await;
        assert!(
            result.is_ok(),
            "Failed to start manager: {:?}",
            result.err()
        );

        let manager = result.unwrap();
        let result = manager.workspace_files().await;

        let mut expected = vec!["graph.py", "main.py", "search.py", "__init__.py"];

        assert!(
            result.is_ok(),
            "Failed to get workspace files: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().sort(), expected.sort());

        reset_mount_dir();
    }

    #[tokio::test]
    async fn test_file_symbols() {
        let result = start_python_manager().await;
        assert!(
            result.is_ok(),
            "Failed to start manager: {:?}",
            result.err()
        );
        let manager = result.unwrap();

        let file_path = "graph.py";
        let result = manager.file_symbols(file_path).await;
        assert!(result.is_ok(), "Failed to find symbols: {:?}", result.err());

        let file_symbols = result.unwrap();
        let symbol_response = SymbolResponse::from((file_symbols, file_path.to_owned(), false));

        let expected = vec![
            Symbol {
                name: String::from("AStarGraph"),
                kind: String::from("class"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 0,
                        character: 6,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("__init__"),
                kind: String::from("method"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 3,
                        character: 5,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("heuristic"),
                kind: String::from("method"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 7,
                        character: 5,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("start"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 7,
                        character: 21,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("goal"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 7,
                        character: 28,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("D"),
                kind: String::from("constant"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 10,
                        character: 2,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("D2"),
                kind: String::from("constant"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 11,
                        character: 2,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("dx"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 12,
                        character: 2,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("dy"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 13,
                        character: 2,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("get_vertex_neighbours"),
                kind: String::from("method"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 16,
                        character: 5,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("pos"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 16,
                        character: 33,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("n"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 17,
                        character: 2,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("dx"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 19,
                        character: 6,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("dy"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 19,
                        character: 10,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("x2"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 20,
                        character: 3,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("y2"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 21,
                        character: 3,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("move_cost"),
                kind: String::from("method"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 27,
                        character: 5,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("a"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 27,
                        character: 21,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("b"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 27,
                        character: 24,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("barrier"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 28,
                        character: 6,
                    },
                },
                source_code: None,
            },
            Symbol {
                name: String::from("barriers"),
                kind: String::from("variable"),
                identifier_start_position: FilePosition {
                    path: String::from("graph.py"),
                    position: Position {
                        line: 4,
                        character: 7,
                    },
                },
                source_code: None,
            },
        ];
        assert_eq!(symbol_response.symbols, expected);

        reset_mount_dir();
    }

    #[tokio::test]
    async fn test_references() {
        let result = start_python_manager().await;
        assert!(
            result.is_ok(),
            "Failed to start manager: {:?}",
            result.err()
        );
        let manager = result.unwrap();

        let file_path = "graph.py";

        let result = manager
            .references(
                file_path,
                lsp_types::Position {
                    line: 0,
                    character: 6,
                },
                false,
            )
            .await;
        assert!(result.is_ok(), "Failed to find symbols: {:?}", result.err());

        let references = result.unwrap();
        let expected = vec![
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 1,
                        character: 18,
                    },
                    end: lsp_types::Position {
                        line: 1,
                        character: 28,
                    },
                },
            },
            Location {
                uri: Url::parse("file:///mnt/lsproxy_root/sample_project/python/main.py").unwrap(),
                range: Range {
                    start: lsp_types::Position {
                        line: 5,
                        character: 8,
                    },
                    end: lsp_types::Position {
                        line: 5,
                        character: 18,
                    },
                },
            },
        ];
        assert_eq!(references, expected);
    }
}
