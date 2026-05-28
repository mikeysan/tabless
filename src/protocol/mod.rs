pub mod error;
pub mod ipc;
pub mod parse;
pub mod registration;
pub mod single_instance;

pub use error::ProtocolError;
pub use single_instance::SingleInstance;

use std::path::PathBuf;

use crate::storage::Storage;
use crate::url::ValidatedUrl;

use self::ipc::IpcServer;
use self::parse::parse_protocol_url;

#[derive(Clone)]
pub struct ProtocolConfig {
    pub scheme: &'static str,
    pub binary_path: PathBuf,
    pub data_dir: PathBuf,
}

impl ProtocolConfig {
    pub fn socket_path(&self) -> PathBuf {
        #[cfg(unix)]
        {
            self.data_dir.join("tabless.ipc")
        }
        #[cfg(windows)]
        {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.data_dir.to_string_lossy().hash(&mut hasher);
            let hash = hasher.finish();
            PathBuf::from(format!(r"\\.\pipe\tabless-{hash:x}"))
        }
    }
}

pub enum RunOutcome {
    FirstInstance(IpcServer),
    UrlForwarded,
}

pub struct ProtocolHandler {
    storage: Storage,
    config: ProtocolConfig,
}

impl ProtocolHandler {
    pub fn new(config: ProtocolConfig, storage: Storage) -> Result<Self, ProtocolError> {
        Ok(ProtocolHandler { storage, config })
    }

    pub fn handle_url(&self, url: &str) -> Result<(), ProtocolError> {
        let raw = parse_protocol_url(url)?;
        let validated = ValidatedUrl::parse(&raw)
            .map_err(|e| ProtocolError::UrlValidationFailed { source: e })?;
        self.storage
            .urls()
            .insert(&validated, None)
            .map_err(|e| ProtocolError::StorageFailed { source: e })?;
        Ok(())
    }

    pub fn run(&self, protocol_url: &str) -> Result<RunOutcome, ProtocolError> {
        let socket_path = self.config.socket_path();

        match SingleInstance::new(&socket_path)? {
            SingleInstance::Subsequent(mut client) => {
                client.send_url(protocol_url)?;
                Ok(RunOutcome::UrlForwarded)
            }
            SingleInstance::First(server) => {
                self.handle_url(protocol_url)?;
                Ok(RunOutcome::FirstInstance(server))
            }
        }
    }
}
