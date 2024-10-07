use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use actix_files::NamedFile;
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::TempDir;
use git2::{Repository, BranchType};
use log::{info, error, debug};
use env_logger::Env;

#[derive(Deserialize)]
struct CloneRequest {
    id: String,
    github_url: String,
    reference: Option<String>,
}

#[derive(Serialize, Clone)]
struct RepoInfo {
    id: String,
    github_url: String,
    branch: Option<String>,
    commit: String,
}

struct AppState {
    clones: Mutex<HashMap<String, HashMap<String, (TempDir, RepoInfo)>>>,
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

async fn index() -> impl Responder {
    NamedFile::open_async("./index.html").await.unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    info!("Starting server at http://127.0.0.1:8080");

    let app_state = web::Data::new(AppState {
        clones: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/", web::get().to(index))
            .route("/clone", web::post().to(clone_repo))
            .route("/list", web::get().to(list_repos))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
