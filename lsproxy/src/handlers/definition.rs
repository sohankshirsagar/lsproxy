use crate::api_types::{
    uri_to_relative_path_string, CodeContext, ErrorResponse, FileRange, Position,
};
use crate::lsp::manager::{LspManager, LspManagerError};
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info, warn};

use crate::api_types::{DefinitionResponse, GetDefinitionRequest};
use crate::AppState;
use lsp_types::{
    DocumentSymbolResponse, GotoDefinitionResponse, Location, Position as LspPosition,
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
    tag = "symbol",
    request_body = GetDefinitionRequest,
    responses(
        (status = 200, description = "Definition retrieved successfully", body = DefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn definition(data: Data<AppState>, info: Json<GetDefinitionRequest>) -> HttpResponse {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.position.line, info.position.position.character
    );

    let lsp_manager = data
        .lsp_manager
        .lock()
        .map_err(|e| {
            error!("Failed to lock lsp_manager: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to lock lsp_manager: {}", e),
            })
        })
        .unwrap();

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
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Definition retrieval failed: {}", e),
            })
        })
        .unwrap();

    let source_code_context = if info.include_source_code {
        fetch_definition_source_code(&lsp_manager, definitions.clone())
            .await
            .map(Some)
            .unwrap_or_else(|e| {
                error!("Failed to fetch definition source code: {:?}", e);
                None
            })
    } else {
        None
    };

    HttpResponse::Ok().json(DefinitionResponse::from((
        definitions,
        source_code_context,
        info.include_raw_response,
    )))
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
        let relative_path = uri_to_relative_path_string(definition.uri.clone());
        let file_symbols = match lsp_manager.file_symbols(&relative_path).await? {
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
        let source_code = match symbol {
            Some(symbol) => {
                lsp_manager
                    .read_source_code(&relative_path, Some(symbol.range))
                    .await?
            }
            None => {
                warn!("Symbol not found for definition: {:?}", definition);
                return Err(LspManagerError::InternalError(format!(
                    "Symbol not found for definition at {:?}",
                    definition
                )));
            }
        };

        if let Some(symbol) = symbol {
            code_contexts.push(CodeContext {
                range: FileRange {
                    path: relative_path,
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
    }
    Ok(code_contexts)
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::api_types::{FilePosition, Position};
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_definition() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetDefinitionRequest {
            position: FilePosition {
                path: String::from("main.py"),
                position: Position {
                    line: 1,
                    character: 18,
                },
            },
            include_code_context_lines: Some(5),
            include_raw_response: false,
        });

        let response = definition(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let definition_response: DefinitionResponse = serde_json::from_slice(&bytes).unwrap();

        let expected_response = DefinitionResponse {
            raw_response: None,
            definitions: vec![FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 0,
                    character: 6,
                },
            }],
        };

        assert_eq!(expected_response, definition_response);
        Ok(())
    }
}
