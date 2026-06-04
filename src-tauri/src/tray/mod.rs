//! System tray module — icon, menu, state indication

use tauri::{
    AppHandle, Emitter, Manager,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem},
};

// Use Option to store menu items since we can't use generics easily
static mut TRAY_ITEMS: Option<(
    MenuItem<tauri::Wry>,
    MenuItem<tauri::Wry>,
    MenuItem<tauri::Wry>,
    MenuItem<tauri::Wry>,
    MenuItem<tauri::Wry>,
    MenuItem<tauri::Wry>,
)> = None;

static mut IS_RECORDING: bool = false;
static mut IS_PAUSED: bool = false;

fn update_menu() {
    unsafe {
        if let Some((ref start, ref stop, ref pause, ref resume, ref open, _)) = TRAY_ITEMS {
            let _ = start.set_enabled(!IS_RECORDING);
            let _ = stop.set_enabled(IS_RECORDING);
            let _ = pause.set_enabled(IS_RECORDING && !IS_PAUSED);
            let _ = resume.set_enabled(IS_RECORDING && IS_PAUSED);
            let _ = open.set_enabled(true);
        }
    }
}

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let start_item = MenuItem::with_id(app, "start", "Start Recording", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop", "Stop Recording", false, None::<&str>)?;
    let pause_item = MenuItem::with_id(app, "pause", "Pause Recording", false, None::<&str>)?;
    let resume_item = MenuItem::with_id(app, "resume", "Resume Recording", false, None::<&str>)?;
    let open_item = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&start_item, &stop_item, &pause_item, &resume_item, &open_item, &quit_item])?;

    let icon_bytes = include_bytes!("../../../public/favicon-16.png");
    let icon = tauri::image::Image::from_bytes(icon_bytes)?;

    // Store menu items for later state updates
    unsafe {
        TRAY_ITEMS = Some((start_item, stop_item, pause_item, resume_item, open_item, quit_item));
    }

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("EasySpecy")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "start" => {
                let _ = app.emit("tray-start-recording", ());
            }
            "stop" => {
                let _ = app.emit("tray-stop-recording", ());
            }
            "pause" => {
                let _ = app.emit("tray-pause-recording", ());
            }
            "resume" => {
                let _ = app.emit("tray-resume-recording", ());
            }
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    if let Ok(true) = window.is_minimized() {
                        let _ = window.unminimize();
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    if let Ok(true) = window.is_minimized() {
                        let _ = window.unminimize();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Update tray menu state based on recording status
pub fn update_tray_state(is_recording: bool, is_paused: bool) {
    unsafe {
        IS_RECORDING = is_recording;
        IS_PAUSED = is_paused;
        update_menu();
    }
}
