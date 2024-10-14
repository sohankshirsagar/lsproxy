use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer};
use env_logger::Env;
use log::{debug, error, info};
use lsp_types::Position;
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod lsp;
mod utils;

use crate::lsp::manager::LspManager;
use crate::lsp::types::{SupportedLSP, MOUNT_DIR};

#[derive(OpenApi)]
#[openapi(
    paths(
        start_langservers,
        file_symbols,
        workspace_symbols,
        get_definition,
        get_references,
    ),
    components(
        schemas(LspInitRequest, FileSymbolsRequest, WorkspaceSymbolsRequest, GetDefinitionRequest, GetReferencesRequest, SupportedLSP)
    ),
    tags(
        (name = "lsp-proxy-api", description = "LSP Proxy API")
    )
)]
struct ApiDoc;

#[derive(Deserialize, utoipa::ToSchema)]
struct GetDefinitionRequest {
    file_path: String,
    line: u32,
    character: u32,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct GetReferencesRequest {
    file_path: String,
    line: u32,
    character: u32,
    include_declaration: Option<bool>,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct LspInitRequest {
    lsp_types: Vec<SupportedLSP>,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct FileSymbolsRequest {
    file_path: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct WorkspaceSymbolsRequest {
    query: String,
}

struct AppState {
    lsp_manager: Arc<Mutex<LspManager>>,
}

#[utoipa::path(
    post,
    path = "/get-definition",
    request_body = GetDefinitionRequest,
    responses(
        (status = 200, description = "Definition retrieved successfully", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_definition(
    data: web::Data<AppState>,
    info: web::Json<GetDefinitionRequest>,
) -> HttpResponse {
    info!(
        "Received get_definition request for file: {}, line: {}, character: {}",
        info.file_path, info.line, info.character
    );

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");

    let result = {
        let lsp_manager = match data.lsp_manager.lock() {
            Ok(manager) => manager,
            Err(poisoned) => {
                error!("Failed to lock lsp_manager: {:?}", poisoned);
                return HttpResponse::InternalServerError().finish();
            }
        };
        lsp_manager
            .get_definition(
                full_path_str,
                Position {
                    line: info.line,
                    character: info.character,
                },
            )
            .await
    };

    match result {
        Ok(definitions) => HttpResponse::Ok().json(definitions),
        Err(e) => {
            error!("Failed to get definition: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get definition: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/start-langservers",
    request_body = LspInitRequest,
    responses(
        (status = 200, description = "LSP server started successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn start_langservers(
    data: web::Data<AppState>,
    info: web::Json<LspInitRequest>,
) -> HttpResponse {
    info!("Received LSP init request");

    let result = {
        let mut lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager
            .start_langservers(MOUNT_DIR, &info.lsp_types)
            .await
    };

    match result {
        Ok(_) => HttpResponse::Ok().body("LSP started successfully"),
        Err(e) => {
            error!("Failed to start LSP: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to initialize LSP: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/file-symbols",
    request_body = FileSymbolsRequest,
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn file_symbols(
    data: web::Data<AppState>,
    info: web::Json<FileSymbolsRequest>,
) -> HttpResponse {
    info!("Received get_symbols request for file: {}", info.file_path);

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");
    debug!("Full path: {}", full_path_str);

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.file_symbols(full_path_str).await
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => {
            error!("Failed to get symbols: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get symbols: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/workspace-symbols",
    request_body = WorkspaceSymbolsRequest,
    responses(
        (status = 200, description = "Workspace symbols retrieved successfully", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn workspace_symbols(
    data: web::Data<AppState>,
    info: web::Json<WorkspaceSymbolsRequest>,
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
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => {
            error!("Failed to get workspace symbols: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to get workspace symbols: {}", e))
        }
    }
}

#[utoipa::path(
    post,
    path = "/references",
    request_body = GetReferencesRequest,
    responses(
        (status = 200, description = "References retrieved successfully", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_references(
    data: web::Data<AppState>,
    info: web::Json<GetReferencesRequest>,
) -> HttpResponse {
    info!(
        "Received get_references request for file: {}, line: {}, character: {}",
        let GetReferencesRequest { file_path, line, character, include_declaration } = info.into_inner();
    );
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let result = lsp_manager
        .get_references(
            &info.file_path,
            Position {
                line: info.line,
                character: info.character,
            },
            info.include_declaration.unwrap_or_else(|| {
                error!("include_declaration not provided, defaulting to true");
                true
            }),
        )
        .await;
    match result {
        Ok(references) => HttpResponse::Ok().json(references),
        Err(e) => {
            error!("Failed to get references: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get references: {}", e))
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

    let app_state = web::Data::new(AppState {
        lsp_manager: Arc::new(Mutex::new(LspManager::new())),
    });

    let openapi = ApiDoc::openapi();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .app_data(app_state.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(web::resource("/start-langservers").route(web::post().to(start_langservers)))
            .service(web::resource("/file-symbols").route(web::post().to(file_symbols)))
            .service(web::resource("/workspace-symbols").route(web::post().to(workspace_symbols)))
            .service(web::resource("/definition").route(web::post().to(get_definition)))
            .service(web::resource("/references").route(web::post().to(get_references)))
    })
    .bind("0.0.0.0:8080")?;

    info!("Starting server...");
    server.run().await
}
