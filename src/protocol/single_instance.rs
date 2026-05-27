use std::path::Path;

use super::error::ProtocolError;
use super::ipc::{IpcClient, IpcServer};

pub enum SingleInstance {
    First(IpcServer),
    Subsequent(IpcClient),
}

impl SingleInstance {
    pub fn new(socket_path: &Path) -> Result<Self, ProtocolError> {
        // Try to connect first — if another instance is running, we are the client.
        match IpcClient::connect(socket_path) {
            Ok(client) => Ok(SingleInstance::Subsequent(client)),
            Err(_) => {
                // No server running; become the server.
                let server = IpcServer::bind(socket_path)?;
                Ok(SingleInstance::First(server))
            }
        }
    }
}
