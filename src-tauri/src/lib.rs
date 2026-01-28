use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdkInfo {
    pub id: String,
    pub version_major: u32,
    pub version_full: String,
    pub home: String,
    pub vendor: Option<String>,
}

#[cfg(target_os = "macos")]
pub fn list_jdks() -> Result<Vec<JdkInfo>, String> {
    list_jdks_macos()
}

#[cfg(not(target_os = "macos"))]
pub fn list_jdks() -> Result<Vec<JdkInfo>, String> {
    Ok(vec![])
}

#[cfg(target_os = "macos")]
fn list_jdks_macos() -> Result<Vec<JdkInfo>, String> {
    let output = Command::new("/usr/libexec/java_home")
        .arg("-V")
        .output()
        .map_err(|e| format!("failed to execute java_home: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "java_home -V exited with status {}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stderr).to_string();
    // Note: `java_home -V` writes to stderr, not stdout.

    let mut jdks = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Matching Java Virtual Machines") {
            continue;
        }

        // Example line (macOS):
        // 21.0.1 (x86_64) "Eclipse Adoptium" - "OpenJDK 64-Bit Server VM" /Library/.../Contents/Home
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        // Last token should be the JAVA_HOME path
        let home = parts.last().unwrap().to_string();
        let version_full = parts[0].to_string();
        let version_major = parse_major_version(&version_full);

        // Try to extract vendor from quoted segment (best-effort only)
        let vendor = extract_quoted_segment(line);

        let id = format!("java-{}", version_full.replace('.', "_"));

        jdks.push(JdkInfo {
            id,
            version_major,
            version_full,
            home,
            vendor,
        });
    }

    Ok(jdks)
}

