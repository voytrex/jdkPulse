#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "tauri")]
fn main() {
    use jdk_pulse::{get_active_jdk, list_jdks, set_active_jdk, JdkInfo};
    use jdk_pulse::tauri_tray::create_system_tray;
    use tauri::Manager;

    // Define Tauri commands directly in the binary crate
    #[tauri::command]
    async fn list_jdks_command() -> Result<Vec<JdkInfo>, String> {
        list_jdks()
    }

    #[tauri::command]
    async fn get_active_jdk_command() -> Result<Option<JdkInfo>, String> {
        get_active_jdk()
    }

    #[tauri::command]
    async fn set_active_jdk_command(id: String) -> Result<String, String> {
        set_active_jdk(&id)
    }

    tauri::Builder::default()
        .setup(|app| {
            // Create system tray and store it in app state
            let tray = create_system_tray(app.handle())?;
            app.manage(tray);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_jdks_command,
            get_active_jdk_command,
            set_active_jdk_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(feature = "tauri"))]
fn main() {
    eprintln!("Tauri feature not enabled. Build with --features tauri");
    std::process::exit(1);
}
