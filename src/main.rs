/// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tabless::launcher::{DefaultPlatform, Launcher, PlatformBrowser, UrlLauncher};
use tabless::protocol::ipc::IpcClient;

/// Internal wire-protocol sentinel used to unblock the IPC server's
/// blocking `accept()` during graceful shutdown. This is not a
/// user-facing URL and cannot be invoked via the protocol handler.
const SHUTDOWN_SENTINEL: &str = "__TABLESS_SHUTDOWN__";

/// Ensures the IPC socket file is removed on scope exit,
/// even if the IPC thread panics or fails to shut down cleanly.
struct SocketGuard<'a>(&'a std::path::Path);

#[cfg(unix)]
impl Drop for SocketGuard<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0);
    }
}

fn build_launcher() -> (
    Option<Box<dyn UrlLauncher>>,
    Vec<tabless::launcher::BrowserIdentity>,
) {
    let platform = DefaultPlatform::new();
    // Discovery is only required for explicit browser selection.
    // Normal URL launching delegates to the OS and must succeed even when
    // discovery fails or returns no results.
    let discovered = platform.discover_browsers().unwrap_or_default();
    let identities: Vec<tabless::launcher::BrowserIdentity> = discovered
        .iter()
        .map(|info| info.identity.clone())
        .collect();
    let launcher = Launcher::new(platform, discovered);
    (Some(Box::new(launcher)), identities)
}

fn spawn_ipc_server(
    db_path: std::path::PathBuf,
    config: tabless::protocol::ProtocolConfig,
    server: tabless::protocol::ipc::IpcServer,
    tx: std::sync::mpsc::Sender<()>,
) -> (std::thread::JoinHandle<()>, Arc<AtomicBool>) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    let handle = std::thread::spawn(move || {
        let storage = match tabless::storage::Storage::open(&db_path) {
            Ok(s) => s,
            Err(e) => {
                log::error!("IPC thread failed to open storage: {}", e);
                return;
            }
        };
        let handler = match tabless::protocol::ProtocolHandler::new(config, storage) {
            Ok(h) => h,
            Err(e) => {
                log::error!("IPC thread failed to create handler: {}", e);
                return;
            }
        };
        loop {
            match server.accept_url() {
                Ok(url) => {
                    if url == SHUTDOWN_SENTINEL {
                        log::debug!("Received shutdown sentinel, exiting IPC loop");
                        break;
                    }
                    if let Err(e) = handler.handle_url(&url) {
                        log::error!("IPC handle error: {}", e);
                    }
                    let _ = tx.send(());
                    if shutdown_clone.load(Ordering::Relaxed) {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("IPC accept error: {}", e);
                }
            }
        }
    });
    (handle, shutdown)
}

/// Shut down the IPC server thread gracefully.
///
/// Connecting to the socket sends a sentinel URL to unblock the server's
/// blocking `accept()` call. The sentinel is not forwarded to the protocol handler.
fn shutdown_ipc(
    shutdown: &AtomicBool,
    socket_path: &std::path::Path,
    handle: std::thread::JoinHandle<()>,
) {
    shutdown.store(true, Ordering::Relaxed);
    if let Ok(mut client) = IpcClient::connect(socket_path) {
        if client.send_url(SHUTDOWN_SENTINEL).is_ok() {
            let _ = handle.join();
        } else {
            log::warn!("Failed to send shutdown sentinel; leaving thread to OS cleanup");
        }
    } else {
        log::warn!("Failed to connect to IPC socket for shutdown; leaving thread to OS cleanup");
    }
}

fn run_gui(storage: tabless::storage::Storage, ipc_rx: Option<std::sync::mpsc::Receiver<()>>) {
    let (launcher, browser_identities) = build_launcher();
    let app = tabless::ui::app::TablessApp::new(storage, launcher, browser_identities, ipc_rx);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tabless",
        options,
        Box::new(|_cc| Ok(Box::new(app) as Box<dyn eframe::App>)),
    )
    .expect("failed to run eframe");
}

fn main() {
    let _ = env_logger::try_init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "register-protocol" {
        let binary_path = std::env::current_exe().expect("failed to get current executable path");
        match tabless::protocol::registration::register_protocol(&binary_path) {
            Ok(()) => log::info!("Protocol registered successfully."),
            Err(e) => log::error!("Registration failed: {}", e),
        }
        return;
    }

    let protocol_url = args.get(1).filter(|s| s.starts_with("tabless://"));

    let data_dir = dirs::data_local_dir()
        .expect("failed to determine data directory")
        .join("tabless");
    std::fs::create_dir_all(&data_dir).expect("failed to create data directory");
    let db_path = data_dir.join("tabless.db");

    if let Some(url) = protocol_url {
        let storage = tabless::storage::Storage::open(&db_path).expect("failed to open storage");

        let config = tabless::protocol::ProtocolConfig {
            scheme: "tabless",
            binary_path: std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("tabless")),
            data_dir,
        };

        let handler = tabless::protocol::ProtocolHandler::new(config.clone(), storage)
            .expect("failed to create handler");

        match handler.run(url) {
            Ok(tabless::protocol::RunOutcome::FirstInstance(server)) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let socket_path = config.socket_path();
                let (handle, shutdown) = spawn_ipc_server(db_path.clone(), config, server, tx);
                let _guard = SocketGuard(&socket_path);

                let storage =
                    tabless::storage::Storage::open(&db_path).expect("failed to open storage");
                run_gui(storage, Some(rx));

                shutdown_ipc(&shutdown, &socket_path, handle);
            }
            Ok(tabless::protocol::RunOutcome::UrlForwarded) => {
                // Silent exit
            }
            Err(e) => {
                log::error!("Protocol handling failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let config = tabless::protocol::ProtocolConfig {
            scheme: "tabless",
            binary_path: std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("tabless")),
            data_dir: data_dir.clone(),
        };
        let socket_path = config.socket_path();

        match tabless::protocol::SingleInstance::new(&socket_path) {
            Ok(tabless::protocol::SingleInstance::Subsequent(_client)) => {
                // Another instance is already running; silently exit.
            }
            Ok(tabless::protocol::SingleInstance::First(server)) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let (handle, shutdown) = spawn_ipc_server(db_path.clone(), config, server, tx);
                let _guard = SocketGuard(&socket_path);

                let storage =
                    tabless::storage::Storage::open(&db_path).expect("failed to open storage");
                run_gui(storage, Some(rx));

                shutdown_ipc(&shutdown, &socket_path, handle);
            }
            Err(e) => {
                log::error!("Single instance check failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}
