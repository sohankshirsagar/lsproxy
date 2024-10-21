use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{Location, Position as LspPosition, Range};

use crate::api_types::{CodeContext, ErrorResponse, FileRange, Position, MOUNT_DIR};
use crate::api_types::{GetReferencesRequest, ReferencesResponse};
use crate::lsp::manager::{LspManager, LspManagerError};
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
    path = "/references",
    request_body = GetReferencesRequest,
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferencesResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn references(data: Data<AppState>, info: Json<GetReferencesRequest>) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.symbol_identifier_position.path,
        info.symbol_identifier_position.position.line,
        info.symbol_identifier_position.position.character
    );
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let references_result = lsp_manager
        .references(
            &info.symbol_identifier_position.path,
            LspPosition {
                line: info.symbol_identifier_position.position.line,
                character: info.symbol_identifier_position.position.character,
            },
            info.include_declaration,
        )
        .await;

    let code_contexts_result = if let Some(lines) = info.include_code_context_lines {
        match &references_result {
            Ok(refs) => fetch_code_context(&lsp_manager, refs.clone(), lines)
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
    lsp_manager: &LspManager,
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
        match lsp_manager
            .read_source_code(
                reference.uri.to_file_path().unwrap().to_str().unwrap(),
                Some(range),
            )
            .await
        {
            Ok(source_code) => {
                code_contexts.push(CodeContext {
                    source_code,
                    range: FileRange {
                        path: reference
                            .uri
                            .to_file_path()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .trim_start_matches(&format!("{}/", MOUNT_DIR))
                            .to_string(),
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
