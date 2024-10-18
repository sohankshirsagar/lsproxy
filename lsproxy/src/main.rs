use actix_cors::Cors;
use actix_web::{
    web::{get, post, resource, scope, Data, Json, Query},
    App, HttpResponse, HttpServer,
};
use api_types::ErrorResponse;
use clap::Parser;
use env_logger::Env;
use log::{error, info};
use lsp::manager::LspManagerError;
use lsp_types::Position;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod api_types;
mod lsp;
mod utils;

use crate::api_types::{
    DefinitionResponse, FilePosition, FileSymbolsRequest, GetDefinitionRequest,
    GetReferencesRequest, ReferencesResponse, SupportedLanguages, Symbol, SymbolResponse,
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
        definition,
        references,
        workspace_files
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
            Symbol,
            ErrorResponse
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
async fn definition(data: Data<AppState>, info: Json<GetDefinitionRequest>) -> HttpResponse {
    info!(
        "Received definition request for file: {}, line: {}, character: {}",
        info.position.path, info.position.line, info.position.character
    );

    match data.lsp_manager.lock() {
        Ok(lsp_manager) => {
            match lsp_manager
                .definition(
                    &info.position.path,
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
async fn file_symbols(data: Data<AppState>, info: Query<FileSymbolsRequest>) -> HttpResponse {
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

/// Find all references to a symbol
///
/// The input position should point to the identifier of the symbol you want to get the references for.
///
/// Returns a list of locations where the symbol at the given position is referenced.
///
/// The returned positions point to the start of the reference identifier.
///
/// e.g. for `User` on line 0 of `src/main.py`:
/// ```
///  0: class User:
///  input____^^^^
///  1:     def __init__(self, name, age):
///  2:         self.name = name
///  3:         self.age = age
///  4:
///  5: user = User("John", 30)
///  output____^
/// ```
#[utoipa::path(
    post,
    path = "/references",
    request_body = GetReferencesRequest,
    responses(
        (status = 200, description = "References retrieved successfully", body = ReferencesResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn references(data: Data<AppState>, info: Json<GetReferencesRequest>) -> HttpResponse {
    info!(
        "Received references request for file: {}, line: {}, character: {}",
        info.symbol_identifier_position.path,
        info.symbol_identifier_position.line,
        info.symbol_identifier_position.character
    );
    let lsp_manager = data.lsp_manager.lock().unwrap();
    let result = lsp_manager
        .references(
            &info.symbol_identifier_position.path,
            Position {
                line: info.symbol_identifier_position.line,
                character: info.symbol_identifier_position.character,
            },
            info.include_declaration,
        )
        .await;
    match result {
        Ok(references) => HttpResponse::Ok().json(ReferencesResponse::from((
            references,
            info.include_raw_response,
        ))),
        Err(e) => {
            error!("Failed to get references: {}", e);
            match e {
                LspManagerError::FileNotFound(path) => {
                    HttpResponse::BadRequest().json(ErrorResponse {
                        error: format!("File not found: {}", path),
                    })
                }
                LspManagerError::LspClientNotFound(lang) => HttpResponse::InternalServerError()
                    .body(format!("LSP client not found for {:?}", lang)),
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
        .ok();
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
                    .service(resource("/definition").route(post().to(definition)))
                    .service(resource("/references").route(post().to(references)))
                    .service(resource("/workspace-files").route(get().to(workspace_files))),
            )
    })
    .bind("0.0.0.0:4444")?;

    info!("Starting server...");
    server.run().await
}
