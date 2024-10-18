use actix_web::HttpResponse;
use actix_web::web::{Data, Query};
use log::info;

use crate::api_types::{FileSymbolsRequest, SymbolResponse};
use crate::AppState;
use crate::api_types::ErrorResponse;
use crate::lsp::manager::LspManagerError;
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
    path = "/file-symbols",
    params(FileSymbolsRequest),
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = SymbolResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn file_symbols(data: Data<AppState>, info: Query<FileSymbolsRequest>) -> HttpResponse {
    info!("Received get_symbols request for file: {}", info.file_path);

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.file_symbols(&info.file_path).await
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(SymbolResponse::from((
            symbols,
            info.file_path.to_owned(),
            info.include_raw_response,
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
