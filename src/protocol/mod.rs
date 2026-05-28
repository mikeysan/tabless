pub mod error;
pub mod ipc;
pub mod parse;
pub mod registration;
pub mod single_instance;

pub use error::ProtocolError;

use std::path::PathBuf;

use crate::storage::Storage;
use crate::url::ValidatedUrl;

use self::ipc::IpcServer;
use self::parse::parse_protocol_url;
use self::single_instance::SingleInstance;

#[derive(Clone)]
pub struct ProtocolConfig {
    pub scheme: &'static str,
    pub binary_path: PathBuf,
    pub data_dir: PathBuf,
}

impl ProtocolConfig {
    pub fn socket_path(&self) -> PathBuf {
        self.data_dir.join("tabless.ipc")
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
