mod commands;
mod paraformer;
mod ipc_socket;
mod recording;
mod settings;
mod transcription;

use commands::AppState;
use recording::Recorder;
use settings::Settings;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

pub struct SharedState {
    pub ipc_result: ipc_socket::ResultBuffer,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let saved = Settings::load();
    let result_buf = ipc_socket::new_result_buffer();
    let result_buf_clone = result_buf.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let resource_dir = app.path().resource_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            app.manage(AppState {
                recorder: Arc::new(Recorder::new()),
                resource_dir,
                settings: tokio::sync::Mutex::new(saved),
            });

            app.manage(SharedState {
                ipc_result: result_buf_clone,
            });

            // Start Unix socket
            ipc_socket::start_socket_server(app.handle().clone(), result_buf);

            // Tray
            let show_item = MenuItem::with_id(app, "show", "Show Kazamo", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide Kazamo", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;
            let icon = Image::from_bytes(include_bytes!("../icons/icon.png")).expect("icon");

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .menu(&menu)
                .tooltip("Kazamo")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
                    "hide" => { if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); } }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); }
                    }
                })
                .build(app)?;

            if let Some(w) = app.get_webview_window("main") {
                if let Ok(s) = w.scale_factor() { if s > 1.0 { let _ = w.set_zoom(s); } }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::start_recording,
            commands::stop_recording,
            commands::transcribe_audio,
            commands::list_models,
            commands::delete_model,
            commands::open_model_dir,
            commands::download_model,
            commands::set_ipc_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
