use std::env;
use std::path::PathBuf;

use tabless::protocol::{ProtocolConfig, ProtocolHandler, RunOutcome};
use tabless::storage::Storage;
use tabless::ui::app::TablessApp;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "register-protocol" {
        let binary_path = env::current_exe().expect("failed to get current executable path");
        match tabless::protocol::registration::register_protocol(&binary_path) {
            Ok(()) => println!("Protocol registered successfully."),
            Err(e) => eprintln!("Registration failed: {}", e),
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
        let storage = Storage::open(&db_path).expect("failed to open storage");

        let config = ProtocolConfig {
            scheme: "tabless",
            binary_path: env::current_exe().unwrap_or_else(|_| PathBuf::from("tabless")),
            data_dir,
        };

        let handler = ProtocolHandler::new(config, storage).expect("failed to create handler");

        match handler.run(url) {
            Ok(RunOutcome::FirstInstance) => {
                // TODO: spawn GUI before starting server loop (requires IPC server on background thread)
                // Server loop blocks until interrupted
            }
            Ok(RunOutcome::UrlForwarded) => {
                // Silent exit
            }
            Err(e) => {
                eprintln!("Protocol handling failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let storage = Storage::open(&db_path).expect("failed to open storage");
        let app = TablessApp::new(storage);
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0]),
            ..Default::default()
        };
        eframe::run_native(
            "Tabless",
            options,
            Box::new(|_cc| Ok(Box::new(app) as Box<dyn eframe::App>)),
        )
        .expect("failed to run eframe");
    }
}
