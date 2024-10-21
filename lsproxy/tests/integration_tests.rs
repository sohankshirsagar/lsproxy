use std::process::{Command, Child};
use std::time::Duration;
use std::thread;
use reqwest;
use serde_json::Value;

fn start_server(mount_dir: &str) -> Child {
    Command::new("cargo")
        .args(&["run", "--bin", "lsproxy", "--", "--mount-dir", mount_dir])
        .spawn()
        .expect("Failed to start server")
}

fn wait_for_server(url: &str) {
    let client = reqwest::blocking::Client::new();
    for _ in 0..30 {  // Try for 30 seconds
        if client.get(url).send().is_ok() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
    panic!("Server did not start within 30 seconds");
}

#[test]
fn test_server_integration() {
    // Use the sample project directory directly as the mount directory
    let mount_dir = "/mnt/lsproxy_root/sample_project/python";

    let mut server = start_server(mount_dir);
    
    let base_url = "http://localhost:4444";
    wait_for_server(&format!("{}/v1/workspace/list-files", base_url));

    let client = reqwest::blocking::Client::new();

    // Test workspace/list-files endpoint
    let response = client.get(&format!("{}/v1/workspace/list-files", base_url))
        .send()
        .expect("Failed to send request");
    
    assert_eq!(response.status(), 200);
    
    let mut workspace_files: Vec<String> = response.json().expect("Failed to parse JSON");
    
    // Check if the expected files are present
    let mut expected_files = vec!["graph.py", "main.py", "search.py", "__init__.py"];
    assert_eq!(workspace_files.len(), expected_files.len(), "Unexpected number of files");
    
    workspace_files.sort();
    expected_files.sort();
    assert_eq!(workspace_files, expected_files, "File lists do not match");

    // Test file_symbols endpoint
    let response = client.get(&format!("{}/v1/symbol/definitions-in-file", base_url))
        .query(&[("file_path", "main.py")])
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let symbols: Value = response.json().expect("Failed to parse JSON");
    assert!(symbols.as_array().unwrap().len() > 0, "No symbols returned");

    // You can add more specific checks for the symbols if needed

    // Shutdown the server
    server.kill().expect("Failed to kill server process");
}
