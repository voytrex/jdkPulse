use serde::Serialize;
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

