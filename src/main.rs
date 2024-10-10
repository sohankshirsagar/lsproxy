use actix_web::{web, App, HttpServer, HttpResponse};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use git2::{Repository, BranchType};
use log::{info, error, debug};
use env_logger::Env;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod lsp_manager;
mod lsp_client;
mod types;
use crate::lsp_manager::LspManager;
use crate::types::{RepoKey, SupportedLSPs};

#[derive(OpenApi)]
#[openapi(
    paths(
        clone_repo,
        init_lsp,
    ),
    components(
        schemas(CloneRequest, RepoInfo, LspInitRequest)
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

#[derive(Deserialize, utoipa::ToSchema)]
struct LspInitRequest {
    repo_key: RepoKey,
    lsp_types: Vec<SupportedLSPs>,
}

#[derive(Serialize, Clone, utoipa::ToSchema)]
struct RepoInfo {
    repo_key: RepoKey,
    temp_dir: String,
}


struct AppState {
    clones: Mutex<HashMap<RepoKey, TempDir>>,
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

    let repo_key = RepoKey {
        id: info.id.clone(),
        github_url: info.github_url.clone(),
        branch: branch.clone(),
        commit: commit.clone(),
    };

    let repo_info = RepoInfo {
        repo_key: repo_key.clone(),
        temp_dir: temp_dir.path().to_string_lossy().into_owned(),
    };

    let mut clones = data.clones.lock().unwrap();
    clones.insert(repo_key, temp_dir);

    info!("Repository cloned successfully. ID: {}, URL: {}, Branch: {:?}, Commit: {}", 
          info.id, info.github_url, repo_info.repo_key.branch, repo_info.repo_key.commit);
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
    info!("Received LSP init request for repo: {:?}", info.repo_key);

    let temp_dir = {
        let clones = data.clones.lock().unwrap();
        match clones.get(&info.repo_key) {
            Some(dir) => dir.path().to_string_lossy().into_owned(),
            None => {
                return HttpResponse::BadRequest().body("Repository not found");
            }
        }
    };

    let result = {
        let mut lsp_manager = data.lsp_manager.lock().unwrap();
        lsp_manager.start_lsps(info.repo_key.clone(), temp_dir, &info.lsp_types).await
    };

    match result {
        Ok(_) => HttpResponse::Ok().body("LSP initialized successfully"),
        Err(e) => {
            error!("Failed to initialize LSP: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to initialize LSP: {}", e))
        },
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
            .service(web::resource("/init-lsp").route(web::post().to(init_lsp)))
    })
    .bind("0.0.0.0:8080")?;

    info!("Starting server...");
    println!("Server is about to start running...");

    server.run().await
}
