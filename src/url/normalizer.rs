use url::Url;

pub fn normalize(url: &Url) -> Url {
    let mut normalized = url.clone();
    if let Some(host) = normalized.host_str() {
        let lower = host.to_lowercase();
        if host != lower {
            let _ = normalized.set_host(Some(&lower));
        }
    }
    normalized
}
