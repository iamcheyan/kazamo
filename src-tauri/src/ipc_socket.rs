use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};
use tauri::image::Image;

// Shared result buffer for IPC transcription results
pub type ResultBuffer = Arc<Mutex<Option<String>>>;

pub fn new_result_buffer() -> ResultBuffer {
    Arc::new(Mutex::new(None))
}

pub fn start_socket_server(app: AppHandle, result_buf: ResultBuffer) {
    let sock_path = socket_path();
    let _ = std::fs::remove_file(&sock_path);

    thread::spawn(move || {
        let listener = match UnixListener::bind(&sock_path) {
            Ok(l) => l,
            Err(e) => { eprintln!("[Kazamo] Socket bind failed: {}", e); return; }
        };
        eprintln!("[Kazamo] Socket listening: {}", sock_path);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let app = app.clone();
                    let buf = result_buf.clone();
                    thread::spawn(move || handle_client(stream, app, buf));
                }
                Err(e) => eprintln!("[Kazamo] Socket accept: {}", e),
            }
        }
    });
}

fn handle_client(mut stream: UnixStream, app: AppHandle, result_buf: ResultBuffer) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).is_ok() {
        let cmd = line.trim();
        match cmd {
            "toggle-start" => {
                // Clear previous result
                *result_buf.lock().unwrap() = None;
                let _ = app.emit("ipc-toggle-start", ());

                // Wait briefly for the UI to confirm whether recording really started.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    if let Ok(buf) = result_buf.lock() {
                        if let Some(ref text) = *buf {
                            let _ = stream.write_all(format!("{}\n", text).as_bytes());
                            return;
                        }
                    }
                    if std::time::Instant::now() > deadline {
                        let _ = stream.write_all(b"error\n");
                        return;
                    }
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            "toggle-stop" => {
                // Clear result, emit stop event
                *result_buf.lock().unwrap() = None;
                let _ = app.emit("ipc-toggle-stop", ());

                // Wait for transcription result (up to 60s)
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
                loop {
                    if let Ok(buf) = result_buf.lock() {
                        if let Some(ref text) = *buf {
                            let _ = stream.write_all(format!("{}\n", text).as_bytes());
                            return;
                        }
                    }
                    if std::time::Instant::now() > deadline {
                        let _ = stream.write_all(b"error\n");
                        return;
                    }
                    thread::sleep(std::time::Duration::from_millis(200));
                }
            }
            "focus-and-toggle" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
                let _ = app.emit("ipc-toggle", ());
                let _ = stream.write_all(b"ok\n");
            }
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
                let _ = stream.write_all(b"ok\n");
            }
            "status" => {
                let state = app.state::<crate::commands::AppState>();
                let status = if state.recorder._is_recording() {
                    "recording\n"
                } else {
                    "idle\n"
                };
                let _ = stream.write_all(status.as_bytes());
            }
            "set-recording" => {
                set_tray_recording(&app, true);
                let _ = stream.write_all(b"ok\n");
            }
            "set-idle" => {
                set_tray_recording(&app, false);
                let _ = stream.write_all(b"ok\n");
            }
            "ping" => { let _ = stream.write_all(b"pong\n"); }
            _ => { let _ = stream.write_all(b"?\n"); }
        }
    }
}

fn set_tray_recording(app: &AppHandle, recording: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = if recording {
            Image::from_bytes(include_bytes!("../icons/tray-recording.png"))
        } else {
            Image::from_bytes(include_bytes!("../icons/tray-mic.png"))
        };
        if let Ok(icon) = icon {
            if let Err(e) = tray.set_icon_with_as_template(Some(icon), false) {
                eprintln!("[Kazamo] Failed to update IPC tray icon: {}", e);
            }
        }
        if let Err(e) = tray.set_tooltip(Some(if recording { "Kazamo ● Recording" } else { "Kazamo" })) {
            eprintln!("[Kazamo] Failed to update IPC tray tooltip: {}", e);
        }
        eprintln!("[Kazamo] IPC tray state: {}", if recording { "recording" } else { "idle" });
    } else {
        eprintln!("[Kazamo] Tray main-tray not found for IPC recording={}", recording);
    }
}

pub fn socket_path() -> String {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join("kazamo.sock").to_string_lossy().to_string()
}
