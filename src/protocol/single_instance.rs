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

#[cfg(test)]
mod tests {
    use super::SingleInstance;
    use std::path::PathBuf;

    fn test_socket_path(name: &str) -> PathBuf {
        #[cfg(unix)]
        {
            let tmp =
                std::env::temp_dir().join(format!("tabless-si-{name}-{}-test", std::process::id()));
            let _ = std::fs::create_dir_all(&tmp);
            tmp.join("test.sock")
        }
        #[cfg(windows)]
        {
            PathBuf::from(format!(
                r"\\.\pipe\tabless-si-{name}-{}",
                std::process::id()
            ))
        }
    }

    #[test]
    fn first_instance_succeeds_when_no_server() {
        let socket = test_socket_path("first");
        let _ = std::fs::remove_file(&socket);

        let result = SingleInstance::new(&socket);
        assert!(result.is_ok(), "expected first instance to succeed");
        assert!(matches!(result.unwrap(), SingleInstance::First(_)));
    }

    #[test]
    fn second_instance_detects_existing_server() {
        let socket = test_socket_path("second");
        let _ = std::fs::remove_file(&socket);

        // First instance binds the server
        let first = SingleInstance::new(&socket).unwrap();
        let server = match first {
            SingleInstance::First(s) => s,
            _ => panic!("expected first instance"),
        };

        // Second instance should connect as client
        let second = SingleInstance::new(&socket).unwrap();
        assert!(
            matches!(second, SingleInstance::Subsequent(_)),
            "expected second instance to be Subsequent"
        );

        // Keep server alive until end of test to avoid spurious errors
        drop(server);
    }
}
