use actix_web::HttpResponse;
use actix_web::web::Data;
use log::error;

use crate::AppState;
use crate::api_types::ErrorResponse;
use crate::lsp::manager::LspManagerError;

/// Get a list of all files in the workspace
///
/// Returns an array of file paths for all files in the current workspace.
///
/// This is a convenience endpoint that does not use the underlying Language Servers directly, but it does apply the same filtering.
#[utoipa::path(
    get,
    path = "/workspace-files",
    responses(
        (status = 200, description = "Workspace files retrieved successfully", body = Vec<String>),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn workspace_files(data: Data<AppState>) -> HttpResponse {
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let files = lsp_manager.workspace_files().await;
    match files {
        Ok(files) => HttpResponse::Ok().json(files),
        Err(e) => {
            error!("Failed to get workspace files: {}", e);
            match e {
                LspManagerError::FileNotFound(path) => {
                    HttpResponse::BadRequest().json(ErrorResponse {
                        error: format!("File not found: {}", path),
                    })
                }
                LspManagerError::LspClientNotFound(lang) => HttpResponse::InternalServerError()
                    .json(ErrorResponse {
                        error: format!("LSP client not found for {:?}", lang),
                    }),
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
    }
}
