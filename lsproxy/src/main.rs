use clap::Parser;
use env_logger::Env;
use log::info;
use lsproxy::{initialize_app_state, run_server, write_openapi_to_file, api_types::set_mount_dir};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Write OpenAPI spec to file (openapi.json)
    #[arg(short, long)]
    write_openapi: bool,

    /// Set the mount directory
    #[arg(short, long, default_value = "/mnt/workspace")]
    mount_dir: String,
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

    // Set the mount directory
    set_mount_dir(&cli.mount_dir);
    info!("Mount directory set to: {}", cli.mount_dir);

    if cli.write_openapi {
        if let Err(e) = write_openapi_to_file(&PathBuf::from("openapi.json")) {
            eprintln!("Error: Failed to write the openapi.json to a file. Please see error for more details.");
            return Err(e);
        }
        return Ok(());
    }

    let app_state = initialize_app_state()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    run_server(app_state).await
}
