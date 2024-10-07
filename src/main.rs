use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use actix_files::NamedFile;
use actix_cors::Cors;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tempfile::TempDir;
use git2::{Repository, BranchType};
use log::{info, error, debug};

#[derive(Deserialize)]
struct CloneRequest {
    id: String,
    github_url: String,
    reference: Option<String>,
}

struct AppState {
    clones: Mutex<HashMap<String, HashMap<String, TempDir>>>,
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

    if let Some(ref_name) = &info.reference {
        debug!("Checking out reference: {}", ref_name);
        if let Err(e) = checkout_reference(&repo, ref_name) {
            error!("Failed to checkout specified reference: {}", e);
            return HttpResponse::BadRequest().body("Failed to checkout specified reference");
        }
    }

    let mut clones = data.clones.lock().unwrap();
    clones
        .entry(info.id.clone())
        .or_insert_with(HashMap::new)
        .insert(info.github_url.clone(), temp_dir);

    info!("Repository cloned successfully. ID: {}, URL: {}", info.id, info.github_url);
    HttpResponse::Ok().body(format!("Repository cloned successfully. ID: {}, URL: {}", info.id, info.github_url))
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

async fn index() -> impl Responder {
    NamedFile::open_async("./index.html").await.unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
