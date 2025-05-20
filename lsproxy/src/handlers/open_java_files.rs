use actix_web::{web, HttpResponse};
use log::debug;
use crate::AppState;
use crate::api_types::{OpenJavaFilesRequest, OpenJavaFilesResponse, ErrorResponse};

/// Open multiple Java files in the LSP server
///
/// Accepts a list of file paths and opens them in the Java language server.
/// This is used when we cache the existing workspace dir and want to open all new/modified java files
#[utoipa::path(
    post,
    path = "/workspace/open-java-files",
    request_body = OpenJavaFilesRequest,
    responses(
        (status = 200, description = "Successfully opened Java files", body = OpenJavaFilesResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Workspace"
)]
pub async fn open_java_files(
    app_state: web::Data<AppState>,
    request: web::Json<OpenJavaFilesRequest>,
) -> HttpResponse {
    debug!("Received request to open {} Java files", request.file_paths.len());
    
    match app_state.manager.open_java_files(&request.file_paths).await {
        Ok(opened_count) => {
            HttpResponse::Ok().json(OpenJavaFilesResponse {
                message: format!("Successfully opened {} Java files", opened_count),
                opened_count,
            })
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to open Java files: {}", e),
            })
        }
    }
}
