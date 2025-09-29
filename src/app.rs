#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke_without_args(cmd: &str) -> JsValue;
    
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
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

pub fn App() -> Element {
    let controllers = use_signal(|| HashMap::<usize, ControllerState>::new());
    let mut server_endpoint = use_signal(|| "http://localhost:8080/light-control".to_string());
    let last_event = use_signal(|| String::new());
    let app_version = use_signal(|| "0.1.2".to_string());
    let debug_info = use_signal(|| None::<DebugInfo>);
    let mut mouse_position = use_signal(|| (0.0, 0.0));
    let mut show_debug = use_signal(|| true);
    let mut last_key_event = use_signal(|| "None".to_string());
    let evdev_devices = use_signal(|| Vec::<EvdevGamepadInfo>::new());
    let steam_deck_info = use_signal(|| "Loading...".to_string());
    let last_evdev_event = use_signal(|| "None".to_string());

    // Poll for connected controllers and debug info
    let mut controllers_clone = controllers.clone();
    let mut debug_info_clone = debug_info.clone();
    let mut evdev_devices_clone = evdev_devices.clone();
    let mut steam_deck_info_clone = steam_deck_info.clone();
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            // Get controller states
            let result = invoke_without_args("get_connected_controllers").await;
            if let Ok(controllers_map) = serde_wasm_bindgen::from_value::<HashMap<usize, ControllerState>>(result) {
                controllers_clone.set(controllers_map);
            }
            
            // Get debug info
            let debug_result = invoke_without_args("get_debug_info").await;
            if let Ok(debug_data) = serde_wasm_bindgen::from_value::<DebugInfo>(debug_result) {
                debug_info_clone.set(Some(debug_data));
            }
            
            // Get evdev devices
            let evdev_result = invoke_without_args("get_evdev_devices").await;
            if let Ok(evdev_data) = serde_wasm_bindgen::from_value::<Vec<EvdevGamepadInfo>>(evdev_result) {
                evdev_devices_clone.set(evdev_data);
            }
            
            // Get Steam Deck info
            let steam_result = invoke_without_args("get_steam_deck_info").await;
            if let Ok(steam_data) = serde_wasm_bindgen::from_value::<String>(steam_result) {
                steam_deck_info_clone.set(steam_data);
            }
            
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    // Listen for gamepad events
    let mut last_event_clone = last_event.clone();
    use_effect(move || {
        spawn(async move {
            let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
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
            
            let _ = listen("gamepad-input", &handler).await;
            handler.forget();
        });
    });

    // Listen for evdev gamepad events
    let mut last_evdev_event_clone = last_evdev_event.clone();
    use_effect(move || {
        spawn(async move {
            let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
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
            
            let _ = listen("evdev-gamepad-input", &handler).await;
            handler.forget();
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

    let check_for_updates = move |_| {
        spawn(async move {
            // Update check will be implemented with tauri-plugin-updater
            gloo_console::log!("Checking for updates...");
        });
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
                let result = invoke_without_args("rescan_evdev_devices").await;
                if let Ok(devices) = serde_wasm_bindgen::from_value::<Vec<EvdevGamepadInfo>>(result) {
                    evdev_devices.set(devices);
                }
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
                button {
                    onclick: check_for_updates,
                    "Check for Updates"
                }
                button {
                    onclick: toggle_debug,
                    if *show_debug.read() { "Hide Debug" } else { "Show Debug" }
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