use actix_cors::Cors;
use actix_web::{
    web::{get, post, resource, scope, Data},
    App, HttpServer,
};
use api_types::{CodeContext, ErrorResponse, FileRange, Position};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod api_types;
mod handlers;
mod lsp;
mod utils;

use crate::api_types::{
    get_mount_dir, DefinitionResponse, FilePosition, FileSymbolsRequest, GetDefinitionRequest,
    GetReferencesRequest, ReferencesResponse, SupportedLanguages, Symbol, SymbolResponse,
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
            CodeContext,
            FileRange,
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

pub async fn initialize_app_state() -> Result<Data<AppState>, Box<dyn std::error::Error>> {
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    lsp_manager
        .lock()
        .unwrap()
        .start_langservers(get_mount_dir().to_str().unwrap())
        .await?;
    Ok(Data::new(AppState { lsp_manager }))
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
                    .service(
                        scope("/symbol")
                            .service(resource("/definitions-in-file").route(get().to(file_symbols)))
                            .service(resource("/find-definition").route(post().to(definition)))
                            .service(resource("/find-references").route(post().to(references))),
                    )
                    .service(
                        scope("/workspace")
                            .service(resource("/list-files").route(get().to(workspace_files))),
                    ),
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
mod test_utils;

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::{js_sample_path, python_sample_path, TestContext};
    use crate::api_types::set_mount_dir;
    use tempfile::TempDir;
    use std::net::TcpStream;
    use std::time::Duration;
    use std::thread;
    use std::sync::mpsc;

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

    #[tokio::test]
    async fn test_initialize_app_python() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&python_sample_path(), false).await?;
        initialize_app_state().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_app_js() -> Result<(), Box<dyn std::error::Error>> {
        let _context = TestContext::setup(&js_sample_path(), false).await?;
        initialize_app_state().await?;
        Ok(())
    }

    #[test]
    fn test_run_server() -> Result<(), Box<dyn std::error::Error>> {
        let test_path = js_sample_path();
        let (tx, rx) = mpsc::channel();

        // Spawn the server in a separate thread
        let _server_thread = thread::spawn(move || {
            // Set the mount directory for the server thread
            set_mount_dir(&test_path);

            let system = actix_web::rt::System::new();
            if let Err(e) = system.block_on(async {
                match initialize_app_state().await {
                    Ok(app_state) => run_server(app_state).await,
                    Err(e) => {
                        tx.send(format!("Failed to initialize app state: {}", e)).unwrap();
                        Ok(())
                    }
                }
            }) {
                tx.send(format!("System error: {}", e)).unwrap();
            }
        });

        // Give the server some time to start
        thread::sleep(Duration::from_secs(5));

        // Check for any errors from the server thread
        if let Ok(error_msg) = rx.try_recv() {
            return Err(error_msg.into());
        }

        // Check if the server is running
        match TcpStream::connect("0.0.0.0:4444") {
            Ok(_) => println!("Server is running"),
            Err(e) => return Err(format!("Failed to connect to server: {}", e).into()),
        }

        Ok(())
    }
}
