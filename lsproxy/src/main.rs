use actix_cors::Cors;
use actix_web::{
    web::{get, resource, scope, Data, Query},
    App, HttpResponse, HttpServer,
};
use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};
use lsp_types::Position;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api_types;
mod lsp;
mod utils;

use crate::api_types::{
    DefinitionResponse, FilePosition, FileSymbolsRequest, GetDefinitionRequest,
    GetReferencesRequest, ReferenceResponse, SupportedLanguages, Symbol, SymbolResponse,
    WorkspaceSymbolsRequest, MOUNT_DIR,
};
use crate::lsp::manager::LspManager;

fn check_mount_dir() -> std::io::Result<()> {
    fs::read_dir(MOUNT_DIR)?;
    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        file_symbols,
        workspace_symbols,
        definition,
        references,
        workspace_files
    ),
    components(
        schemas(
            FileSymbolsRequest,
            WorkspaceSymbolsRequest,
            GetDefinitionRequest,
            GetReferencesRequest,
            SupportedLanguages,
            DefinitionResponse,
            ReferenceResponse,
            SymbolResponse,
            FilePosition,
            Symbol
        )
    ),
    tags(
        (name = "lsproxy-api", description = "LSP Proxy API")
    ),
    servers(
        (url = "/v1", description = "API v1")
    )
)]
struct ApiDoc;

struct AppState {
    lsp_manager: Arc<Mutex<LspManager>>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Write OpenAPI spec to file (openapi.json)
    #[arg(short, long)]
    write_openapi: bool,
}

/// Get the definition of a symbol at a specific position in a file
///
/// Returns the location of the definition for the symbol at the given position.
#[utoipa::path(
    get,
    path = "/definition",
    params(GetDefinitionRequest),
    responses(
        (status = 200, description = "Definition retrieved successfully", body = DefinitionResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn definition(data: Data<AppState>, info: Query<GetDefinitionRequest>) -> HttpResponse {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.line, info.position.character
    );

    let full_path = Path::new(&MOUNT_DIR).join(&info.position.path);
    let full_path_str = full_path.to_str().unwrap_or_default();

    match data.lsp_manager.lock() {
        Ok(lsp_manager) => {
            match lsp_manager
                .definition(
                    full_path_str,
                    Position {
                        line: info.position.line,
                        character: info.position.character,
                    },
                )
                .await
            {
                Ok(definitions) => HttpResponse::Ok().json(DefinitionResponse::from((
                    definitions,
                    info.include_raw_response,
                ))),
                Err(e) => {
                    error!("Failed to get definition: {}", e);
                    HttpResponse::InternalServerError().body(e.to_string())
                }
            }
        }
        Err(e) => {
            error!("Failed to lock lsp_manager: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Get symbols in a specific file
///
/// Returns a list of symbols (functions, classes, variables, etc.) defined in the specified file.
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
async fn file_symbols(data: Data<AppState>, info: Query<FileSymbolsRequest>) -> HttpResponse {
    info!("Received get_symbols request for file: {}", info.file_path);

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");
    debug!("Full path: {}", full_path_str);

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.file_symbols(full_path_str).await
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(SymbolResponse::from((
            symbols,
            info.file_path.to_owned(),
            info.include_raw_response,
        ))),
        Err(e) => {
            error!("Failed to get symbols: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get symbols: {}", e))
        }
    }
}

/// Search for symbols across the entire workspace
///
/// Returns a list of symbols matching the given query string from all files in the workspace.
#[utoipa::path(
    get,
    path = "/workspace-symbols",
    params(WorkspaceSymbolsRequest),
    responses(
        (status = 200, description = "Workspace symbols retrieved successfully", body = SymbolResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn workspace_symbols(
    data: Data<AppState>,
    info: Query<WorkspaceSymbolsRequest>,
) -> HttpResponse {
    info!(
        "Received workspace_symbols request for query: {}",
        info.query
    );

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.workspace_symbols(&info.query).await
    };

    match result {
        Ok(symbols) => {
            HttpResponse::Ok().json(SymbolResponse::from((symbols, info.include_raw_response)))
        }
        Err(e) => {
            error!("Failed to get workspace symbols: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to get workspace symbols: {}", e))
        }
    }
}

/// Find all references to a symbol
///
/// Returns a list of locations where the symbol at the given position is referenced.
#[utoipa::path(
    get,
    path = "/references",
    params(GetReferencesRequest),
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferenceResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn references(data: Data<AppState>, info: Query<GetReferencesRequest>) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.symbol_identifier_position.path,
        info.symbol_identifier_position.line,
        info.symbol_identifier_position.character
    );
    let full_path = Path::new(&MOUNT_DIR).join(&info.symbol_identifier_position.path);
    let full_path_str = match full_path.to_str() {
        Some(s) => s,
        None => {
            error!("Failed to convert path to string");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let result = lsp_manager
        .references(
            full_path_str,
            Position {
                line: info.symbol_identifier_position.line,
                character: info.symbol_identifier_position.character,
            },
            info.include_declaration,
        )
        .await;
    match result {
        Ok(references) => HttpResponse::Ok().json(ReferenceResponse::from((
            references,
            info.include_raw_response,
        ))),
        Err(e) => {
            error!("Failed to get references: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get references: {}", e))
        }
    }
}

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
async fn workspace_files(data: Data<AppState>) -> HttpResponse {
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let files = lsp_manager.workspace_files().await;
    match files {
        Ok(files) => HttpResponse::Ok().json(files),
        Err(e) => {
            error!("Failed to get workspace files: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to get workspace files: {}", e))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting...");
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Server panicked: {:?}", panic_info);
    }));

    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    info!("Logger initialized");

    let cli = Cli::parse();

    let openapi = ApiDoc::openapi();

    if cli.write_openapi {
        let file_path = PathBuf::from("openapi.json");
        let openapi_json = serde_json::to_string_pretty(&openapi).unwrap();
        let mut file = match File::create(&file_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create file: {}", e);
                return Err(e);
            }
        };
        if let Err(e) = file.write_all(openapi_json.as_bytes()) {
            eprintln!("Failed to write to file: {}", e);
            return Err(e);
        }
        println!("OpenAPI spec written to: {}", file_path.display());
        return Ok(());
    }

    // Check if MOUNT_DIR exists and is mounted
    if let Err(e) = check_mount_dir() {
        eprintln!("Error: Your workspace isn't mounted at '{}'. Please mount your workspace at this location in your docker run or docker compose commands.", MOUNT_DIR);
        return Err(e);
    }

    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));
    lsp_manager
        .lock()
        .unwrap()
        .start_langservers(MOUNT_DIR)
        .await
        .unwrap();
    let app_state = Data::new(AppState { lsp_manager });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(app_state.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(
                scope("/v1")
                    .service(resource("/file-symbols").route(get().to(file_symbols)))
                    .service(resource("/workspace-symbols").route(get().to(workspace_symbols)))
                    .service(resource("/definition").route(get().to(definition)))
                    .service(resource("/references").route(get().to(references)))
                    .service(resource("/workspace-files").route(get().to(workspace_files))),
            )
    })
    .bind("0.0.0.0:4444")?;

    info!("Starting server...");
    server.run().await
}
