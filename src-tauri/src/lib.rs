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
    let mut all = Vec::new();

    // System JDKs from java_home
    all.extend(list_system_jdks_macos()?);

    // jenv-managed JDKs (if any)
    all.extend(list_jenv_jdks()?);

    Ok(all)
}

#[cfg(not(target_os = "macos"))]
pub fn list_jdks() -> Result<Vec<JdkInfo>, String> {
    Ok(vec![])
}

#[cfg(target_os = "macos")]
fn list_system_jdks_macos() -> Result<Vec<JdkInfo>, String> {
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

/// Discover JDKs managed by jenv under ~/.jenv/versions
#[cfg(target_os = "macos")]
fn list_jenv_jdks() -> Result<Vec<JdkInfo>, String> {
    let mut result = Vec::new();

    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let versions_dir = home.join(".jenv").join("versions");
    if !versions_dir.is_dir() {
        return Ok(result);
    }

    let entries = std::fs::read_dir(&versions_dir)
        .map_err(|e| format!("Failed to read jenv versions dir {}: {e}", versions_dir.display()))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        // jenv version name is the directory name, e.g. "21.0.10" or "openjdk64-21.0.10"
        let version_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Heuristic: use version_name as version_full, extract major version
        let version_full = version_name.clone();
        let version_major = parse_major_version(&version_full);

        // Determine JAVA_HOME:
        // - If there's a "Contents/Home" subdir (mac-style JDK), use that
        // - Else, use the version dir itself
        let contents_home = path.join("Contents").join("Home");
        let home_path = if contents_home.is_dir() {
            contents_home
        } else {
            path.clone()
        };

        // Require bin/java to exist
        if !home_path.join("bin").join("java").exists() {
            continue;
        }

        let id = format!("jenv-{}", version_name.replace('.', "_"));

        result.push(JdkInfo {
            id,
            version_major,
            version_full,
            home: home_path.to_string_lossy().to_string(),
            vendor: Some("jenv".to_string()),
        });
    }

    Ok(result)
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
    use super::{get_active_jdk, list_jdks, set_active_jdk};
    use tauri::{AppHandle, Manager};
    use tauri::menu::MenuBuilder;
    use tauri::tray::{TrayIconBuilder, TrayIcon};

    pub fn create_system_tray<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, Box<dyn std::error::Error>> {
        let menu = create_tray_menu(app)?;
        
        // Use default window icon if available, otherwise create without icon
        let mut builder = TrayIconBuilder::new();
        
        // Try to use the default window icon
        if let Some(default_icon) = app.default_window_icon() {
            builder = builder.icon(default_icon.clone());
        }
        
        let tray = builder
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(|app: &AppHandle<R>, event| {
                match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    id => {
                        // It's a JDK selection
                        match set_active_jdk(id) {
                            Ok(home) => {
                                println!("Active JDK set to: {}", home);
                                if let Err(e) = update_tray_menu(app) {
                                    eprintln!("Error updating tray menu: {e}");
                                }
                            }
                            Err(e) => {
                                eprintln!("Error setting JDK: {e}");
                            }
                        }
                    }
                }
            })
            .build(app)?;
        
        Ok(tray)
    }

    fn create_tray_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<tauri::menu::Menu<R>, Box<dyn std::error::Error>> {
        let mut builder = MenuBuilder::new(app);

        match (list_jdks(), get_active_jdk()) {
            (Ok(jdks), Ok(active_jdk)) => {
                // Add JDK selection items
                // First, a synthetic "Use jenv default" entry if applicable
                if jenv_default_exists() {
                    builder = builder.text("jenv-default", "Use jenv default");
                    builder = builder.separator();
                }

                for jdk in jdks {
                    let label = match &jdk.vendor {
                        Some(vendor) if vendor == "jenv" => {
                            format!("{} (jenv)", jdk.version_full)
                        }
                        Some(vendor) => format!("Java {} ({})", jdk.version_major, vendor),
                        None => format!("Java {}", jdk.version_major),
                    };

                    let is_active = active_jdk
                        .as_ref()
                        .map(|a| a.id == jdk.id || a.home == jdk.home)
                        .unwrap_or(false);

                    let text = if is_active {
                        format!("✓ {}", label)
                    } else {
                        label
                    };

                    builder = builder.text(&jdk.id, &text);
                }
            }
            _ => {
                builder = builder.text("error", "Error loading JDKs");
            }
        }

        // Add separator
        builder = builder.separator();

        // Add quit item
        builder = builder.text("quit", "Quit");

        builder.build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    fn update_tray_menu<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
        // Get the tray handle from app state
        let menu = create_tray_menu(app)?;
        
        // Try to get the tray from state and update it
        if let Some(tray_state) = app.try_state::<TrayIcon<R>>() {
            let tray = tray_state.inner();
            tray.set_menu(Some(menu))?;

            // Update tooltip with active JDK version
            match get_active_jdk() {
                Ok(Some(jdk)) => {
                    let tooltip = format!("JDK-Pulse – Java {}", jdk.version_major);
                    tray.set_tooltip(Some(&tooltip))?;
                }
                Ok(None) => {
                    tray.set_tooltip(Some("JDK-Pulse – No JDK selected"))?;
                }
                Err(e) => {
                    eprintln!("Error getting active JDK: {e}");
                }
            }
        } else {
            // If we can't get the tray from state, the menu will be updated on next creation
            println!("Tray handle not found in app state - menu will update on next interaction");
        }
        Ok(())
    }

    /// Check if a jenv default version is configured (~/.jenv/version exists)
    fn jenv_default_exists() -> bool {
        if let Some(home) = dirs::home_dir() {
            let default_file = home.join(".jenv").join("version");
            default_file.is_file()
        } else {
            false
        }
    }
}
