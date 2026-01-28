use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize)]
struct JdkInfo {
    id: String,
    version_major: u32,
    version_full: String,
    home: String,
    vendor: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--list" | "-l" => {
                list_jdks();
            }
            "--set" | "-s" => {
                if args.len() < 3 {
                    eprintln!("Usage: {} --set <id|home>", args[0]);
                    std::process::exit(1);
                }
                set_active_jdk(&args[2]);
            }
            "--get" | "-g" => {
                get_active_jdk();
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
                eprintln!("Usage:");
                eprintln!("  {} [--list]     List all installed JDKs", args[0]);
                eprintln!("  {} --set <id>   Set active JDK by ID or home path", args[0]);
                eprintln!("  {} --get         Get current active JDK", args[0]);
                std::process::exit(1);
            }
        }
    } else {
        // Default: list JDKs
        list_jdks();
    }
}

fn list_jdks() {
    #[cfg(target_os = "macos")]
    {
        match list_jdks_macos() {
            Ok(jdks) => {
                println!("{}", serde_json::to_string_pretty(&jdks).unwrap());
            }
            Err(e) => {
                eprintln!("Error listing JDKs: {e}");
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!("[]");
    }
}

fn get_active_jdk() {
    let state_file = get_state_file_path();
    match fs::read_to_string(&state_file) {
        Ok(home) => {
            let home = home.trim();
            if home.is_empty() {
                println!("{{}}");
            } else {
                // Try to find matching JDK info
                #[cfg(target_os = "macos")]
                {
                    if let Ok(jdks) = list_jdks_macos() {
                        if let Some(jdk) = jdks.iter().find(|j| j.home == home) {
                            println!("{}", serde_json::to_string_pretty(jdk).unwrap());
                            return;
                        }
                    }
                }
                // Fallback: just return the home path
                println!("{{\"home\":\"{}\"}}", home);
            }
        }
        Err(_) => {
            println!("{{}}");
        }
    }
}

fn set_active_jdk(id_or_home: &str) {
    let jdk_home = if id_or_home.starts_with('/') || id_or_home.starts_with("~/") {
        // It's a path
        let mut path = PathBuf::from(id_or_home);
        if id_or_home.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                path = home.join(&id_or_home[2..]);
            }
        }
        if !path.exists() {
            eprintln!("Error: JDK path does not exist: {}", path.display());
            std::process::exit(1);
        }
        path.to_string_lossy().to_string()
    } else {
        // It's an ID - find the matching JDK
        #[cfg(target_os = "macos")]
        {
            match list_jdks_macos() {
                Ok(jdks) => {
                    if let Some(jdk) = jdks.iter().find(|j| j.id == id_or_home) {
                        jdk.home.clone()
                    } else {
                        eprintln!("Error: JDK with ID '{}' not found", id_or_home);
                        eprintln!("Available JDKs:");
                        for jdk in &jdks {
                            eprintln!("  {} - {}", jdk.id, jdk.home);
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error listing JDKs: {e}");
                    std::process::exit(1);
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            eprintln!("Error: JDK ID lookup not supported on this platform");
            std::process::exit(1);
        }
    };

    // Validate the JDK home path
    let jdk_path = PathBuf::from(&jdk_home);
    if !jdk_path.exists() {
        eprintln!("Error: JDK path does not exist: {}", jdk_home);
        std::process::exit(1);
    }

    // Check for bin/java to ensure it's a valid JDK
    let java_bin = jdk_path.join("bin").join("java");
    if !java_bin.exists() {
        eprintln!("Warning: {} does not contain bin/java", jdk_home);
        eprintln!("This may not be a valid JDK installation.");
    }

    // Write to state file
    let state_file = get_state_file_path();
    if let Some(parent) = state_file.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Error creating state directory: {e}");
            std::process::exit(1);
        }
    }

    match fs::File::create(&state_file) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(jdk_home.as_bytes()) {
                eprintln!("Error writing state file: {e}");
                std::process::exit(1);
            }
            println!("Active JDK set to: {}", jdk_home);
        }
        Err(e) => {
            eprintln!("Error creating state file: {e}");
            std::process::exit(1);
        }
    }
}

fn get_state_file_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".jdk_current")
    } else {
        PathBuf::from(".jdk_current")
    }
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

