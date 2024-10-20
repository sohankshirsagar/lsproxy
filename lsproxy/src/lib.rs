use actix_cors::Cors;
use actix_web::{
    web::{get, post, resource, scope, Data},
    App, HttpServer,
};
use api_types::{ErrorResponse, Position};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api_types;
mod handlers;
mod lsp;
mod utils;

use crate::api_types::{
    DefinitionResponse, FilePosition, FileSymbolsRequest, GetDefinitionRequest,
    GetReferencesRequest, ReferencesResponse, SupportedLanguages, Symbol, SymbolResponse, get_mount_dir
};
use crate::handlers::{definition, file_symbols, references, workspace_files};
use crate::lsp::manager::LspManager;

pub fn check_mount_dir() -> std::io::Result<()> {
    fs::read_dir(get_mount_dir())?;
    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::file_symbols,
        crate::handlers::definition,
        crate::handlers::references,
        crate::handlers::workspace_files
    ),
    components(
        schemas(
            FileSymbolsRequest,
            GetDefinitionRequest,
            GetReferencesRequest,
            SupportedLanguages,
            DefinitionResponse,
            ReferencesResponse,
            SymbolResponse,
            FilePosition,
            Position,
            Symbol,
            ErrorResponse,
        )
    ),
    tags(
        (name = "lsproxy-api", description = "LSP Proxy API")
    ),
    servers(
        (url = "/v1", description = "API v1")
    )
)]
pub struct ApiDoc;

pub struct AppState {
    lsp_manager: Arc<Mutex<LspManager>>,
}

pub async fn initialize_app_state() -> Data<AppState> {
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    lsp_manager
        .lock()
        .unwrap()
        .start_langservers(get_mount_dir().to_str().unwrap())
        .await
        .ok();
    Data::new(AppState { lsp_manager })
}

pub async fn run_server(app_state: Data<AppState>) -> std::io::Result<()> {
    // Check if mount dir exists and is mounted
    if let Err(e) = check_mount_dir() {
        eprintln!("Error: Your workspace isn't mounted at '{}'. Please mount your workspace at this location in your docker run or docker compose commands.", get_mount_dir().to_string_lossy());
        return Err(e);
    }

    let openapi = ApiDoc::openapi();
    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(
                scope("/v1")
                    .service(resource("/file-symbols").route(get().to(file_symbols)))
                    .service(resource("/definition").route(post().to(definition)))
                    .service(resource("/references").route(post().to(references)))
                    .service(resource("/workspace-files").route(get().to(workspace_files))),
            )
    })
    .bind("0.0.0.0:4444")?
    .run()
    .await
}

pub fn write_openapi_to_file(file_path: &PathBuf) -> std::io::Result<()> {
    let openapi = ApiDoc::openapi();
    let openapi_json =
        serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI to JSON");
    let mut file = File::create(file_path)?;
    file.write_all(openapi_json.as_bytes())?;
    println!("OpenAPI spec written to: {}", file_path.display());
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_openapi_json() -> Result<(), Box<dyn std::error::Error>> {
        // Create a new temporary directory
        let temp_dir = TempDir::new()?;
        let temp_openapi_path = temp_dir.path().join("openapi.json");

        // Write the OpenAPI spec to the temporary file
        write_openapi_to_file(&PathBuf::from(&temp_openapi_path))?;

        // Read the content of the generated file
        let generated_content = fs::read_to_string(&temp_openapi_path)
            .map_err(|e| format!("Failed to load generate openapi spec: {}", e))?;

        // Read the content of the existing file
        // Assume you have a known good file to compare against
        let existing_path = PathBuf::from("/mnt/lsproxy_root/openapi.json");
        let existing_content = fs::read_to_string(existing_path)
            .map_err(|e| format!("Failed to load existing openapi spec: {}", e))?;

        // Compare the contents
        if generated_content != existing_content {
            let diff = simple_diff(&existing_content, &generated_content);
            return Err(format!(
                "Differences: {}
                Generated OpenAPI JSON does not match existing content. Make sure to run ./scripts/generate_spec.sh",
                diff
            ).into());
        }

        Ok(())
    }

    fn simple_diff(text1: &str, text2: &str) -> String {
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();
        let mut diff = String::new();
        let mut diff_count = 0;

        for (i, (l1, l2)) in lines1.iter().zip(lines2.iter()).enumerate() {
            if l1 != l2 {
                diff_count += 1;
                if diff_count <= 3 {
                    // Show up to 3 differences
                    diff.push_str(&format!("Line {}: '{}' vs '{}'\n", i + 1, l1, l2));
                }
            }
        }

        if lines1.len() != lines2.len() {
            diff.push_str(&format!(
                "Files have different number of lines: {} vs {}\n",
                lines1.len(),
                lines2.len()
            ));
        }

        if diff_count > 3 {
            diff.push_str(&format!("... and {} more differences\n", diff_count - 3));
        }

        diff
    }
}
