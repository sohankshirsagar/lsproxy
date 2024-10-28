use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Location, Position as LspPosition, Range};

use crate::api_types::{CodeContext, ErrorResponse, FileRange, Position};
use crate::api_types::{GetReferencesRequest, ReferencesResponse};
use crate::lsp::manager::{LspManagerError, Manager};
use crate::utils::file_utils::uri_to_relative_path_string;
use crate::AppState;

/// Find all references to a symbol
///
/// The input position should point to the identifier of the symbol you want to get the references for.
///
/// Returns a list of locations where the symbol at the given position is referenced.
///
/// The returned positions point to the start of the reference identifier.
///
/// e.g. for `User` on line 0 of `src/main.py`:
/// ```
///  0: class User:
///  input____^^^^
///  1:     def __init__(self, name, age):
///  2:         self.name = name
///  3:         self.age = age
///  4:
///  5: user = User("John", 30)
///  output____^
/// ```
#[utoipa::path(
    post,
    path = "/symbol/find-references",
    tag = "symbol",
    request_body = GetReferencesRequest,
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferencesResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn find_references(
    data: Data<AppState>,
    info: Json<GetReferencesRequest>,
) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.identifier_position.path,
        info.identifier_position.position.line,
        info.identifier_position.position.character
    );
    let manager = data.manager.lock().unwrap();
    let references_result = manager
        .find_references(
            &info.identifier_position.path,
            LspPosition {
                line: info.identifier_position.position.line,
                character: info.identifier_position.position.character,
            },
            info.include_declaration,
        )
        .await;

    let code_contexts_result = if let Some(lines) = info.include_code_context_lines {
        match &references_result {
            Ok(refs) => fetch_code_context(&manager, refs.clone(), lines)
                .await
                .map(Some)
                .map_err(|e| {
                    error!("Failed to fetch code context: {}", e);
                    e
                }),
            Err(_) => Err(LspManagerError::InternalError(
                "Failed to get references".to_string(),
            )),
        }
    } else {
        Ok(None)
    };
    match (references_result, code_contexts_result) {
        (Ok(references), Ok(code_contexts)) => HttpResponse::Ok().json(ReferencesResponse::from((
            references,
            code_contexts,
            info.include_raw_response,
        ))),
        (Err(e), _) => {
            error!("Failed to get references: {}", e);
            match e {
                LspManagerError::FileNotFound(path) => {
                    HttpResponse::BadRequest().json(ErrorResponse {
                        error: format!("File not found: {}", path),
                    })
                }
                LspManagerError::LspClientNotFound(lang) => HttpResponse::InternalServerError()
                    .body(format!("LSP client not found for {:?}", lang)),
                LspManagerError::InternalError(msg) => {
                    HttpResponse::InternalServerError().json(ErrorResponse {
                        error: format!("Internal error: {}", msg),
                    })
                }
                LspManagerError::UnsupportedFileType(path) => {
                    HttpResponse::BadRequest().json(ErrorResponse {
                        error: format!("Unsupported file type: {}", path),
                    })
                }
            }
        }
        (_, Err(e)) => {
            error!("Failed to fetch code context: {}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to fetch code context: {}", e),
            })
        }
    }
}

async fn fetch_code_context(
    manager: &Manager,
    references: Vec<Location>,
    context_lines: u32,
) -> Result<Vec<CodeContext>, LspManagerError> {
    let mut code_contexts = Vec::new();
    for reference in references {
        let range = Range {
            start: LspPosition {
                line: reference.range.start.line.saturating_sub(context_lines),
                character: 0,
            },
            end: LspPosition {
                line: reference.range.end.line.saturating_add(context_lines),
                character: 0,
            },
        };
        match manager
            .read_source_code(&uri_to_relative_path_string(&reference.uri), Some(range))
            .await
        {
            Ok(source_code) => {
                code_contexts.push(CodeContext {
                    source_code,
                    range: FileRange {
                        path: uri_to_relative_path_string(&reference.uri),
                        start: Position {
                            line: range.start.line,
                            character: 0,
                        },
                        end: Position {
                            line: range.end.line,
                            character: 0,
                        },
                    },
                });
            }
            Err(e) => return Err(e),
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
    async fn test_python_references() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Json(GetReferencesRequest {
            identifier_position: FilePosition {
                path: String::from("graph.py"),
                position: Position {
                    line: 0,
                    character: 6,
                },
            },
            include_declaration: false,
            include_code_context_lines: None,
            include_raw_response: false,
        });

        let response = find_references(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let reference_response: ReferencesResponse = serde_json::from_slice(&bytes).unwrap();

        let expected_response = ReferencesResponse {
            raw_response: None,
            references: vec![
                FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 1,
                        character: 18,
                    },
                },
                FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 5,
                        character: 8,
                    },
                },
            ],
            context: None,
        };

        assert_eq!(expected_response, reference_response);
        Ok(())
    }
}
