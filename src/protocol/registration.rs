use std::fs;
use std::path::Path;

use super::error::ProtocolError;

pub fn register_protocol(binary_path: &Path) -> Result<(), ProtocolError> {
    #[cfg(target_os = "linux")]
    return register_linux(binary_path);

    #[cfg(target_os = "macos")]
    return register_macos(binary_path);

    #[cfg(target_os = "windows")]
    return register_windows(binary_path);

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err(ProtocolError::RegistrationFailed {
        platform: "unknown".to_string(),
        reason: "unsupported platform".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn register_linux(binary_path: &Path) -> Result<(), ProtocolError> {
    let apps_dir = dirs::data_local_dir()
        .ok_or_else(|| ProtocolError::RegistrationFailed {
            platform: "linux".to_string(),
            reason: "could not determine data local dir".to_string(),
        })?
        .join("applications");

    fs::create_dir_all(&apps_dir).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "linux".to_string(),
        reason: format!("create dir failed: {}", e),
    })?;

    let desktop_path = apps_dir.join("tabless.desktop");
    if desktop_path.exists() {
        return Err(ProtocolError::AlreadyRegistered);
    }

    let exec = binary_path.to_string_lossy();
    let desktop_entry = format!(
        "[Desktop Entry]\n\
         Name=Tabless\n\
         Exec={} %u\n\
         Type=Application\n\
         Terminal=false\n\
         MimeType=x-scheme-handler/tabless;\n",
        exec
    );

    fs::write(&desktop_path, desktop_entry).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "linux".to_string(),
        reason: format!("write .desktop failed: {}", e),
    })?;

    let status = std::process::Command::new("xdg-mime")
        .args(["default", "tabless.desktop", "x-scheme-handler/tabless"])
        .status()
        .map_err(|e| ProtocolError::RegistrationFailed {
            platform: "linux".to_string(),
            reason: format!("xdg-mime failed: {}", e),
        })?;

    if !status.success() {
        return Err(ProtocolError::RegistrationFailed {
            platform: "linux".to_string(),
            reason: "xdg-mime exited with error".to_string(),
        });
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn register_macos(binary_path: &Path) -> Result<(), ProtocolError> {
    let apps_dir = dirs::home_dir()
        .ok_or_else(|| ProtocolError::RegistrationFailed {
            platform: "macos".to_string(),
            reason: "could not determine home dir".to_string(),
        })?
        .join("Applications")
        .join("Tabless.app");

    if apps_dir.exists() {
        return Err(ProtocolError::AlreadyRegistered);
    }

    let contents = apps_dir.join("Contents");
    let macos = contents.join("MacOS");
    fs::create_dir_all(&macos).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "macos".to_string(),
        reason: format!("create dir failed: {}", e),
    })?;

    let info_plist = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         <key>CFBundleIdentifier</key>\n\
         <string>com.tabless.app</string>\n\
         <key>CFBundleName</key>\n\
         <string>Tabless</string>\n\
         <key>CFBundleExecutable</key>\n\
         <string>tabless-wrapper</string>\n\
         <key>CFBundleURLTypes</key>\n\
         <array>\n\
         <dict>\n\
         <key>CFBundleURLName</key>\n\
         <string>Tabless URL</string>\n\
         <key>CFBundleURLSchemes</key>\n\
         <array>\n\
         <string>tabless</string>\n\
         </array>\n\
         </dict>\n\
         </array>\n\
         </dict>\n\
         </plist>\n"
        .to_string();

    fs::write(contents.join("Info.plist"), info_plist).map_err(|e| {
        ProtocolError::RegistrationFailed {
            platform: "macos".to_string(),
            reason: format!("write Info.plist failed: {}", e),
        }
    })?;

    let wrapper_script = format!(
        "#!/bin/sh\nexec \"{}\" \"$1\"\n",
        binary_path.to_string_lossy()
    );

    let wrapper_path = macos.join("tabless-wrapper");
    fs::write(&wrapper_path, wrapper_script).map_err(|e| ProtocolError::RegistrationFailed {
        platform: "macos".to_string(),
        reason: format!("write wrapper failed: {}", e),
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&wrapper_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&wrapper_path, perms).map_err(|e| {
            ProtocolError::RegistrationFailed {
                platform: "macos".to_string(),
                reason: format!("chmod wrapper failed: {}", e),
            }
        })?;
    }

    let _ = std::process::Command::new("lsregister")
        .arg("-f")
        .arg(&apps_dir)
        .status();

    Ok(())
}

#[cfg(target_os = "windows")]
fn register_windows(binary_path: &Path) -> Result<(), ProtocolError> {
    use std::process::Command;

    let binary_str = binary_path.to_string_lossy();

    let cmds: [Vec<String>; 3] = [
        vec![
            "add".to_string(),
            r"HKEY_CLASSES_ROOT\tabless".to_string(),
            "/ve".to_string(),
            "/d".to_string(),
            "URL:Tabless Protocol".to_string(),
            "/f".to_string(),
        ],
        vec![
            "add".to_string(),
            r"HKEY_CLASSES_ROOT\tabless".to_string(),
            "/v".to_string(),
            "URL Protocol".to_string(),
            "/d".to_string(),
            "".to_string(),
            "/f".to_string(),
        ],
        vec![
            "add".to_string(),
            r"HKEY_CLASSES_ROOT\tabless\shell\open\command".to_string(),
            "/ve".to_string(),
            "/d".to_string(),
            format!(r#""{}" "%1""#, binary_str),
            "/f".to_string(),
        ],
    ];

    for args in &cmds {
        let status = Command::new("reg").args(args).status().map_err(|e| {
            ProtocolError::RegistrationFailed {
                platform: "windows".to_string(),
                reason: format!("reg command failed: {}", e),
            }
        })?;

        if !status.success() {
            return Err(ProtocolError::RegistrationFailed {
                platform: "windows".to_string(),
                reason: format!("reg command exited with error: {:?}", args),
            });
        }
    }

    Ok(())
}
