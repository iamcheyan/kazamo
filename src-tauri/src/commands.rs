use crate::recording::Recorder;
use crate::settings::Settings;
use crate::transcription;
use crate::SharedState;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::image::Image;
use tauri::Manager;

pub struct AppState {
    pub recorder: Arc<Recorder>,
    pub resource_dir: PathBuf,
    pub settings: tokio::sync::Mutex<Settings>,
}

fn set_tray_icon(app: &tauri::AppHandle, recording: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = if recording {
            Image::from_bytes(include_bytes!("../icons/icon-recording.png"))
        } else {
            Image::from_bytes(include_bytes!("../icons/icon.png"))
        };
        if let Ok(icon) = icon {
            let _ = tray.set_icon(Some(icon));
        }
        let _ = tray.set_tooltip(Some(if recording { "Kazamo ● Recording" } else { "Kazamo" }));
    }
}

#[derive(Serialize)]
pub struct ModelInfo { pub name: String, pub downloaded: bool, pub path: String, pub size_mb: u64 }

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
pub async fn save_settings(state: tauri::State<'_, AppState>, language: String, provider: String, hotkey: String, theme: String) -> Result<(), String> {
    let mut s = state.settings.lock().await;
    s.language = language;
    s.provider = provider;
    s.hotkey = hotkey;
    s.theme = theme;
    s.save()
}

#[tauri::command]
pub async fn start_recording(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    state.recorder.start()?;
    set_tray_icon(&app, true);
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: tauri::State<'_, AppState>, app: tauri::AppHandle) -> Result<Vec<u8>, String> {
    let data = state.recorder.stop()?;
    set_tray_icon(&app, false);
    Ok(data)
}

#[tauri::command]
pub async fn list_models() -> Result<Vec<ModelInfo>, String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    Ok(vec![
        check_model("SenseVoice", &cache.join("sensevoice-models"), &["sense-voice-small-q4_k.gguf", "sense-voice-small-q4_1.gguf", "sense-voice-small-q5_0.gguf", "sense-voice-small-q8_0.gguf"]),
        check_model("Paraformer", &cache.join("paraformer-models").join("paraformer-large-zh"), &["model.onnx", "tokens.txt"]),
    ])
}

fn check_model(name: &str, dir: &std::path::Path, files: &[&str]) -> ModelInfo {
    if files.is_empty() { return ModelInfo { name: name.to_string(), downloaded: false, path: dir.to_string_lossy().to_string(), size_mb: 0 }; }
    let all_exist = files.iter().all(|f| dir.join(f).exists());
    if all_exist {
        // report size of the first file (e.g. the onnx)
        let path = dir.join(files[0]);
        let size = std::fs::metadata(&path).map(|m| m.len() / 1_000_000).unwrap_or(0);
        return ModelInfo { name: name.to_string(), downloaded: true, path: path.to_string_lossy().to_string(), size_mb: size };
    }
    ModelInfo { name: name.to_string(), downloaded: false, path: dir.to_string_lossy().to_string(), size_mb: 0 }
}

#[tauri::command]
pub async fn delete_model(name: String) -> Result<String, String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    let dir = match name.as_str() {
        "SenseVoice" => cache.join("sensevoice-models"),
        "Paraformer" => cache.join("paraformer-models").join("paraformer-large-zh"),
        _ => return Err(format!("Unknown model: {}", name)),
    };
    if !dir.exists() { return Err("Model directory not found".into()); }
    let mut total = 0u64;
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() { total += path.metadata().map(|m| m.len()).unwrap_or(0); std::fs::remove_file(&path).map_err(|e| e.to_string())?; }
    }
    Ok(format!("Deleted {} (freed {}MB)", name, total / 1_000_000))
}

