mod config;
mod lan_ip;
mod server;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::Serialize;
use tauri::Manager;

use server::{build_router, random_token, ServerState};

pub struct AppState {
    pub server: Arc<ServerState>,
    pub lan_ip: String,
    pub port: u16,
}

#[derive(Serialize)]
struct ServerInfo {
    url: String,
    lan_ip: String,
    port: u16,
    token: String,
    dir: String,
}

#[tauri::command]
fn get_server_info(state: tauri::State<'_, AppState>) -> ServerInfo {
    let st = state.server.clone();
    let token = st.token.clone();
    let dir = st.dir.read().unwrap().clone();
    ServerInfo {
        url: format!("http://{}:{}/t/{}/", state.lan_ip, state.port, token),
        lan_ip: state.lan_ip.clone(),
        port: state.port,
        token,
        dir: dir.to_string_lossy().to_string(),
    }
}

#[tauri::command]
fn set_dir(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    dir: String,
) -> Result<(), String> {
    let path = PathBuf::from(&dir);
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    *state.server.dir.write().unwrap() = path.clone();
    let app_config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    config::save(&app_config_dir, &path)
}

#[tauri::command]
fn open_dir(dir: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";
    std::process::Command::new(program)
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())
        .map(|_| ())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&app_config_dir)?;

            let cfg = config::load(&app_config_dir);
            let dir = std::env::var("AIRBOX_DIR")
                .map(PathBuf::from)
                .unwrap_or(cfg.dir);
            let token = std::env::var("AIRBOX_TOKEN").unwrap_or_else(|_| random_token(8));

            let server = Arc::new(ServerState {
                token,
                dir: RwLock::new(dir),
            });
            let router = build_router(server.clone());

            let port_override: Option<u16> = std::env::var("AIRBOX_PORT")
                .ok()
                .and_then(|p| p.parse().ok());
            let addr = ("0.0.0.0", port_override.unwrap_or(0));

            let port = tauri::async_runtime::block_on(async {
                let listener = tokio::net::TcpListener::bind(addr)
                    .await
                    .expect("绑定端口失败，请检查防火墙");
                let port = listener.local_addr().unwrap().port();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = axum::serve(listener, router).await {
                        eprintln!("[airbox] HTTP 服务异常退出: {e}");
                    }
                });
                port
            });

            let lan_ip = lan_ip::detect_lan_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "127.0.0.1".to_string());
            app.manage(AppState {
                server,
                lan_ip,
                port,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_server_info, set_dir, open_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
