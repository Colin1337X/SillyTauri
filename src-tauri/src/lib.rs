use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_dialog::DialogExt;

mod server;

/// State holding the server process handle
pub struct ServerState {
    pub process: Mutex<Option<server::ServerProcess>>,
    pub port: u16,
    pub data_dir: PathBuf,
}

#[derive(Clone, Serialize)]
struct ServerStartedPayload {
    port: u16,
    url: String,
}

/// Import user data from a .zip file containing a previous install's /data directory.
/// Extracts the zip contents into the app's data directory.
#[tauri::command]
async fn import_user_data(app: AppHandle, zip_path: String) -> Result<String, String> {
    let state = app.state::<ServerState>();
    let data_dir = state.data_dir.clone();

    tokio::task::spawn_blocking(move || {
        let zip_file_path = Path::new(&zip_path);
        if !zip_file_path.exists() {
            return Err("Zip file not found".to_string());
        }

        let file = fs::File::open(zip_file_path).map_err(|e| format!("Failed to open zip: {}", e))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

        // Create the data directory if it doesn't exist
        fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| format!("Failed to read entry: {}", e))?;
            let entry_path = entry.mangled_name();

            // Strip leading "data/" or "data\" prefix if present (the zip might contain data/... paths)
            let relative_path = entry_path
                .strip_prefix("data/")
                .or_else(|_| entry_path.strip_prefix("data\\"))
                .unwrap_or(&entry_path);

            let output_path = data_dir.join(relative_path);

            // Validate that the output path is within data_dir (prevent zip slip)
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
            }
            let canonical_data_dir = data_dir.canonicalize().unwrap_or_else(|_| data_dir.clone());
            if let Ok(canonical_output) = output_path.canonicalize() {
                if !canonical_output.starts_with(&canonical_data_dir) {
                    continue; // Skip entries that would escape the data directory
                }
            }

            if entry.is_dir() {
                fs::create_dir_all(&output_path).map_err(|e| format!("Failed to create dir: {}", e))?;
            } else {
                let mut outfile = fs::File::create(&output_path)
                    .map_err(|e| format!("Failed to create file {}: {}", output_path.display(), e))?;
                io::copy(&mut entry, &mut outfile)
                    .map_err(|e| format!("Failed to extract file: {}", e))?;
            }
        }

        Ok(format!("Successfully imported data to {}", data_dir.display()))
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

/// Open a file picker dialog for selecting a .zip file, then import it
#[tauri::command]
async fn pick_and_import_data(app: AppHandle) -> Result<String, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("Zip Archives", &["zip"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            let path_buf = path.into_path().map_err(|e| format!("Invalid path: {}", e))?;
            let path_str = path_buf.to_string_lossy().to_string();
            import_user_data(app, path_str).await
        }
        None => Err("No file selected".to_string()),
    }
}

/// Get the current server URL
#[tauri::command]
fn get_server_url(app: AppHandle) -> String {
    let state = app.state::<ServerState>();
    format!("http://127.0.0.1:{}", state.port)
}

pub fn run() {
    let port = portpicker::pick_unused_port().unwrap_or(8000);

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());

    // Single instance plugin only on desktop
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}));
    }

    let app = builder
        .invoke_handler(tauri::generate_handler![
            import_user_data,
            pick_and_import_data,
            get_server_url,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            // Create data directory
            fs::create_dir_all(&data_dir).expect("Failed to create app data dir");

            // Manage server state
            app.manage(ServerState {
                process: Mutex::new(None),
                port,
                data_dir: data_dir.clone(),
            });

            // Start the Node.js server
            let handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                match server::start_server(&handle, port).await {
                    Ok(process) => {
                        let state = handle.state::<ServerState>();
                        *state.process.lock().unwrap() = Some(process);

                        // Navigate the main window to the server URL
                        let url = format!("http://127.0.0.1:{}", port);
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.eval(format!("window.location.replace('{}')", url));
                        }

                        let _ = handle.emit("server-started", ServerStartedPayload {
                            port,
                            url,
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to start server: {}", e);
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.eval(format!(
                                "document.querySelector('.status').textContent = 'Error: {}'",
                                e.replace('\'', "\\'")
                            ));
                        }
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            // Clean up the server process on exit
            let state = app_handle.state::<ServerState>();
            let process = state.process.lock().unwrap().take();
            if let Some(process) = process {
                server::stop_server(process);
            }
        }
    });
}
