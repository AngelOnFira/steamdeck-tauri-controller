mod gamepad;
mod commands;
mod evdev_gamepad;

use gamepad::GamepadManager;
use evdev_gamepad::EvdevGamepadManager;
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
            
            // Initialize evdev gamepad manager for Steam Deck compatibility
            let evdev_manager = EvdevGamepadManager::new()
                .expect("Failed to initialize evdev gamepad manager");
            let evdev_manager = Arc::new(evdev_manager);
            app.manage(evdev_manager.clone());
            
            // Scan for evdev devices on startup
            if let Err(e) = evdev_manager.scan_for_gamepad_devices() {
                println!("⚠️  Failed to scan evdev devices: {}", e);
            }
            
            let app_handle = app.handle().clone();
            let evdev_manager_clone = evdev_manager.clone();
            std::thread::spawn(move || {
                loop {
                    gamepad_manager.poll_events(&app_handle);
                    if let Err(e) = evdev_manager_clone.poll_events(&app_handle) {
                        println!("⚠️  Evdev polling error: {}", e);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_connected_controllers,
            commands::get_controller_state,
            commands::get_debug_info,
            commands::send_to_light_server,
            commands::get_evdev_devices,
            commands::rescan_evdev_devices,
            commands::get_steam_deck_info,
            commands::check_for_updates,
            commands::download_and_install_update,
            commands::exit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
