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

fn integrate_appimage() {
    if let Ok(appimage_path) = std::env::var("APPIMAGE") {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return,
        };

        let desktop_dir = home.join(".local/share/applications");
        let icon_dir = home.join(".local/share/icons");

        let _ = std::fs::create_dir_all(&desktop_dir);
        let _ = std::fs::create_dir_all(&icon_dir);

        let target_icon_path = icon_dir.join("kazamo.png");
        let target_desktop_path = desktop_dir.join("kazamo-appimage.desktop");

        if let Ok(appdir_path) = std::env::var("APPDIR") {
            let appdir = std::path::Path::new(&appdir_path);
            let candidate_icons = [
                appdir.join("usr/share/icons/hicolor/256x256@2/apps/kazamo.png"),
                appdir.join("usr/share/icons/hicolor/256x256@2/apps/Kazamo.png"),
                appdir.join("usr/share/icons/hicolor/512x512/apps/kazamo.png"),
                appdir.join("usr/share/icons/hicolor/512x512/apps/Kazamo.png"),
                appdir.join("Kazamo.png"),
                appdir.join("usr/share/icons/hicolor/256x256/apps/kazamo.png"),
                appdir.join("usr/share/icons/hicolor/256x256/apps/Kazamo.png"),
                appdir.join("kazamo.png"),
                appdir.join("usr/share/icons/hicolor/128x128/apps/kazamo.png"),
                appdir.join("usr/share/icons/hicolor/128x128/apps/Kazamo.png"),
            ];

            if let Some(src_icon) = candidate_icons.iter().find(|p| p.exists()) {
                let _ = std::fs::copy(src_icon, &target_icon_path);
            }
        }

        let desktop_content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Kazamo\n\
             Comment=Kazamo - Voice-to-text for Linux\n\
             Exec=\"{}\" %U\n\
             Icon={}\n\
             Categories=Utility;AudioVideo;\n\
             Terminal=false\n\
             StartupNotify=true\n",
            appimage_path,
            target_icon_path.to_string_lossy()
        );

        let _ = std::fs::write(&target_desktop_path, desktop_content);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Integrate AppImage shortcut and icon
    integrate_appimage();
    // Check if another instance is already running by connecting to the IPC socket
    let sock_path = ipc_socket::socket_path();
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&sock_path) {
        use std::io::Write;
        let _ = stream.write_all(b"show\n");
        return;
    }

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

            // Hide to tray on close request instead of exiting
            if let Some(w) = app.get_webview_window("main") {
                let w_clone = w.clone();
                w.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_clone.hide();
                    }
                });
            }

            // Tray
            let dictation_item = MenuItem::with_id(app, "toggle-dictation", "Toggle Dictation", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show Kazamo", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide Kazamo", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&dictation_item, &show_item, &hide_item, &quit_item])?;
            let icon = Image::from_bytes(include_bytes!("../icons/tray-mic.png")).expect("tray icon");

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .icon_as_template(false)
                .menu(&menu)
                .tooltip("Kazamo")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "toggle-dictation" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = commands::toggle_tray_dictation(app.clone()).await {
                                eprintln!("[Kazamo] Tray dictation failed: {}", e);
                                if let Some(tray) = app.tray_by_id("main-tray") {
                                    let _ = tray.set_tooltip(Some(format!("Kazamo: {}", e)));
                                }
                            }
                        });
                    }
                    "show" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); } }
                    "hide" => { if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); } }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
                    }
                })
                .build(app)?;

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
