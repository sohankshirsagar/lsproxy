use std::{error::Error, os::unix::fs::PermissionsExt, path::Path, process::Stdio};


use async_trait::async_trait;
use glob::glob;
use log::debug;
use lsp_types::{InitializeResult, TextDocumentItem, Url};
use notify_debouncer_mini::DebouncedEvent;
use tokio::{process::Command, sync::broadcast::Receiver};
use futures::stream::FuturesUnordered;
use futures_util::StreamExt;

use crate::{
   lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
   utils::{
       file_utils::search_files,
       workspace_documents::{
           DidOpenConfiguration, WorkspaceDocumentsHandler, DEFAULT_EXCLUDE_PATTERNS,
           JAVA_FILE_PATTERNS, JAVA_ROOT_FILES,
       },
   },
};

pub struct JdtlsClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for JdtlsClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        JAVA_ROOT_FILES.iter().map(|&s| s.to_string()).collect()
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);
        self.start_response_listener().await?;

        let mut params = self.get_initialize_params(root_path.clone()).await?;
        params.initialization_options = Some(serde_json::json!({
            "bundles": [],
            // Setting this to root uri triggers dependency resolution which takes a long time for large repos
            // This is needed for things like code completion and other things we don't care about.
            // So we set it to an empty array to avoid doing this.
            // Getting definitions and references only needs the java files to be indexed which is done in setup_workspace
            "workspaceFolders": [""],
        }));
        let result = self
            .send_request("initialize", Some(serde_json::to_value(params)?))
            .await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;
        debug!("Initialization successful: {:?}", init_result);
        self.send_initialized().await?;
        Ok(init_result)
    }

    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Setting up Java workspace at path: {}", root_path);
        let start_time = std::time::Instant::now();

        let files = search_files(
            Path::new(root_path),
            JAVA_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            true
        ).unwrap_or_default();

        let all_files = files.clone();
        debug!("Found {} Java files to process", all_files.len());

        // First, read all files in parallel without using workspace_documents directly
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
        let mut read_futures = FuturesUnordered::new();

        for file_path in all_files {
            let path_buf = std::path::PathBuf::from(&file_path);
            let semaphore_clone = semaphore.clone();
 
            read_futures.push(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                match tokio::fs::read_to_string(&path_buf).await {
                    Ok(content) => Some((path_buf, content)),
                    Err(_) => None,
                }
            });
        }

        // Collect results as they complete
        let mut document_items = Vec::new();
        while let Some(result) = read_futures.next().await {
            if let Some((path_buf, content)) = result {
                if let Ok(uri) = Url::from_file_path(&path_buf) {
                    document_items.push(TextDocumentItem {
                        uri,
                        language_id: "java".to_string(),
                        version: 1,
                        text: content,
                    });
                    debug!("Prepared file for indexing: {}", path_buf.display());
                }
            }
        }

        // log time took to read files
        let elapsed = start_time.elapsed();
        debug!("Time took to read {} files: {:?}", document_items.len(), elapsed);

        debug!("Finished reading {} files, now opening them in the LSP", document_items.len());

        // Process files in batches to avoid overwhelming the server
        const BATCH_SIZE: usize = 100;
        let total_batches = (document_items.len() + BATCH_SIZE - 1) / BATCH_SIZE;
        for (batch_index, chunk) in document_items.chunks(BATCH_SIZE).enumerate() {
            self.text_document_did_open_batch(chunk.to_vec()).await?;
            debug!("Opened batch {} of {} ({} files)",
                batch_index + 1,
                total_batches,
                chunk.len()
            );
            // wait for 0.5 seconds
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // Give the server some time to process these files
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let elapsed2 = start_time.elapsed();
        debug!("Java setup_workspace completed in {:.2} seconds", elapsed2.as_secs_f64());
        Ok(())
    }
}

impl JdtlsClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let workspace_dir = Path::new("/usr/src/app/jdtls_workspace");

        // Delete the workspace directory if it exists
        // Doing this so we can start fresh when testing locally
        if workspace_dir.exists() {
            debug!("Deleting existing JDTLS workspace directory");
            tokio::fs::remove_dir_all(&workspace_dir).await?;
        }

        // Create a fresh workspace directory
        debug!("Creating fresh JDTLS workspace directory");
        tokio::fs::create_dir_all(&workspace_dir).await?;
        tokio::fs::set_permissions(&workspace_dir, PermissionsExt::from_mode(0o777)).await?;

        // Find the launcher jar dynamically
        let launcher_pattern = "/opt/jdtls/plugins/org.eclipse.equinox.launcher_*.jar";
        let launcher_path = match glob(launcher_pattern)
            .map_err(|e| format!("Failed to read glob pattern: {}", e))?
            .next()
        {
            Some(Ok(path)) => path,
            Some(Err(e)) => return Err(format!("Error reading launcher jar path: {}", e).into()),
            None => {
                return Err(format!(
                    "No launcher jar found matching pattern: {}",
                    launcher_pattern
                )
                .into())
            }
        };

        debug!("Using launcher jar: {:?}", launcher_path);

        let process = Command::new("java")
            .arg("-Declipse.application=org.eclipse.jdt.ls.core.id1")
            .arg("-Dosgi.bundles.defaultStartLevel=4")
            .arg("-Declipse.product=org.eclipse.jdt.ls.core.product")
            .arg("-Dlog.protocol=true")
            .arg("-Dlog.level=ALL")
            .arg("-Xmx1g")
            .arg("--add-modules=ALL-SYSTEM")
            .arg("--add-opens")
            .arg("java.base/java.util=ALL-UNNAMED")
            .arg("--add-opens")
            .arg("java.base/java.lang=ALL-UNNAMED")
            .arg("-jar")
            .arg(launcher_path)
            .arg("-configuration")
            .arg("/opt/jdtls/config_linux")
            .arg("-data")
            .arg(workspace_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "Failed to spawn Java process: {}",
                    e
                ))
            })?;

        let process_handler = ProcessHandler::new(process).await.map_err(|e| {
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Failed to create ProcessHandler: {}",
                e
            ))
        })?;

        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            JAVA_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
            DidOpenConfiguration::Lazy,
        );

        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests: PendingRequests::new(),
        })
    }
}
