#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "tauri")]
fn main() {
    use jdk_pulse::tauri_commands::{get_active_jdk_command, list_jdks_command, set_active_jdk_command};
    use jdk_pulse::tauri_tray::create_system_tray;
    use tauri::Manager;

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
