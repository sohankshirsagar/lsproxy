use crate::api_types::ErrorResponse;
use crate::lsp::manager::LspManagerError;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::{error, info};

use crate::api_types::{DefinitionResponse, GetDefinitionRequest};
use crate::AppState;
use lsp_types::Position;
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

    match data.lsp_manager.lock() {
        Ok(lsp_manager) => {
            match lsp_manager
                .definition(
                    &info.position.path,
                    Position {
                        line: info.position.position.line,
                        character: info.position.position.character,
                    },
                )
                .await
            {
                Ok(definitions) => HttpResponse::Ok().json(DefinitionResponse::from((
                    definitions,
                    info.include_raw_response,
                ))),
                Err(e) => match e {
                    LspManagerError::FileNotFound(path) => {
                        HttpResponse::BadRequest().json(format!("File not found: {}", path))
                    }
                    LspManagerError::LspClientNotFound(lang) => HttpResponse::InternalServerError()
                        .json(ErrorResponse {
                            error: format!("LSP client not found for {:?}", lang),
                        }),
                    LspManagerError::InternalError(msg) => HttpResponse::InternalServerError()
                        .json(ErrorResponse {
                            error: format!("Internal error: {}", msg),
                        }),
                    LspManagerError::UnsupportedFileType(path) => {
                        HttpResponse::BadRequest().json(ErrorResponse {
                            error: format!("Unsupported file type: {}", path),
                        })
                    }
                },
            }
        }
        Err(e) => {
            error!("Failed to lock lsp_manager: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to lock lsp_manager".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use actix_web::http::StatusCode;

    use crate::api_types::{Position, FilePosition};
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
                }
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
            definitions: vec![FilePosition {path: String::from("graph.py"), position: Position {line:0, character: 6 }}],
        };

        assert_eq!(expected_response, definition_response);
        Ok(())
    }
}
