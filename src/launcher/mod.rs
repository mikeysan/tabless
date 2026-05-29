use std::process::Child;
use std::sync::mpsc::{Sender, channel};
use std::thread;

pub mod error;
pub mod identity;
pub mod info;
#[allow(clippy::module_inception)]
// Module name matches crate convention (launcher::launcher::Launcher).
pub mod launcher;
pub mod linux;
pub mod macos;
pub mod platform;
pub mod registry;
pub mod windows;

#[cfg(test)]
pub mod mock;

// Public re-exports for convenient access
pub use error::{DiscoveryError, LaunchError};
pub use identity::BrowserIdentity;
pub use info::BrowserInfo;
pub use launcher::Launcher;
pub use platform::PlatformBrowser;
pub use registry::BrowserRegistry;

// Platform-specific default implementation selector
#[cfg(target_os = "linux")]
pub use linux::LinuxBrowser as DefaultPlatform;

#[cfg(target_os = "macos")]
pub use macos::MacBrowser as DefaultPlatform;

#[cfg(target_os = "windows")]
pub use windows::WindowsBrowser as DefaultPlatform;

/// Reaps browser child processes on a dedicated background thread.
///
/// When a `Launcher` is dropped, its `ChildReaper` is dropped, the channel
/// closes, and the background thread exits cleanly after draining any pending
/// children.
pub struct ChildReaper {
    tx: Sender<Child>,
}

impl ChildReaper {
    pub fn new() -> Self {
        let (tx, rx) = channel::<Child>();
        thread::spawn(move || {
            for mut child in rx {
                if let Err(e) = child.wait() {
                    log::warn!("Browser process wait failed: {}", e);
                }
            }
        });
        Self { tx }
    }

    pub fn submit(&self, child: Child) {
        let _ = self.tx.send(child);
    }
}

impl Default for ChildReaper {
    fn default() -> Self {
        Self::new()
    }
}

pub trait UrlLauncher: Send + Sync {
    fn launch(&self, url: &str) -> Result<(), LaunchError>;
    fn launch_with_identity(&self, url: &str, identity: BrowserIdentity)
    -> Result<(), LaunchError>;
}

impl<P: PlatformBrowser> UrlLauncher for Launcher<P> {
    fn launch(&self, url: &str) -> Result<(), LaunchError> {
        let child = self.launch(url)?;
        self.reaper.submit(child);
        Ok(())
    }

    fn launch_with_identity(
        &self,
        url: &str,
        identity: BrowserIdentity,
    ) -> Result<(), LaunchError> {
        let child = self.launch_with_identity(&identity, url)?;
        self.reaper.submit(child);
        Ok(())
    }
}
