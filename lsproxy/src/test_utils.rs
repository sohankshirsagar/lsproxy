use crate::api_types::{set_thread_local_mount_dir, unset_thread_local_mount_dir};
use crate::lsp::manager::LspManager;

pub fn python_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/python".to_string()
}

pub fn js_sample_path() -> String {
    "/mnt/lsproxy_root/sample_project/js".to_string()
}

pub struct TestContext {
    pub manager: Option<LspManager>,
}

impl TestContext {
    pub async fn setup(file_path: &str, manager: bool) -> Result<Self, Box<dyn std::error::Error>> {
        set_thread_local_mount_dir(file_path);
        if manager {
            let mut manager = LspManager::new();
            if let Err(e) = manager.start_langservers(file_path).await {
                unset_thread_local_mount_dir();
                return Err(e);
            }
            return Ok(Self {
                manager: Some(manager),
            });
        }
        Ok(Self { manager: None })
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        unset_thread_local_mount_dir();
    }
}
