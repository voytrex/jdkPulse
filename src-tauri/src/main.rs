use jdk_pulse::{get_active_jdk, list_jdks, set_active_jdk};
use serde_json;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--list" | "-l" => {
                match list_jdks() {
                    Ok(jdks) => {
                        println!("{}", serde_json::to_string_pretty(&jdks).unwrap());
                    }
                    Err(e) => {
                        eprintln!("Error listing JDKs: {e}");
                        std::process::exit(1);
                    }
                }
            }
            "--set" | "-s" => {
                if args.len() < 3 {
                    eprintln!("Usage: {} --set <id|home>", args[0]);
                    std::process::exit(1);
                }
                match set_active_jdk(&args[2]) {
                    Ok(home) => {
                        println!("Active JDK set to: {}", home);
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            "--get" | "-g" => {
                match get_active_jdk() {
                    Ok(Some(jdk)) => {
                        println!("{}", serde_json::to_string_pretty(&jdk).unwrap());
                    }
                    Ok(None) => {
                        println!("{{}}");
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
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
        match list_jdks() {
            Ok(jdks) => {
                println!("{}", serde_json::to_string_pretty(&jdks).unwrap());
            }
            Err(e) => {
                eprintln!("Error listing JDKs: {e}");
                std::process::exit(1);
            }
        }
    }
}

