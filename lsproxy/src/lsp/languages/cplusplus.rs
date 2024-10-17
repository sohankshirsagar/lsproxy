use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::{
    lsp::{JsonRpcHandler, LspClient, ProcessHandler},
    utils::file_utils::search_directory_for_string,
};

pub struct ClangdClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

pub const CPP_ROOT_FILES: &[&str] = &["makefile"];
pub const CPP_FILE_PATTERNS: &[&str] = &["**/*.cpp", "**/*.cc", "**/*.c", "**/*.cxx"];

#[async_trait]
impl LspClient for ClangdClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        CPP_ROOT_FILES.iter().map(|&s| s.to_owned()).collect()
    }
}

impl ClangdClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        //Clangd requires a compile_commands.json to be present. On the one hand, this makes configuration easier,
        //on the other hand, if there's no compile_commands.json present we need to create one.
        if !Path::new(&format!("{}/compile_commands.json", root_path)).exists() {
            //if there's a makefile, run that.
            if Path::new(&format!("{}/makefile", root_path)).exists() {
                Command::new("compiledb")
                    .arg("-n")
                    .arg("make")
                    .current_dir(root_path)
                    .spawn()
                    .expect("Couldn't compiledb to generate compile_commands");
            } else {
                //if there's not a makefile, try to detect main and use that for compile_commands generation
                let rp = Path::new(root_path);
                let mut inc_pats = Vec::new();
                CPP_FILE_PATTERNS
                    .iter()
                    .for_each(|fp| inc_pats.push(fp.to_string()));
                match search_directory_for_string(rp, inc_pats, Vec::new(), "int main".to_string())
                {
                    Ok(files) => {
                        println!("Found files for clangd: {:?}, just using the first one for main compilation",files);
                        match files[0].file_stem() {
                            Some(stem) => {
                                Command::new("compiledb")
                                    .arg("-n")
                                    .arg("make")
                                    .arg(stem)
                                    .current_dir(root_path)
                                    .spawn()
                                    .expect("Couldn't compiledb to generate compile_commands");
                            }
                            None => {
                                println!("For some reason, the first file we found doesn't have a name. This will cause an error.")
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        let process = Command::new("clangd")
            .arg("--stdio")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
        })
    }
}
