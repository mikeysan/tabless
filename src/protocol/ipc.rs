use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use interprocess::local_socket::{
    ConnectOptions, GenericFilePath, ListenerOptions, ToFsName,
    prelude::{LocalSocketListener, LocalSocketStream},
    traits::Listener,
};

use super::error::ProtocolError;

pub struct IpcServer {
    listener: LocalSocketListener,
    #[cfg(unix)]
    socket_path: std::path::PathBuf,
}

#[cfg(unix)]
impl Drop for IpcServer {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.socket_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::debug!("IPC socket already removed: {}", e);
            }
            Err(e) => {
                log::warn!("Failed to remove IPC socket: {}", e);
            }
            Ok(()) => {}
        }
    }
}

impl IpcServer {
    pub fn bind(socket_path: &Path) -> Result<Self, ProtocolError> {
        #[cfg(unix)]
        let _ = std::fs::remove_file(socket_path);

        let name = socket_path.to_fs_name::<GenericFilePath>().map_err(|e| {
            ProtocolError::IpcBindFailed {
                reason: e.to_string(),
            }
        })?;

        let listener = ListenerOptions::new()
            .name(name)
            .create_sync()
            .map_err(|e| ProtocolError::IpcBindFailed {
                reason: e.to_string(),
            })?;

        Ok(Self {
            listener,
            #[cfg(unix)]
            socket_path: socket_path.to_path_buf(),
        })
    }

    pub fn accept_url(&self) -> Result<String, ProtocolError> {
        let stream = self
            .listener
            .accept()
            .map_err(|e| ProtocolError::IpcBindFailed {
                reason: e.to_string(),
            })?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| ProtocolError::IpcBindFailed {
                reason: e.to_string(),
            })?;

        line.strip_prefix("URL:")
            .map(|s| s.trim_end().to_string())
            .ok_or_else(|| ProtocolError::InvalidUrl {
                reason: "missing URL: prefix".to_string(),
            })
    }
}

pub struct IpcClient {
    stream: LocalSocketStream,
}

impl IpcClient {
    pub fn connect(socket_path: &Path) -> Result<Self, ProtocolError> {
        let name = socket_path.to_fs_name::<GenericFilePath>().map_err(|e| {
            ProtocolError::IpcConnectFailed {
                reason: e.to_string(),
            }
        })?;

        let stream = ConnectOptions::new()
            .name(name)
            .connect_sync()
            .map_err(|e| ProtocolError::IpcConnectFailed {
                reason: e.to_string(),
            })?;

        Ok(Self { stream })
    }

    pub fn send_url(&mut self, url: &str) -> Result<(), ProtocolError> {
        writeln!(self.stream, "URL:{}", url).map_err(|e| ProtocolError::IpcConnectFailed {
            reason: e.to_string(),
        })?;
        self.stream
            .flush()
            .map_err(|e| ProtocolError::IpcConnectFailed {
                reason: e.to_string(),
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::thread;

    fn test_socket_path(name: &str) -> PathBuf {
        #[cfg(unix)]
        {
            let tmp =
                std::env::temp_dir().join(format!("tabless-{name}-{}-test", std::process::id()));
            let _ = std::fs::create_dir_all(&tmp);
            tmp.join("test.sock")
        }
        #[cfg(windows)]
        {
            PathBuf::from(format!(r"\\.\pipe\tabless-{name}-{}", std::process::id()))
        }
    }

    #[test]
    #[cfg(unix)]
    fn drop_removes_socket_file() {
        let socket_path = test_socket_path("drop");

        {
            let server = IpcServer::bind(&socket_path).unwrap();
            assert!(socket_path.exists());
            drop(server);
        }

        assert!(!socket_path.exists());
    }

    #[test]
    fn roundtrip_url() {
        let socket_path = test_socket_path("roundtrip");

        let server = IpcServer::bind(&socket_path).unwrap();
        let expected_url = "https://example.com/test";

        let handle = thread::spawn(move || server.accept_url().unwrap());

        let mut client = IpcClient::connect(&socket_path).unwrap();
        client.send_url(expected_url).unwrap();

        let received = handle.join().unwrap();
        assert_eq!(received, expected_url);
    }

    #[test]
    fn ipc_delivery_persists_url() {
        use crate::storage::Storage;
        use crate::url::ValidatedUrl;

        let tmp_dir = std::env::temp_dir().join(format!("tabless-ipc-db-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let db_path = tmp_dir.join("test.db");
        let _ = std::fs::remove_file(&db_path);

        let socket_path = test_socket_path("delivery");

        let server = IpcServer::bind(&socket_path).unwrap();
        let db_for_thread = db_path.clone();

        let handle = std::thread::spawn(move || {
            let url = server.accept_url().unwrap();
            let storage = Storage::open(&db_for_thread).unwrap();
            let validated = ValidatedUrl::parse(&url).unwrap();
            storage.urls().insert(&validated, None).unwrap();
        });

        let mut client = IpcClient::connect(&socket_path).unwrap();
        client.send_url("https://example.com").unwrap();

        handle.join().unwrap();

        let storage = Storage::open(&db_path).unwrap();
        let urls = storage.urls().list_inbox().unwrap();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].canonical_url, "https://example.com/");
    }
}
