use std::thread;
use std::time::Duration;

use tabless::protocol::ipc::{IpcClient, IpcServer};
use tabless::protocol::parse::parse_protocol_url;
use tabless::storage::Storage;
use tabless::url::ValidatedUrl;

fn test_socket_path(name: &str) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        let tmp =
            std::env::temp_dir().join(format!("tabless-e2e-{name}-{}-test", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        tmp.join("test.sock")
    }
    #[cfg(windows)]
    {
        std::path::PathBuf::from(format!(
            r"\\.\pipe\tabless-e2e-{name}-{}",
            std::process::id()
        ))
    }
}

#[test]
fn end_to_end_single_instance_and_storage() {
    let tmpdir = std::env::temp_dir().join(format!("tabless-e2e-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmpdir);
    let socket_path = test_socket_path("protocol");
    let db_path = tmpdir.join("e2e.db");

    // First instance: bind server, store first URL, accept one forwarded URL
    let server_socket = socket_path.clone();
    let server_db = db_path.clone();
    let handle = thread::spawn(move || {
        let server = IpcServer::bind(&server_socket).unwrap();
        let storage = Storage::open(&server_db).unwrap();

        let url = parse_protocol_url("tabless://open?url=https://example.com").unwrap();
        let validated = ValidatedUrl::parse(&url).unwrap();
        storage.urls().insert(&validated, None).unwrap();

        // Accept one forwarded URL then exit
        let forwarded = server.accept_url().unwrap();
        let url2 = parse_protocol_url(&forwarded).unwrap();
        let validated2 = ValidatedUrl::parse(&url2).unwrap();
        storage.urls().insert(&validated2, None).unwrap();
    });

    thread::sleep(Duration::from_millis(100));

    // Second instance: connect, send URL
    let mut client = IpcClient::connect(&socket_path).unwrap();
    client
        .send_url("tabless://open?url=https://example.org")
        .unwrap();

    handle.join().unwrap();

    // Verify both URLs are in the database
    let storage = Storage::open(&db_path).unwrap();
    let urls = storage.urls().list_main().unwrap();
    assert_eq!(urls.len(), 2);

    let _ = std::fs::remove_dir_all(&tmpdir);
}
