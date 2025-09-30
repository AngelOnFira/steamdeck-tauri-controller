use crate::gamepad::{ControllerState, GamepadManager, DebugInfo};
use crate::evdev_gamepad::{EvdevGamepadManager, EvdevGamepadInfo};
use std::collections::HashMap;
use tauri::{State, Emitter};
use serde::{Serialize, Deserialize};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub fn get_connected_controllers(
    gamepad_manager: State<'_, GamepadManager>,
) -> Result<HashMap<usize, ControllerState>, String> {
    Ok(gamepad_manager.get_controller_states())
}

#[tauri::command]
pub fn get_controller_state(
    controller_id: usize,
    gamepad_manager: State<'_, GamepadManager>,
) -> Result<Option<ControllerState>, String> {
    Ok(gamepad_manager.get_controller_state(controller_id))
}

#[tauri::command]
pub fn get_debug_info(
    gamepad_manager: State<'_, GamepadManager>,
) -> Result<DebugInfo, String> {
    Ok(gamepad_manager.get_debug_info())
}

#[tauri::command]
pub fn send_to_light_server(
    endpoint: String,
    data: serde_json::Value,
) -> Result<String, String> {
    use reqwest::blocking::Client;
    
    let client = Client::new();
    let response = client
        .post(&endpoint)
        .json(&data)
        .send()
        .map_err(|e| format!("Failed to send to server: {}", e))?;
    
    if response.status().is_success() {
        Ok("Success".to_string())
    } else {
        Err(format!("Server returned error: {}", response.status()))
    }
}

#[tauri::command]
pub fn get_evdev_devices(
    evdev_manager: State<'_, EvdevGamepadManager>,
) -> Result<Vec<EvdevGamepadInfo>, String> {
    Ok(evdev_manager.get_detected_devices())
}

#[tauri::command]
pub fn rescan_evdev_devices(
    evdev_manager: State<'_, EvdevGamepadManager>,
) -> Result<Vec<EvdevGamepadInfo>, String> {
    evdev_manager.scan_for_gamepad_devices()
        .map_err(|e| format!("Failed to scan devices: {}", e))?;
    Ok(evdev_manager.get_detected_devices())
}

#[tauri::command]
pub fn get_steam_deck_info(
    evdev_manager: State<'_, EvdevGamepadManager>,
) -> Result<String, String> {
    Ok(evdev_manager.get_steam_deck_info())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub current_version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(
    app: tauri::AppHandle,
) -> Result<UpdateInfo, String> {
    println!("🔍 Checking for updates...");
    
    let updater = app.updater_builder().build()
        .map_err(|e| {
            println!("❌ Failed to build updater: {}", e);
            format!("Failed to initialize updater: {}", e)
        })?;
    
    match updater.check().await {
        Ok(Some(update)) => {
            println!("✅ Update available: {}", update.version);
            Ok(UpdateInfo {
                available: true,
                version: Some(update.version.clone()),
                current_version: update.current_version.clone(),
                body: update.body.clone(),
                date: update.date.map(|d| d.to_string()),
            })
        }
        Ok(None) => {
            println!("✅ No updates available - already on latest version");
            Ok(UpdateInfo {
                available: false,
                version: None,
                current_version: app.package_info().version.to_string(),
                body: None,
                date: None,
            })
        }
        Err(e) => {
            println!("❌ Error checking for updates: {}", e);
            Err(format!("Failed to check for updates: {}", e))
        }
    }
}

#[tauri::command]
pub async fn download_and_install_update(
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("📦 Starting update download and installation...");
    
    let updater = app.updater_builder().build()
        .map_err(|e| {
            println!("❌ Failed to build updater: {}", e);
            format!("Failed to initialize updater: {}", e)
        })?;
    
    match updater.check().await {
        Ok(Some(update)) => {
            println!("📥 Downloading update version: {}", update.version);
            
            // Download and install with progress events
            let mut downloaded_bytes = 0u64;
            let mut is_first_chunk = true;
            let app_clone = app.clone();
            let app_clone2 = app.clone();
            
            update.download_and_install(
                move |chunk_size, total_size| {
                    if is_first_chunk {
                        // First chunk - emit start event
                        println!("🚀 Download started - total size: {:?} bytes", total_size);
                        let _ = app_clone.emit("update-download-started", total_size);
                        is_first_chunk = false;
                    }
                    
                    downloaded_bytes += chunk_size as u64;
                    println!("📊 Downloaded {} bytes (total downloaded: {})", chunk_size, downloaded_bytes);
                    
                    let _ = app_clone.emit("update-download-progress", chunk_size as u64);
                },
                move || {
                    println!("✅ Download completed! Installing update...");
                    let _ = app_clone2.emit("update-download-finished", ());
                    let _ = app_clone2.emit("update-installing", ());
                }
            ).await.map_err(|e| {
                println!("❌ Failed to download/install update: {}", e);
                format!("Failed to download/install update: {}", e)
            })?;
            
            println!("🎉 Update installed successfully!");
            Ok("Update installed successfully!".to_string())
        }
        Ok(None) => {
            println!("ℹ️  No updates available");
            Err("No updates available".to_string())
        }
        Err(e) => {
            println!("❌ Error checking for updates: {}", e);
            Err(format!("Failed to check for updates: {}", e))
        }
    }
}

#[tauri::command]
pub async fn exit_app(
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    println!("👋 Exiting application...");
    app_handle.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn restart_app(
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!("🔄 Restarting application...");
    
    // Use the process plugin to restart the app
    app.request_restart();
    
    Ok("Restarting...".to_string())
}