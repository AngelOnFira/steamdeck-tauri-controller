mod gamepad;
mod commands;

use gamepad::GamepadManager;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let gamepad_manager = GamepadManager::new()
                .expect("Failed to initialize gamepad manager");
            
            let gamepad_manager = Arc::new(gamepad_manager);
            app.manage(gamepad_manager.clone());
            
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    gamepad_manager.poll_events(&app_handle);
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_connected_controllers,
            commands::get_controller_state,
            commands::send_to_light_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
