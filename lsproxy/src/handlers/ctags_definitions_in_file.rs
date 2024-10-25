use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use log::{error, info};

use crate::api_types::{ErrorResponse, FileSymbolsRequest, Symbol};
use crate::AppState;
/// Get symbols in a specific file using ctags
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
    path = "/symbol/ctags-definitions-in-file",
    tag = "symbol",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = Vec<Symbol>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn ctags_definitions_in_file(
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

    let result = manager.definitions_in_file_ctags(&info.file_path).await;

    match result {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => HttpResponse::BadRequest().json(ErrorResponse {
            error: format!("Couldn't get symbols from ctags: {}", e),
        }),
    }
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

        let response = ctags_definitions_in_file(state, mock_request).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Check the body
        let body = response.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let file_symbols_response: Vec<Symbol> = serde_json::from_slice(&bytes).unwrap();

        let expected_response = vec![
            Symbol {
                name: String::from("plt"),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 0,
                        character: 28,
                    },
                },
            },
            Symbol {
                name: String::from("graph"),
                kind: String::from("ctag_definition"),
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
                kind: String::from("ctag_definition"),
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
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 8,
                    },
                },
            },
        ];

        assert_eq!(expected_response, file_symbols_response);
        Ok(())
    }
}