#[tauri::command]
pub async fn open_model_dir(name: String) -> Result<(), String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    let dir = match name.as_str() {
        "SenseVoice" => cache.join("sensevoice-models"),
        "Paraformer" => cache.join("paraformer-models").join("paraformer-large-zh"),
        _ => return Err(format!("Unknown model: {}", name)),
    };
    let _ = std::fs::create_dir_all(&dir);
    opener::open(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transcribe_audio(state: tauri::State<'_, AppState>, audio_data: Vec<u8>) -> Result<serde_json::Value, String> {
    let settings = state.settings.lock().await.clone();
    let res_dir = state.resource_dir.clone();
    let result = match settings.provider.as_str() {
        "sensevoice" => {
            let binary = find_binary(&["sense-voice-main"], &res_dir).await;
            let model = find_model("sensevoice").await;
            match (binary, model) {
                (Some(b), Some(m)) => transcription::transcribe_sensevoice(&audio_data, &m, &b, &settings.language, &res_dir).await,
                (None, _) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("sense-voice-main not found.".into()) },
                (_, None) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("SenseVoice model not downloaded.".into()) },
            }
        }
        "paraformer" => {
            let binary = find_binary(&["sherpa-onnx-ws-linux-x64", "sherpa-onnx-ws"], &res_dir).await;
            let model = find_model("paraformer").await;
            eprintln!("[Kazamo] Paraformer: binary={:?}, model={:?}, res_dir={}", binary, model, res_dir.display());
            match (binary, model) {
                (Some(b), Some(m)) => {
                    match crate::paraformer::transcribe_paraformer(&audio_data, &m, &b, &res_dir).await {
                        Ok(text) => transcription::TranscriptionResult { success: true, text, error: None },
                        Err(e) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some(e) },
                    }
                }
                (None, _) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("sherpa-onnx-ws not found.".into()) },
                (_, None) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("Paraformer model not downloaded.".into()) },
            }
        }
        _ => transcription::TranscriptionResult { success: false, text: String::new(), error: Some(format!("Unknown provider: {}", settings.provider)) },
    };
    Ok(serde_json::json!({ "success": result.success, "text": result.text, "error": result.error }))
}

async fn find_binary(names: &[&str], resource_dir: &PathBuf) -> Option<String> {
    for &name in names {
        let bundled = resource_dir.join("bin").join(name);
        if tokio::fs::metadata(&bundled).await.is_ok() { return Some(bundled.to_string_lossy().to_string()); }
        if let Ok(out) = tokio::process::Command::new("which").arg(name).output().await {
            if out.status.success() { let p = String::from_utf8_lossy(&out.stdout).trim().to_string(); if !p.is_empty() { return Some(p); } }
        }
        let home = dirs::home_dir().unwrap_or_default();
        let p = home.join(".local/bin").join(name);
        if tokio::fs::metadata(&p).await.is_ok() { return Some(p.to_string_lossy().to_string()); }

        // Extra locations to support direct execution of debug binary (e.g. via kazamo launcher script)
        // which bypasses `tauri dev` resource staging. This ensures newly added binaries in resources/bin/
        // are discoverable without requiring re-copy to target/debug/resources/bin .
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                // Staged dev resources sibling to the exe: target/debug/resources/bin/...
                let p = dir.join("resources").join("bin").join(name);
                if tokio::fs::metadata(&p).await.is_ok() { return Some(p.to_string_lossy().to_string()); }
                // Source tree resources (for src-tauri/target/debug/... layout): src-tauri/resources/bin/...
                if let Some(base) = dir.parent().and_then(|p| p.parent()) {
                    let p = base.join("resources").join("bin").join(name);
                    if tokio::fs::metadata(&p).await.is_ok() { return Some(p.to_string_lossy().to_string()); }
                }
            }
        }
    }
    None
}

