use actix_web::HttpResponse;
use actix_web::web::{Json, Data};
use crate::api_types::ErrorResponse;
use log::{error, info};
use crate::lsp::manager::LspManagerError;

use lsp_types::Position;
use crate::AppState;
use crate::api_types::{DefinitionResponse, GetDefinitionRequest};
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
