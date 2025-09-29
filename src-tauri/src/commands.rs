use crate::gamepad::{ControllerState, GamepadManager, DebugInfo};
use crate::evdev_gamepad::{EvdevGamepadManager, EvdevGamepadInfo};
use std::collections::HashMap;
use tauri::State;

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