async fn find_model(model_type: &str) -> Option<String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    let paths: Vec<PathBuf> = match model_type {
        "sensevoice" => { let d = cache.join("sensevoice-models"); vec![d.join("sense-voice-small-q4_k.gguf"), d.join("sense-voice-small-q8_0.gguf")] }
        "paraformer" => vec![cache.join("paraformer-models").join("paraformer-large-zh").join("model.onnx")],
        _ => vec![],
    };
    for p in paths {
        if tokio::fs::metadata(&p).await.is_ok() {
            if model_type == "paraformer" {
                if let Some(parent) = p.parent() {
                    let tokens = parent.join("tokens.txt");
                    if tokio::fs::metadata(&tokens).await.is_ok() {
                        return Some(parent.to_string_lossy().to_string());
                    } else {
                        // model present but incomplete (missing tokens.txt); treat as not ready
                        continue;
                    }
                }
            }
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

// ── IPC Result (called by frontend after transcription) ──

#[tauri::command]
pub async fn set_ipc_result(state: tauri::State<'_, SharedState>, text: String) -> Result<(), String> {
    *state.ipc_result.lock().unwrap() = Some(text);
    Ok(())
}

// ── Model Download ──

#[tauri::command]
pub async fn download_model(name: String, app: tauri::AppHandle) -> Result<String, String> {
    use tauri::Emitter;
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");

    let (dir, url, filename) = match name.as_str() {
        "SenseVoice" => {
            let dir = cache.join("sensevoice-models");
            (dir, "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q4_k.gguf", "sense-voice-small-q4_k.gguf")
        }
        "Paraformer" => {
            // Paraformer is a directory with multiple files - download model.onnx + tokens.txt
            let dir = cache.join("paraformer-models").join("paraformer-large-zh");
            return download_paraformer(&dir, &app).await;
        }
        _ => return Err(format!("Unknown model: {}", name)),
    };

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(filename);
    if dest.exists() {
        return Ok(format!("{} already downloaded", name));
    }

    eprintln!("[Kazamo] Downloading {} to {}", name, dest.display());
    let _ = app.emit("download-progress", serde_json::json!({ "model": name, "status": "downloading", "percent": 0 }));

    let client = reqwest::Client::new();
    let resp = client.get(url).send().await.map_err(|e| format!("Download failed: {}", e))?;
    let total = resp.content_length().unwrap_or(0);

    let mut file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    use std::io::Write;
    let mut downloaded: u64 = 0;

    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        if total > 0 {
            let pct = (downloaded * 100 / total) as u32;
            let _ = app.emit("download-progress", serde_json::json!({ "model": name, "status": "downloading", "percent": pct }));
        }
    }

    let _ = app.emit("download-progress", serde_json::json!({ "model": name, "status": "complete", "percent": 100 }));
    Ok(format!("{} downloaded ({}MB)", name, downloaded / 1_000_000))
}

async fn download_paraformer(dir: &std::path::Path, app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Emitter;
    use std::io::Write;
    use futures_util::StreamExt;

    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let files = vec![
        ("https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-2023-09-14.tar.bz2", "model.onnx"),
    ];

    // For simplicity, download model.onnx directly if available
    let model_url = "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2023-09-14/resolve/main/model.onnx";
    let tokens_url = "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2023-09-14/resolve/main/tokens.txt";

    let client = reqwest::Client::new();

    for (url, filename) in vec![(model_url, "model.onnx"), (tokens_url, "tokens.txt")] {
        let dest = dir.join(filename);
        if dest.exists() { continue; }

        eprintln!("[Kazamo] Downloading Paraformer/{}", filename);
        let _ = app.emit("download-progress", serde_json::json!({ "model": "Paraformer", "status": "downloading", "file": filename }));

        let resp = client.get(url).send().await.map_err(|e| format!("Download {} failed: {}", filename, e))?;
        if !resp.status().is_success() {
            return Err(format!("Download {} failed: HTTP {}", filename, resp.status()));
        }
        let total = resp.content_length().unwrap_or(0);
        let mut file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let pct = (downloaded * 100 / total) as u32;
                let _ = app.emit("download-progress", serde_json::json!({ "model": "Paraformer", "status": "downloading", "file": filename, "percent": pct }));
            }
        }
    }

    let _ = app.emit("download-progress", serde_json::json!({ "model": "Paraformer", "status": "complete", "percent": 100 }));
    Ok("Paraformer downloaded".to_string())
}
