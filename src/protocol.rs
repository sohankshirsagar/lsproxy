use lsp_types::{
    DidOpenTextDocumentParams, DocumentSymbolParams, GotoDefinitionParams, InitializeParams,
    TextDocumentItem, TextDocumentPositionParams, Url,
};
use serde_json::Value;

pub struct LspProtocol;

impl LspProtocol {
    pub fn initialize_params(root_path: Option<String>) -> InitializeParams {
        let mut params = InitializeParams {
            capabilities: Default::default(),
            ..Default::default()
        };

        if let Some(path) = root_path {
            params.workspace_folders = Some(vec![lsp_types::WorkspaceFolder {
                uri: Url::from_file_path(path).unwrap(),
                name: "workspace".to_string(),
            }]);
        }

        params
    }

    pub fn did_open_params(item: TextDocumentItem) -> Value {
        serde_json::to_value(DidOpenTextDocumentParams {
            text_document: item,
        })
        .unwrap()
    }

    pub fn goto_definition_params(file_path: &str, line: u32, character: u32) -> Value {
        serde_json::to_value(GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).unwrap(),
                },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .unwrap()
    }

    pub fn document_symbol_params(file_path: &str) -> Value {
        serde_json::to_value(DocumentSymbolParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: Url::from_file_path(file_path).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .unwrap()
    }
}
