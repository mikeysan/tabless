pub mod error;
pub mod identity;
pub mod info;
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
