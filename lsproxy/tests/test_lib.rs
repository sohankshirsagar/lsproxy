use lsproxy::{write_openapi_to_file, initialize_app_state};
use lsproxy::api_types::set_mount_dir;
use actix_web::{test, web, App};
use std::fs;
use tempfile::TempDir;
use std::path::PathBuf;

pub fn python_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/python".to_string()
}

pub fn js_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/js".to_string()
}

pub struct TestContext;

impl TestContext {
    pub async fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        set_mount_dir(file_path);
        Ok(Self {})
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        set_mount_dir("/mnt/workspace");
    }
}



