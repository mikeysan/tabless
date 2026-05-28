use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tabless::launcher::{DefaultPlatform, Launcher, PlatformBrowser, UrlLauncher};
use tabless::protocol::ipc::IpcClient;

fn build_launcher() -> Option<Box<dyn UrlLauncher>> {
    let platform = DefaultPlatform::new();
    let discovered = platform.discover_browsers().ok()?;
    let mut launcher = Launcher::new(platform, discovered);
    // Collect defaults first to avoid borrowing launcher mutably while iterating registry.
    let defaults: Vec<_> = launcher
        .registry()
        .all_browsers()
        .into_iter()
        .filter(|info| info.is_default)
        .map(|info| info.identity.clone())
        .collect();
    for identity in defaults {
        let _ = launcher.registry_mut().set_preferred(identity);
    }
    Some(Box::new(launcher))
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
                    if shutdown_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Err(e) = handler.handle_url(&url) {
                        log::error!("IPC handle error: {}", e);
                    }
                    let _ = tx.send(());
                }
                Err(e) => {
                    log::warn!("IPC accept error: {}", e);
                }
            }
        }
    });
    (handle, shutdown)
}

fn run_gui(storage: tabless::storage::Storage, ipc_rx: Option<std::sync::mpsc::Receiver<()>>) {
    let launcher = build_launcher();
    let app = tabless::ui::app::TablessApp::new(storage, launcher, ipc_rx);
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

                let storage =
                    tabless::storage::Storage::open(&db_path).expect("failed to open storage");
                run_gui(storage, Some(rx));

                shutdown.store(true, Ordering::Relaxed);
                if let Ok(mut client) = IpcClient::connect(&socket_path) {
                    let _ = client.send_url("tabless://shutdown");
                }
                let _ = handle.join();
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

                let storage =
                    tabless::storage::Storage::open(&db_path).expect("failed to open storage");
                run_gui(storage, Some(rx));

                shutdown.store(true, Ordering::Relaxed);
                if let Ok(mut client) = IpcClient::connect(&socket_path) {
                    let _ = client.send_url("tabless://shutdown");
                }
                let _ = handle.join();
            }
            Err(e) => {
                log::error!("Single instance check failed: {}", e);
                std::process::exit(1);
            }
        }
    }
}
