mod client;
mod errors;
mod json_rpc;
mod lsp_manager;
mod process;
mod symbol_finder;
mod types;
mod utils;

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer};
use env_logger::Env;
use log::{debug, error, info};
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::lsp_manager::LspManager;
use crate::types::{SupportedLSP, MOUNT_DIR};

#[derive(OpenApi)]
#[openapi(
    paths(
        start_langservers,
        get_symbols,
        get_definition,
    ),
    components(
        schemas(LspInitRequest, GetSymbolsRequest, GetDefinitionRequest, SupportedLSP)
    ),
    tags(
        (name = "lsp-adapter-api", description = "LSP Adapter API")
    )
)]
struct ApiDoc;

#[derive(Deserialize, utoipa::ToSchema)]
struct GetDefinitionRequest {
    file_path: String,
    symbol_name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct LspInitRequest {
    lsp_types: Vec<SupportedLSP>,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct GetSymbolsRequest {
    file_path: String,
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
        "Received get_definition request for file: {}, symbol: {}",
        info.file_path, info.symbol_name
    );

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager
            .get_definition(full_path_str, &info.symbol_name)
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
    path = "/get-symbols",
    request_body = GetSymbolsRequest,
    responses(
        (status = 200, description = "Symbols retrieved successfully", body = String),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_symbols(
    data: web::Data<AppState>,
    info: web::Json<GetSymbolsRequest>,
) -> HttpResponse {
    info!("Received get_symbols request for file: {}", info.file_path);

    let full_path = Path::new(&MOUNT_DIR).join(&info.file_path);
    let full_path_str = full_path.to_str().unwrap_or("");
    debug!("Full path: {}", full_path_str);

    let result = {
        let lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.get_symbols(full_path_str).await
    };

    match result {
        Ok(symbols) => HttpResponse::Ok().json(symbols),
        Err(e) => {
            error!("Failed to get symbols: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get symbols: {}", e))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting main function");
    eprintln!("This is a test error message");

    // Set up panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Server panicked: {:?}", panic_info);
    }));

    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    println!("Logger initialized");
    info!("Logger initialized");

    // Initialize app state
    info!("Initializing app state");
    let app_state = web::Data::new(AppState {
        lsp_manager: Arc::new(Mutex::new(LspManager::new())),
    });
    info!("App state initialized");

    // Generate OpenAPI documentation
    info!("Generating OpenAPI documentation");
    let openapi = ApiDoc::openapi();
    info!("OpenAPI documentation generated");

    // Initialize HTTP server
    info!("Initializing HTTP server");
    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
            .service(web::resource("/start-langservers").route(web::post().to(start_langservers)))
            .service(web::resource("/get-symbols").route(web::post().to(get_symbols)))
            .service(web::resource("/get-definition").route(web::post().to(get_definition)))
    })
    .bind("0.0.0.0:8080")?;

    info!("Starting server...");
    println!("Server is about to start running...");

    server.run().await
}