fn parse_major_version(version_full: &str) -> u32 {
    // Java 8 style: 1.8.0_382 -> major 8
    // Java 11+ style: 21.0.1 -> major 21
    if let Some(stripped) = version_full.strip_prefix("1.") {
        stripped
            .split('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(8)
    } else {
        version_full
            .split('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    }
}

fn extract_quoted_segment(line: &str) -> Option<String> {
    let mut in_quotes = false;
    let mut current = String::new();
    let mut segments = Vec::new();

    for c in line.chars() {
        if c == '"' {
            if in_quotes {
                // closing quote
                segments.push(current.clone());
                current.clear();
                in_quotes = false;
            } else {
                in_quotes = true;
            }
        } else if in_quotes {
            current.push(c);
        }
    }

    // Heuristic: first quoted segment is usually vendor (e.g. "Eclipse Adoptium")
    segments.into_iter().next()
}

pub fn get_active_jdk() -> Result<Option<JdkInfo>, String> {
    let state_file = get_state_file_path();
    match fs::read_to_string(&state_file) {
        Ok(home) => {
            let home = home.trim();
            if home.is_empty() {
                Ok(None)
            } else {
                // Try to find matching JDK info
                match list_jdks() {
                    Ok(jdks) => {
                        if let Some(jdk) = jdks.iter().find(|j| j.home == home) {
                            Ok(Some(jdk.clone()))
                        } else {
                            // Return a minimal JdkInfo with just the home path
                            Ok(Some(JdkInfo {
                                id: "unknown".to_string(),
                                version_major: 0,
                                version_full: "unknown".to_string(),
                                home: home.to_string(),
                                vendor: None,
                            }))
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
        Err(_) => Ok(None),
    }
}

pub fn set_active_jdk(id_or_home: &str) -> Result<String, String> {
    let jdk_home = if id_or_home.starts_with('/') || id_or_home.starts_with("~/") {
        // It's a path
        let mut path = PathBuf::from(id_or_home);
        if id_or_home.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                path = home.join(&id_or_home[2..]);
            }
        }
        if !path.exists() {
            return Err(format!("JDK path does not exist: {}", path.display()));
        }
        path.to_string_lossy().to_string()
    } else {
        // It's an ID - find the matching JDK
        match list_jdks() {
            Ok(jdks) => {
                if let Some(jdk) = jdks.iter().find(|j| j.id == id_or_home) {
                    jdk.home.clone()
                } else {
                    return Err(format!("JDK with ID '{}' not found", id_or_home));
                }
            }
            Err(e) => return Err(e),
        }
    };

    // Validate the JDK home path
    let jdk_path = PathBuf::from(&jdk_home);
    if !jdk_path.exists() {
        return Err(format!("JDK path does not exist: {}", jdk_home));
    }

    // Check for bin/java to ensure it's a valid JDK
    let java_bin = jdk_path.join("bin").join("java");
    if !java_bin.exists() {
        // Warning but don't fail
        eprintln!("Warning: {} does not contain bin/java", jdk_home);
    }

    // Write to state file
    let state_file = get_state_file_path();
    if let Some(parent) = state_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Error creating state directory: {e}"))?;
    }

    let mut file = fs::File::create(&state_file)
        .map_err(|e| format!("Error creating state file: {e}"))?;
    file.write_all(jdk_home.as_bytes())
        .map_err(|e| format!("Error writing state file: {e}"))?;

    Ok(jdk_home)
}

fn get_state_file_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".jdk_current")
    } else {
        PathBuf::from(".jdk_current")
    }
}

// Tauri commands
#[cfg(feature = "tauri")]
pub mod tauri_commands {
    use super::{get_active_jdk, list_jdks, set_active_jdk, JdkInfo};

    #[tauri::command]
    pub async fn list_jdks_command() -> Result<Vec<JdkInfo>, String> {
        list_jdks()
    }

    #[tauri::command]
    pub async fn get_active_jdk_command() -> Result<Option<JdkInfo>, String> {
        get_active_jdk()
    }

    #[tauri::command]
    pub async fn set_active_jdk_command(id: String) -> Result<String, String> {
        set_active_jdk(&id)
    }
}

#[cfg(feature = "tauri")]
pub mod tauri_tray {
    use super::{get_active_jdk, list_jdks, set_active_jdk, JdkInfo};
    use tauri::{AppHandle, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

    pub fn create_system_tray() -> SystemTray {
        match (list_jdks(), get_active_jdk()) {
            (Ok(jdks), Ok(active_jdk)) => {
                let menu = create_tray_menu(&jdks, active_jdk.as_ref());
                SystemTray::new().with_menu(menu)
            }
            _ => SystemTray::new().with_menu(
                SystemTrayMenu::new()
                    .add_item(SystemTrayMenuItem::with_id(
                        "error",
                        "Error loading JDKs",
                        false,
                        None::<&str>,
                    ))
                    .add_item(SystemTrayMenuItem::with_id(
                        "quit",
                        "Quit",
                        false,
                        None::<&str>,
                    )),
            ),
        }
    }

    fn create_tray_menu(jdks: &[JdkInfo], active_jdk: Option<&JdkInfo>) -> SystemTrayMenu {
        let mut menu = SystemTrayMenu::new();

        // Add JDK selection items
        for jdk in jdks {
            let label = if let Some(vendor) = &jdk.vendor {
                format!("Java {} ({})", jdk.version_major, vendor)
            } else {
                format!("Java {}", jdk.version_major)
            };

            let is_active = active_jdk
                .map(|a| a.id == jdk.id || a.home == jdk.home)
                .unwrap_or(false);

            let item = if is_active {
                SystemTrayMenuItem::with_id(
                    &jdk.id,
                    format!("✓ {}", label),
                    true,
                    None::<&str>,
                )
            } else {
                SystemTrayMenuItem::with_id(&jdk.id, label, true, None::<&str>)
            };
            menu = menu.add_item(item);
        }

        menu = menu.add_native_item(SystemTrayMenuItem::Separator);
        menu = menu.add_item(SystemTrayMenuItem::with_id(
            "quit",
            "Quit",
            false,
            None::<&str>,
        ));

        menu
    }

    pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                if id == "quit" {
                    app.exit(0);
                } else {
                    // It's a JDK selection
                    match set_active_jdk(&id) {
                        Ok(home) => {
                            println!("Active JDK set to: {}", home);
                            update_tray_menu(app);
                        }
                        Err(e) => {
                            eprintln!("Error setting JDK: {e}");
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn update_tray_menu(app: &AppHandle) {
        let tray = app.tray_handle();
        match (list_jdks(), get_active_jdk()) {
            (Ok(jdks), Ok(active_jdk)) => {
                let menu = create_tray_menu(&jdks, active_jdk.as_ref());
                if let Err(e) = tray.set_menu(menu) {
                    eprintln!("Error updating tray menu: {e}");
                }

                // Update tooltip with active JDK version
                let tooltip = if let Some(jdk) = active_jdk {
                    format!("JDK-Pulse – Java {}", jdk.version_major)
                } else {
                    "JDK-Pulse – No JDK selected".to_string()
                };
                if let Err(e) = tray.set_tooltip(&tooltip) {
                    eprintln!("Error setting tooltip: {e}");
                }
            }
            (Err(e), _) => {
                eprintln!("Error listing JDKs: {e}");
            }
            (_, Err(e)) => {
                eprintln!("Error getting active JDK: {e}");
            }
        }
    }
}
