#![allow(non_snake_case)]

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
    
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], catch)]
    async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<JsValue, JsValue>;
}

// Helper function to invoke commands without arguments
async fn invoke_without_args(cmd: &str) -> Result<JsValue, JsValue> {
    let empty_args = serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap();
    invoke(cmd, empty_args).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerState {
    pub buttons: HashMap<String, bool>,
    pub axes: HashMap<String, f32>,
    pub connected: bool,
    pub controller_id: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerEvent {
    pub controller_id: usize,
    pub event_type: String,
    pub button: Option<String>,
    pub axis: Option<String>,
    pub value: Option<f32>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    pub gilrs_initialized: bool,
    pub total_gamepads: usize,
    pub connected_gamepads: Vec<GamepadInfo>,
    pub input_devices: Vec<String>,
    pub permissions_check: String,
    pub last_event_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadInfo {
    pub id: usize,
    pub name: String,
    pub is_connected: bool,
    pub power_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvdevGamepadInfo {
    pub device_path: String,
    pub name: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub is_gamepad: bool,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvdevControllerEvent {
    pub device_path: String,
    pub event_type: String,
    pub code: u16,
    pub value: i32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub current_version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

pub fn App() -> Element {
    let controllers = use_signal(|| HashMap::<usize, ControllerState>::new());
    let mut server_endpoint = use_signal(|| "0.1.11".to_string());
    let last_event = use_signal(|| String::new());
    let app_version = use_signal(|| "0.1.11".to_string());
    let debug_info = use_signal(|| None::<DebugInfo>);
    let mut mouse_position = use_signal(|| (0.0, 0.0));
    let show_debug = use_signal(|| true);
    let mut last_key_event = use_signal(|| "0.1.11".to_string());
    let evdev_devices = use_signal(|| Vec::<EvdevGamepadInfo>::new());
    let steam_deck_info = use_signal(|| "0.1.11".to_string());
    let last_evdev_event = use_signal(|| "0.1.11".to_string());
    let update_status = use_signal(|| "0.1.11".to_string());
    let update_info = use_signal(|| None::<UpdateInfo>);
    let is_checking_update = use_signal(|| false);
    let is_downloading_update = use_signal(|| false);
    let download_progress = use_signal(|| 0u64);
    let download_total = use_signal(|| 0u64);

    // Poll for connected controllers and debug info
    let mut controllers_clone = controllers.clone();
    let mut debug_info_clone = debug_info.clone();
    let mut evdev_devices_clone = evdev_devices.clone();
    let mut steam_deck_info_clone = steam_deck_info.clone();
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            // Get controller states
            if let Ok(result) = invoke_without_args("get_connected_controllers").await {
                if let Ok(controllers_map) = serde_wasm_bindgen::from_value::<HashMap<usize, ControllerState>>(result) {
                    controllers_clone.set(controllers_map);
                }
            }
            
            // Get debug info
            if let Ok(debug_result) = invoke_without_args("get_debug_info").await {
                if let Ok(debug_data) = serde_wasm_bindgen::from_value::<DebugInfo>(debug_result) {
                    debug_info_clone.set(Some(debug_data));
                }
            }
            
            // Get evdev devices
            if let Ok(evdev_result) = invoke_without_args("get_evdev_devices").await {
                if let Ok(evdev_data) = serde_wasm_bindgen::from_value::<Vec<EvdevGamepadInfo>>(evdev_result) {
                    evdev_devices_clone.set(evdev_data);
                }
            }
            
            // Get Steam Deck info
            if let Ok(steam_result) = invoke_without_args("get_steam_deck_info").await {
                if let Ok(steam_data) = serde_wasm_bindgen::from_value::<String>(steam_result) {
                    steam_deck_info_clone.set(steam_data);
                }
            }
            
            TimeoutFuture::new(1000).await;
        }
    });

    // Listen for gamepad events and update progress
    let mut last_event_clone = last_event.clone();
    let mut last_evdev_event_clone = last_evdev_event.clone();
    let mut download_progress_clone = download_progress.clone();
    let download_total_clone = download_total.clone();
    let mut update_status_clone = update_status.clone();
    use_effect(move || {
        spawn(async move {
            // Set up gamepad event listener
            let gamepad_handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                if let Ok(event_data) = serde_wasm_bindgen::from_value::<ControllerEvent>(event) {
                    last_event_clone.set(format!(
                        "Controller {}: {} - {:?}{:?} = {:?}",
                        event_data.controller_id,
                        event_data.event_type,
                        event_data.button.as_deref().unwrap_or(""),
                        event_data.axis.as_deref().unwrap_or(""),
                        event_data.value
                    ));
                }
            });
            
            // Set up evdev event listener
            let evdev_handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                if let Ok(event_data) = serde_wasm_bindgen::from_value::<EvdevControllerEvent>(event) {
                    last_evdev_event_clone.set(format!(
                        "EVDEV {}: {} code={} value={}",
                        event_data.device_path,
                        event_data.event_type,
                        event_data.code,
                        event_data.value
                    ));
                }
            });
            
            // Update download started handler
            let mut download_total_clone2 = download_total_clone.clone();
            let mut update_status_clone2 = update_status_clone.clone();
            let download_started_handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                if let Ok(content_length) = serde_wasm_bindgen::from_value::<Option<u64>>(event) {
                    if let Some(size) = content_length {
                        download_total_clone2.set(size);
                        update_status_clone2.set(format!("Downloading update... ({:.2} MB)", size as f64 / 1024.0 / 1024.0));
                        gloo_console::log!(&format!("Download started - size: {} bytes", size));
                    }
                }
            });
            
            // Update download progress handler
            let download_progress_handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
                if let Ok(chunk_length) = serde_wasm_bindgen::from_value::<u64>(event) {
                    let current = *download_progress_clone.read() + chunk_length;
                    download_progress_clone.set(current);
                    
                    let total = *download_total_clone.read();
                    if total > 0 {
                        let percent = (current as f64 / total as f64 * 100.0) as u8;
                        update_status_clone.set(format!("Downloading... {}%", percent));
                    }
                }
            });
            
            // Update installing handler
            let mut update_status_clone3 = update_status_clone.clone();
            let installing_handler = Closure::<dyn FnMut(JsValue)>::new(move |_: JsValue| {
                update_status_clone3.set("Installing update...".to_string());
                gloo_console::log!("Installing update...");
            });
            
            let _ = listen("gamepad-input", &gamepad_handler).await;
            let _ = listen("evdev-gamepad-input", &evdev_handler).await;
            let _ = listen("update-download-started", &download_started_handler).await;
            let _ = listen("update-download-progress", &download_progress_handler).await;
            let _ = listen("update-installing", &installing_handler).await;
            
            gamepad_handler.forget();
            evdev_handler.forget();
            download_started_handler.forget();
            download_progress_handler.forget();
            installing_handler.forget();
        });
    });

    let send_to_server = {
        let server_endpoint = server_endpoint.clone();
        move |controller_id: usize, action: String| {
            let endpoint_clone = server_endpoint.clone();
            spawn(async move {
                let endpoint = endpoint_clone.read().clone();
                let data = serde_json::json!({
                    "controller_id": controller_id,
                    "action": action,
                    "timestamp": js_sys::Date::now()
                });
                
                let args = serde_wasm_bindgen::to_value(&serde_json::json!({
                    "endpoint": endpoint,
                    "data": data
                })).unwrap();
                
                let _ = invoke("send_to_light_server", args).await;
            });
        }
    };

    let check_for_updates = {
        let update_status = update_status.clone();
        let update_info = update_info.clone();
        let is_checking_update = is_checking_update.clone();
        move |_| {
            let mut update_status = update_status.clone();
            let mut update_info = update_info.clone();
            let mut is_checking_update = is_checking_update.clone();
            
            spawn(async move {
                is_checking_update.set(true);
                update_status.set("Checking for updates...".to_string());
                gloo_console::log!("🔍 Starting update check...");
                
                let result = invoke_without_args("check_for_updates").await;
                
                match result {
                    Ok(update_data) => {
                        if let Ok(info) = serde_wasm_bindgen::from_value::<UpdateInfo>(update_data) {
                            gloo_console::log!("✅ Update check complete");
                            
                            if info.available {
                                update_status.set(format!(
                                    "Update available: {} → {}",
                                    info.current_version,
                                    info.version.as_deref().unwrap_or("unknown")
                                ));
                            } else {
                                update_status.set(format!(
                                    "You're on the latest version ({})",
                                    info.current_version
                                ));
                            }
                            
                            update_info.set(Some(info));
                        } else {
                            gloo_console::error!("Failed to parse update info");
                            update_status.set("Failed to parse update info".to_string());
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Error checking updates: {:?}", e);
                        gloo_console::error!(&error_msg);
                        update_status.set(error_msg);
                    }
                }
                
                is_checking_update.set(false);
            });
        }
    };
    
    let toggle_debug = {
        let mut show_debug = show_debug.clone();
        move |_| {
            let current = *show_debug.read();
            show_debug.set(!current);
        }
    };

    let rescan_evdev = {
        let mut evdev_devices = evdev_devices.clone();
        move |_| {
            spawn(async move {
                // Add a small delay to prevent rapid successive calls
                TimeoutFuture::new(100).await;
                if let Ok(result) = invoke_without_args("rescan_evdev_devices").await {
                    if let Ok(devices) = serde_wasm_bindgen::from_value::<Vec<EvdevGamepadInfo>>(result) {
                        evdev_devices.set(devices);
                    }
                }
            });
        }
    };
    
    let exit_app = move |_| {
        spawn(async move {
            gloo_console::log!("Exiting application...");
            let _ = invoke_without_args("exit_app").await;
        });
    };
    
    let download_and_install = {
        let update_status = update_status.clone();
        let is_downloading_update = is_downloading_update.clone();
        let download_progress = download_progress.clone();
        let download_total = download_total.clone();
        
        move |_| {
            let mut update_status = update_status.clone();
            let mut is_downloading_update = is_downloading_update.clone();
            let mut download_progress = download_progress.clone();
            let mut download_total = download_total.clone();
            
            spawn(async move {
                is_downloading_update.set(true);
                update_status.set("Downloading update...".to_string());
                download_progress.set(0);
                download_total.set(0);
                
                gloo_console::log!("📦 Starting update download...");
                
                let result = invoke_without_args("download_and_install_update").await;
                
                match result {
                    Ok(_) => {
                        gloo_console::log!("✅ Update installed successfully!");
                        update_status.set("Update installed! Restarting application...".to_string());
                        
                        // Wait a moment to show the message, then restart
                        TimeoutFuture::new(2000).await;
                        
                        gloo_console::log!("🔄 Triggering application restart...");
                        let _ = invoke_without_args("restart_app").await;
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to install update: {:?}", e);
                        gloo_console::error!(&error_msg);
                        update_status.set(error_msg);
                    }
                }
                
                is_downloading_update.set(false);
            });
        }
    };

    rsx! {
        link { rel: "stylesheet", href: "styles.css" }
        main {
            class: "container",
            tabindex: "0",
            onmousemove: move |event| {
                mouse_position.set((event.client_coordinates().x, event.client_coordinates().y));
            },
            onkeydown: move |event| {
                last_key_event.set(format!("KeyDown: {} (code: {})", event.key(), event.code()));
            },
            onkeyup: move |event| {
                last_key_event.set(format!("KeyUp: {} (code: {})", event.key(), event.code()));
            },
            
            h1 { "Steam Deck Controller Light Show Control" }
            
            div {
                class: "version-info",
                p { "Version: {app_version}" }
                
                div {
                    class: "update-section",
                    button {
                        onclick: check_for_updates,
                        disabled: *is_checking_update.read(),
                        if *is_checking_update.read() { "Checking..." } else { "Check for Updates" }
                    }
                    p { 
                        class: "update-status",
                        "{update_status}" 
                    }
                    
                    if let Some(info) = update_info.read().as_ref() {
                        if info.available {
                            div {
                                class: "update-available",
                                p { "📦 New version available: {info.version.as_deref().unwrap_or(\"unknown\")}" }
                                if let Some(body) = &info.body {
                                    div {
                                        class: "update-changelog",
                                        h4 { "What's New:" }
                                        pre { "{body}" }
                                    }
                                }
                                button {
                                    class: "update-install-button",
                                    onclick: download_and_install,
                                    disabled: *is_downloading_update.read(),
                                    if *is_downloading_update.read() {
                                        "Installing..."
                                    } else {
                                        "Download and Install"
                                    }
                                }
                                
                                if *is_downloading_update.read() && *download_total.read() > 0 {
                                    div {
                                        class: "download-progress",
                                        div {
                                            class: "progress-bar",
                                            div {
                                                class: "progress-fill",
                                                style: "width: {(*download_progress.read() as f64 / *download_total.read() as f64 * 100.0)}%"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                div {
                    class: "button-group",
                    button {
                        onclick: toggle_debug,
                        if *show_debug.read() { "Hide Debug" } else { "Show Debug" }
                    }
                    button {
                        onclick: exit_app,
                        class: "exit-button",
                        "Exit"
                    }
                }
            }
            
            div {
                class: "server-config",
                h2 { "Server Configuration" }
                input {
                    value: "{server_endpoint}",
                    oninput: move |event| server_endpoint.set(event.value()),
                    placeholder: "http://localhost:8080/light-control"
                }
            }
            
            div {
                class: "controllers-section",
                h2 { "Connected Controllers" }
                
                if controllers.read().is_empty() {
                    p { "No controllers connected. Please connect a controller." }
                } else {
                    {controllers.read().iter().map(|(id, controller)| {
                        let controller_id = *id;
                        let buttons_elements = controller.buttons.iter().map(|(button, pressed)| {
                            let button_name = button.clone();
                            let button_action = button.clone();
                            let is_pressed = *pressed;
                            rsx! {
                                button {
                                    key: "{button_name}",
                                    class: if is_pressed { "button pressed" } else { "button" },
                                    onclick: move |_| {
                                        send_to_server(controller_id, format!("button:{}", button_action));
                                    },
                                    "{button_name}: {is_pressed}"
                                }
                            }
                        });
                        
                        let axes_elements = controller.axes.iter().map(|(axis, value)| {
                            let axis_name = axis.clone();
                            let axis_value = *value;
                            rsx! {
                                div {
                                    key: "{axis_name}",
                                    class: "axis-display",
                                    "{axis_name}: {axis_value:.2}"
                                    div {
                                        class: "axis-bar",
                                        div {
                                            class: "axis-value",
                                            style: "width: {(axis_value + 1.0) * 50.0}%"
                                        }
                                    }
                                }
                            }
                        });
                        
                        rsx! {
                            div {
                                key: "{controller_id}",
                                class: "controller-card",
                                h3 { "Controller {controller_id}" }
                                
                                div {
                                    class: "buttons-grid",
                                    h4 { "Buttons" }
                                    {buttons_elements}
                                }
                                
                                div {
                                    class: "axes-grid",
                                    h4 { "Axes" }
                                    {axes_elements}
                                }
                            }
                        }
                    })}
                }
            }
            
            if *show_debug.read() {
                div {
                    class: "debug-panel",
                    h2 { "🐛 Debug Information" }
                    
                    div {
                        class: "debug-section",
                        h3 { "Input Events" }
                        p { "Mouse: X={mouse_position.read().0:.0}, Y={mouse_position.read().1:.0}" }
                        p { "Keyboard: {last_key_event}" }
                    }
                    
                    if let Some(debug) = debug_info.read().as_ref() {
                        div {
                            class: "debug-section",
                            h3 { "Gamepad System Status" }
                            p { "GilRs Initialized: {debug.gilrs_initialized}" }
                            p { "Total Gamepads: {debug.total_gamepads}" }
                            if let Some(last_time) = debug.last_event_time {
                                p { "Last Event: {last_time}" }
                            } else {
                                p { "Last Event: None" }
                            }
                        }
                        
                        div {
                            class: "debug-section",
                            h3 { "Detected Gamepads" }
                            if debug.connected_gamepads.is_empty() {
                                p { "❌ No gamepads detected by GilRs" }
                            } else {
                                for gamepad in &debug.connected_gamepads {
                                    div {
                                        class: "debug-gamepad",
                                        p { "ID: {gamepad.id}" }
                                        p { "Name: {gamepad.name}" }
                                        p { "Connected: {gamepad.is_connected}" }
                                        p { "Power: {gamepad.power_info}" }
                                    }
                                }
                            }
                        }
                        
                        div {
                            class: "debug-section",
                            h3 { "Input Devices (/dev/input/)" }
                            if debug.input_devices.is_empty() {
                                p { "❌ No input devices found" }
                            } else {
                                for device in &debug.input_devices {
                                    p { "{device}" }
                                }
                            }
                        }
                        
                        div {
                            class: "debug-section",
                            h3 { "Permissions Check" }
                            pre { "{debug.permissions_check}" }
                        }
                        
                        div {
                            class: "debug-section",
                            h3 { "🎮 Steam Deck Compatibility" }
                            pre { "{steam_deck_info}" }
                        }
                        
                        div {
                            class: "debug-section",
                            h3 { "⚡ Direct Evdev Devices" }
                            button {
                                onclick: rescan_evdev,
                                "🔄 Rescan Devices"
                            }
                            if evdev_devices.read().is_empty() {
                                p { "❌ No evdev gamepad devices detected" }
                            } else {
                                for device in evdev_devices.read().iter() {
                                    div {
                                        class: "debug-gamepad",
                                        p { "Path: {device.device_path}" }
                                        p { "Name: {device.name}" }
                                        if let (Some(vid), Some(pid)) = (device.vendor_id, device.product_id) {
                                            p { "VID/PID: {vid:04x}:{pid:04x}" }
                                        }
                                        p { "Capabilities: {device.capabilities.join(\", \")}" }
                                    }
                                }
                            }
                        }
                    } else {
                        p { "Loading debug information..." }
                    }
                }
            }
            
            div {
                class: "last-event",
                h3 { "Last Events" }
                p { "GilRs: {last_event}" }
                p { "Evdev: {last_evdev_event}" }
            }
        }
    }
}