use actix_web::{web, App, HttpServer, HttpResponse};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use git2::{Repository, BranchType};
use log::{info, error, debug};
use env_logger::Env;
use std::path::PathBuf;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod lsp_manager;
mod lsp_client;
use lsp_manager::LspManager;

#[derive(OpenApi)]
#[openapi(
    paths(
        clone_repo,
        list_repos,
        init_lsp,
        get_function_definition,
        get_document_symbols
    ),
    components(
        schemas(CloneRequest, RepoInfo, LspInitRequest, FunctionDefinitionRequest, SymbolRequest)
    ),
    tags(
        (name = "github-clone-api", description = "GitHub Clone API")
    )
)]
struct ApiDoc;

#[derive(Deserialize, utoipa::ToSchema)]
struct CloneRequest {
    id: String,
    github_url: String,
    reference: Option<String>,
}

#[derive(Serialize, Clone, utoipa::ToSchema)]
struct RepoInfo {
    id: String,
    github_url: String,
    branch: Option<String>,
    commit: String,
    temp_dir: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct LspInitRequest {
    id: String,
    github_url: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct FunctionDefinitionRequest {
    id: String,
    github_url: String,
    file_path: String,
    line: u32,
    character: u32,
}

#[derive(Deserialize, utoipa::ToSchema)]
struct SymbolRequest {
    id: String,
    github_url: String,
    file_path: String,
}

struct AppState {
    clones: Mutex<HashMap<String, HashMap<String, (TempDir, RepoInfo)>>>,
    lsp_manager: Arc<Mutex<LspManager>>,
}

fn get_branch_and_commit(repo: &Repository, reference: &str) -> Result<(Option<String>, String), git2::Error> {
    let obj = repo.revparse_single(reference)?;
    let commit = obj.peel_to_commit()?;
    let commit_id = commit.id().to_string();

    if let Ok(branch) = repo.find_branch(reference, BranchType::Local) {
        Ok((Some(branch.name()?.unwrap_or("").to_string()), commit_id))
    } else if let Ok(branch) = repo.find_branch(reference, BranchType::Remote) {
        Ok((Some(branch.name()?.unwrap_or("").to_string()), commit_id))
    } else {
        let branches = repo.branches(None)?;
        let branch = branches
            .filter_map(|b| b.ok())
            .find(|(branch, _)| branch.get().target() == Some(commit.id()));
        
        match branch {
            Some((branch, _)) => Ok((branch.name()?.map(String::from), commit_id)),
            None => Ok((None, commit_id)),
        }
    }
}

#[utoipa::path(
    post,
    path = "/clone",
    request_body = CloneRequest,
    responses(
        (status = 200, description = "Repository cloned successfully", body = RepoInfo),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn clone_repo(
    data: web::Data<AppState>,
    info: web::Json<CloneRequest>,
) -> HttpResponse {
    info!("Received clone request for ID: {}, URL: {}", info.id, info.github_url);

    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Failed to create temp directory: {}", e);
            return HttpResponse::InternalServerError().body("Failed to create temp directory");
        }
    };

    let repo = match Repository::clone(&info.github_url, temp_dir.path()) {
        Ok(repo) => repo,
        Err(e) => {
            error!("Failed to clone repository: {}", e);
            return HttpResponse::BadRequest().body("Failed to clone repository");
        }
    };

    let (branch, commit) = match &info.reference {
        Some(ref_name) => {
            debug!("Checking out reference: {}", ref_name);
            match get_branch_and_commit(&repo, ref_name) {
                Ok((branch, commit)) => {
                    if let Err(e) = checkout_reference(&repo, ref_name) {
                        error!("Failed to checkout specified reference: {}", e);
                        return HttpResponse::BadRequest().body("Failed to checkout specified reference");
                    }
                    (branch, commit)
                },
                Err(e) => {
                    error!("Failed to get branch and commit info: {}", e);
                    return HttpResponse::BadRequest().body("Failed to get branch and commit info");
                }
            }
        },
        None => {
            match repo.head() {
                Ok(head) => {
                    match head.peel_to_commit() {
                        Ok(commit_obj) => {
                            let commit = commit_obj.id().to_string();
                            (head.shorthand().map(String::from), commit)
                        },
                        Err(e) => {
                            error!("Failed to get commit: {}", e);
                            return HttpResponse::InternalServerError().body("Failed to get commit");
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to get repository head: {}", e);
                    return HttpResponse::InternalServerError().body("Failed to get repository head");
                }
            }
        }
    };

    let repo_info = RepoInfo {
        id: info.id.clone(),
        github_url: info.github_url.clone(),
        branch,
        commit,
        temp_dir: temp_dir.path().to_string_lossy().into_owned(),
    };

    let mut clones = data.clones.lock().unwrap();
    clones
        .entry(info.id.clone())
        .or_insert_with(HashMap::new)
        .insert(info.github_url.clone(), (temp_dir, repo_info.clone()));

    info!("Repository cloned successfully. ID: {}, URL: {}, Branch: {:?}, Commit: {}", 
          info.id, info.github_url, repo_info.branch, repo_info.commit);
    HttpResponse::Ok().json(repo_info)
}

fn checkout_reference(repo: &Repository, reference: &str) -> Result<(), git2::Error> {
    if let Ok(oid) = repo.revparse_single(reference).and_then(|obj| obj.peel_to_commit()) {
        repo.checkout_tree(&oid.as_object(), None)?;
        repo.set_head_detached(oid.id())?;
    } else {
        let branch = repo.find_branch(reference, BranchType::Remote)?;
        let commit = branch.get().peel_to_commit()?;
        repo.checkout_tree(&commit.as_object(), None)?;
        repo.set_head(branch.get().name().unwrap())?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/list",
    responses(
        (status = 200, description = "List of cloned repositories", body = Vec<RepoInfo>)
    )
)]
async fn list_repos(data: web::Data<AppState>) -> HttpResponse {
    let clones = data.clones.lock().unwrap();
    let repo_list: Vec<RepoInfo> = clones
        .iter()
        .flat_map(|(_, repos)| {
            repos.values().map(|(_, repo_info)| repo_info.clone())
        })
        .collect();

    HttpResponse::Ok().json(repo_list)
}

#[utoipa::path(
    post,
    path = "/init-lsp",
    request_body = LspInitRequest,
    responses(
        (status = 200, description = "LSP server initialized successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn init_lsp(
    data: web::Data<AppState>,
    info: web::Json<LspInitRequest>,
) -> HttpResponse {
    info!("Received LSP init request for ID: {}, URL: {}", info.id, info.github_url);

    let clones = data.clones.lock().unwrap();
    let repo_map = match clones.get(&info.id) {
        Some(map) => map,
        None => {
            return HttpResponse::BadRequest().body("No repositories found for the given ID");
        }
    };

    let (_, repo_info) = match repo_map.get(&info.github_url) {
        Some(entry) => entry,
        None => {
            return HttpResponse::BadRequest().body("Repository not found");
        }
    };

    let mut lsp_manager = data.lsp_manager.lock().unwrap();
    match lsp_manager.start_lsp(info.id.clone(), info.github_url.clone(), repo_info.temp_dir.clone()) {
        Ok(_) => HttpResponse::Ok().body("LSP initialized successfully"),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to initialize LSP: {}", e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting main function");

    // Set up panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Server panicked: {:?}", panic_info);
    }));

    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("debug"));
    info!("Logger initialized");

