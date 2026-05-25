use std::process::{Child, Command};
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Represents a running Node.js server process
pub struct ServerProcess {
    child: Child,
}

/// Get the path to the bundled server resources
fn get_server_dir(app: &AppHandle) -> PathBuf {
    // In development, use the project root
    // In production, use the bundled resource directory
    if cfg!(debug_assertions) {
        let mut path = std::env::current_dir().unwrap();
        // If we're in src-tauri, go up one level
        if path.ends_with("src-tauri") {
            path.pop();
        }
        path
    } else {
        app.path()
            .resource_dir()
            .expect("Failed to get resource dir")
    }
}

/// Start the Node.js server process
pub async fn start_server(app: &AppHandle, port: u16) -> Result<ServerProcess, String> {
    let server_dir = get_server_dir(app);
    let server_js = server_dir.join("server.js");

    if !server_js.exists() {
        return Err(format!("server.js not found at: {}", server_js.display()));
    }

    // Determine the data root (app data directory)
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let data_root = data_dir.to_string_lossy().to_string();

    // Find the node binary
    let node_cmd = find_node_binary();

    let child = Command::new(&node_cmd)
        .arg(&server_js)
        .arg("--port")
        .arg(port.to_string())
        .arg("--dataRoot")
        .arg(&data_root)
        .arg("--disableCsrf")
        .arg("--browserLaunchEnabled")
        .arg("false")
        .current_dir(&server_dir)
        .env("NODE_ENV", "production")
        .spawn()
        .map_err(|e| format!("Failed to start Node.js server (cmd: {}): {}", node_cmd, e))?;

    // Wait for the server to become available
    let url = format!("http://127.0.0.1:{}", port);
    let max_retries = 30;
    for i in 0..max_retries {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Ok(resp) = reqwest::get(&url).await {
            if resp.status().is_success() || resp.status().is_redirection() {
                break;
            }
        }
        if i == max_retries - 1 {
            eprintln!("Warning: Server may not be fully ready after {}s", max_retries / 2);
        }
    }

    Ok(ServerProcess { child })
}

/// Stop the server process
pub fn stop_server(mut process: ServerProcess) {
    let _ = process.child.kill();
    let _ = process.child.wait();
}

/// Find the Node.js binary path
fn find_node_binary() -> String {
    // Check common locations
    let candidates = if cfg!(target_os = "windows") {
        vec![
            "node.exe".to_string(),
            r"C:\Program Files\nodejs\node.exe".to_string(),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "node".to_string(),
            "/usr/local/bin/node".to_string(),
            "/opt/homebrew/bin/node".to_string(),
        ]
    } else {
        vec![
            "node".to_string(),
            "/usr/bin/node".to_string(),
            "/usr/local/bin/node".to_string(),
        ]
    };

    // Try to find node in PATH first
    for candidate in &candidates {
        if Command::new(candidate).arg("--version").output().is_ok() {
            return candidate.clone();
        }
    }

    // Default fallback
    "node".to_string()
}
