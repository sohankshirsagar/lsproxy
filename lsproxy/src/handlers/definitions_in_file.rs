use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use log::{error, info};
use lsp_types::{DocumentSymbol, DocumentSymbolResponse, Range};

use crate::api_types::{CodeContext, ErrorResponse, FileRange, Position};
use crate::api_types::{FileSymbolsRequest, SymbolResponse};
use crate::lsp::manager::LspManagerError;
use crate::AppState;
/// Get symbols in a specific file
///
/// Returns a list of symbols (functions, classes, variables, etc.) defined in the specified file.
///
/// The returned positions point to the start of the symbol's identifier.
///
/// e.g. for `User` on line 0 of `src/main.py`:
/// ```
/// 0: class User:
/// _________^
/// 1:     def __init__(self, name, age):
/// 2:         self.name = name
/// 3:         self.age = age
/// ```
#[utoipa::path(
    get,
    path = "/symbol/definitions-in-file",
    tag = "symbol",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = SymbolResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn definitions_in_file(
    data: Data<AppState>,
    info: Query<FileSymbolsRequest>,
) -> HttpResponse {
    info!("Received get_symbols request for file: {}", info.file_path);
    let manager = match data.manager.lock() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to acquire lock on LSP manager: {}", e);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Internal server error".to_string(),
            });
        }
    };
    let result = manager.definitions_in_file(&info.file_path).await;

    let source_code_context = if info.include_source_code {
        match result {
            Ok(DocumentSymbolResponse::Flat(ref _symbols)) => {
                error!("Source code context not supported for flat response");
                None
            }
            Ok(DocumentSymbolResponse::Nested(ref symbols)) => {
                let ranges = collect_ranges(symbols);

                let mut context = Vec::new();
                for range in ranges {
                    if let Ok(source_code) = manager
                        .read_source_code(&info.file_path, Some(range.clone()))
                        .await
                    {
                        context.push(CodeContext {
                            range: FileRange {
                                path: info.file_path.clone(),
                                start: Position {
                                    line: range.start.line,
                                    character: range.start.character,
                                },
                                end: Position {
                                    line: range.end.line,
                                    character: range.end.character,
                                },
                            },
                            source_code,
                        });
                    }
                }
                Some(context)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(SymbolResponse::from((
            symbols,
            info.file_path.to_owned(),
            info.include_raw_response,
            source_code_context,
        ))),
        Err(e) => match e {
            LspManagerError::FileNotFound(path) => HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("File not found: {}", path),
            }),
            LspManagerError::LspClientNotFound(lang) => {
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("LSP client not found for {:?}", lang),
                })
            }
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
        },
    }
}

fn collect_ranges(symbols: &[DocumentSymbol]) -> Vec<Range> {
    symbols
        .iter()
        .flat_map(|s| {
            let mut ranges = vec![s.range.clone()];
            if let Some(children) = &s.children {
                ranges.extend(collect_ranges(children));
            }
            ranges
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::api_types::{FilePosition, Position, Symbol};
    use crate::initialize_app_state;
    use crate::test_utils::{python_sample_path, TestContext};

    #[tokio::test]
    async fn test_python_file_symbols() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        let state = initialize_app_state().await?;

        let mock_request = Query(FileSymbolsRequest {
            file_path: String::from("main.py"),
            include_raw_response: false,
            include_source_code: false,
        });

        let response = definitions_in_file(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let file_symbols_response: SymbolResponse = serde_json::from_slice(&bytes).unwrap();

        let expected_response = SymbolResponse {
            raw_response: None,
            symbols: vec![
                Symbol {
                    name: String::from("graph"),
                    kind: String::from("variable"),
                    start_position: FilePosition {
                        path: String::from("main.py"),
                        position: Position {
                            line: 5,
                            character: 0,
                        },
                    },
                },
                Symbol {
                    name: String::from("result"),
                    kind: String::from("variable"),
                    start_position: FilePosition {
                        path: String::from("main.py"),
                        position: Position {
                            line: 6,
                            character: 0,
                        },
                    },
                },
                Symbol {
                    name: String::from("cost"),
                    kind: String::from("variable"),
                    start_position: FilePosition {
                        path: String::from("main.py"),
                        position: Position {
                            line: 6,
                            character: 8,
                        },
                    },
                },
                Symbol {
                    name: String::from("barrier"),
                    kind: String::from("variable"),
                    start_position: FilePosition {
                        path: String::from("main.py"),
                        position: Position {
                            line: 10,
                            character: 4,
                        },
                    },
                },
            ],
            source_code_context: None,
        };

        assert_eq!(expected_response, file_symbols_response);
        Ok(())
    }
}