    // Initialize app state
    info!("Initializing app state");
    let app_state = web::Data::new(AppState {
        clones: Mutex::new(HashMap::new()),
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
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi.clone())
            )
            .service(web::resource("/clone").route(web::post().to(clone_repo)))
            .service(web::resource("/list").route(web::get().to(list_repos)))
            .service(web::resource("/init-lsp").route(web::post().to(init_lsp)))
            .service(web::resource("/function-definition").route(web::post().to(get_function_definition)))
            .service(web::resource("/document-symbols").route(web::post().to(get_document_symbols)))
    })
    .bind("0.0.0.0:8080")?;

    info!("Starting server...");
    println!("Server is about to start running...");

    server.run().await
}

#[utoipa::path(
    post,
    path = "/function-definition",
    request_body = FunctionDefinitionRequest,
    responses(
        (status = 200, description = "Function definition retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_function_definition(
    _data: web::Data<AppState>,
    info: web::Json<FunctionDefinitionRequest>,
) -> HttpResponse {
    info!("Received function definition request for ID: {}, URL: {}", info.id, info.github_url);
    // TODO: Implement function definition retrieval
    HttpResponse::Ok().body("Function definition retrieval not implemented yet")
}

#[utoipa::path(
    post,
    path = "/document-symbols",
    request_body = SymbolRequest,
    responses(
        (status = 200, description = "Document symbols retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_document_symbols(
    _data: web::Data<AppState>,
    info: web::Json<SymbolRequest>,
) -> HttpResponse {
    info!("Received document symbols request for ID: {}, URL: {}, File: {}", info.id, info.github_url, info.file_path);
    // TODO: Implement document symbols retrieval
    HttpResponse::Ok().body("Document symbols retrieval not implemented yet")
}
