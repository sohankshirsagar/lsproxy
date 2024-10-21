use crate::api_types::{CodeContext, ErrorResponse, FileRange, Position};
use crate::lsp::manager::{LspManager, LspManagerError};
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info, warn};
use serde_json::to_value;

use crate::api_types::{DefinitionResponse, GetDefinitionRequest};
use crate::AppState;
use lsp_types::{
    DocumentSymbolResponse, GotoDefinitionResponse, Location, Position as LspPosition, Range,
};
/// Get the definition of a symbol at a specific position in a file
///
/// Returns the location of the definition for the symbol at the given position.
///
/// The input position should point inside the symbol's identifier, e.g.
///
/// The returned position points to the identifier of the symbol, and the file_path from workspace root
///
/// e.g. for the definition of `User` on line 5 of `src/main.py` with the code:
/// ```
/// 0: class User:
/// output___^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// 4:
/// 5: user = User("John", 30)
/// input_____^^^^
/// ```
#[utoipa::path(
    post,
    path = "/definition",
    request_body = GetDefinitionRequest,
    responses(
        (status = 200, description = "Definition retrieved successfully", body = DefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn definition(
    data: Data<AppState>,
    info: Json<GetDefinitionRequest>,
) -> Result<HttpResponse, ErrorResponse> {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.position.line, info.position.position.character
    );

    let lsp_manager = data.lsp_manager.lock().map_err(|e| {
        error!("Failed to lock lsp_manager: {:?}", e);
        ErrorResponse {
            error: "Failed to lock lsp_manager".to_string(),
        }
    })?;

    let definitions = lsp_manager
        .definition(
            &info.position.path,
            LspPosition {
                line: info.position.position.line,
                character: info.position.position.character,
            },
        )
        .await
        .map_err(|e| {
            error!("Definition error: {:?}", e);
            ErrorResponse {
                error: "Definition retrieval failed".to_string(),
            }
        })?;

    let source_code_context = if info.include_source_code {
        fetch_definition_source_code(&lsp_manager, definitions.clone())
            .await
            .map(Some)
            .unwrap_or_else(|e| {
                warn!("Failed to fetch definition source code: {:?}", e);
                None
            })
    } else {
        None
    };

    Ok(HttpResponse::Ok().json(DefinitionResponse::from((
        definitions,
        source_code_context,
        info.include_raw_response,
    ))))
}

async fn fetch_definition_source_code(
    lsp_manager: &LspManager,
    definitions_response: GotoDefinitionResponse,
) -> Result<Vec<CodeContext>, LspManagerError> {
    let mut code_contexts = Vec::new();
    let definitions = match definitions_response {
        GotoDefinitionResponse::Scalar(definition) => vec![definition],
        GotoDefinitionResponse::Array(definitions) => definitions,
        GotoDefinitionResponse::Link(links) => links
            .into_iter()
            .map(|link| Location::new(link.target_uri, link.target_range))
            .collect(),
    };

    for definition in definitions {
        let file_symbols = match lsp_manager
            .file_symbols(&definition.uri.to_file_path().unwrap().to_str().unwrap())
            .await?
        {
            DocumentSymbolResponse::Nested(file_symbols) => file_symbols,
            DocumentSymbolResponse::Flat(_) => {
                return Err(LspManagerError::InternalError(
                    "Flat document symbols are not supported".to_string(),
                ))
            }
        };
        let symbol = file_symbols
            .iter()
            .find(|s| s.selection_range == definition.range);

        let source_code = lsp_manager
            .read_source_code(
                definition.uri.to_file_path().unwrap().to_str().unwrap(),
                Some(definition.range),
            )
            .await?;
        match symbol {
            Some(symbol) => {
                code_contexts.push(CodeContext {
                    range: FileRange {
                        path: definition
                            .uri
                            .to_file_path()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                        start: Position {
                            line: symbol.range.start.line,
                            character: symbol.range.start.character,
                        },
                        end: Position {
                            line: symbol.range.end.line,
                            character: symbol.range.end.character,
                        },
                    },
                    source_code,
                });
            }
            None => {
                warn!("Symbol not found for definition: {:?}", definition);
                return Err(LspManagerError::InternalError(format!(
                    "Symbol not found for definition at {:?}",
                    definition
                )));
            }
        }
    }
    Ok(code_contexts)
}
