use lsproxy::api_types::{FilePosition, Position, Symbol, SymbolResponse};
use reqwest;
use std::process::Command;
use std::thread;
use std::time::Duration;

fn wait_for_server(url: &str) {
    let client = reqwest::blocking::Client::new();
    for _ in 0..60 {
        // Try for 60 seconds
        if client.get(url).send().is_ok() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
    panic!("Server did not start within 60 seconds");
}

#[test]
fn test_server_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Use the sample project directory directly as the mount directory
    let mount_dir = "/mnt/lsproxy_root/sample_project/python";

    Command::new("cargo")
        .args(&["run", "--bin", "lsproxy", "--", "--mount-dir", mount_dir])
        .spawn()
        .expect("Failed to start server");

    let base_url = "http://localhost:4444";
    wait_for_server(&format!("{}/v1/workspace/list-files", base_url));

    let client = reqwest::blocking::Client::new();

    // Test workspace/list-files endpoint
    let response = client
        .get(&format!("{}/v1/workspace/list-files", base_url))
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let mut workspace_files: Vec<String> = response.json().expect("Failed to parse JSON");

    // Check if the expected files are present
    let mut expected_files = vec!["graph.py", "main.py", "search.py", "__init__.py"];
    assert_eq!(
        workspace_files.len(),
        expected_files.len(),
        "Unexpected number of files"
    );

    workspace_files.sort();
    expected_files.sort();
    assert_eq!(workspace_files, expected_files, "File lists do not match");

    // Test file_symbols endpoint
    let response = client
        .get(&format!("{}/v1/symbol/definitions-in-file", base_url))
        .query(&[("file_path", "main.py")])
        .send()
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let returned_symbols: SymbolResponse =
        serde_json::from_value(response.json().expect("Failed to parse JSON"))?;
    let expected = SymbolResponse {
        raw_response: None,
        symbols: vec![
            Symbol {
                name: String::from("graph"),
                kind: String::from("variable"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 5,
                        character: 0,
                    },
                },
            },
            Symbol {
                name: String::from("result"),
                kind: String::from("variable"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 0,
                    },
                },
            },
            Symbol {
                name: String::from("cost"),
                kind: String::from("variable"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 6,
                        character: 8,
                    },
                },
            },
            Symbol {
                name: String::from("barrier"),
                kind: String::from("variable"),
                start_position: FilePosition {
                    path: String::from("main.py"),
                    position: Position {
                        line: 10,
                        character: 4,
                    },
                },
            },
        ],
    };
    assert_eq!(returned_symbols, expected);
    Ok(())
}
