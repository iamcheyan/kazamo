use crate::recording::Recorder;
use crate::settings::Settings;
use crate::transcription;
use crate::SharedState;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::image::Image;
use tauri::Manager;
use tokio::io::AsyncWriteExt;

pub struct AppState {
    pub recorder: Arc<Recorder>,
    pub resource_dir: PathBuf,
    pub settings: tokio::sync::Mutex<Settings>,
}

fn set_tray_icon(app: &tauri::AppHandle, recording: bool) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let icon = if recording {
            Image::from_bytes(include_bytes!("../icons/tray-recording.png"))
        } else {
            Image::from_bytes(include_bytes!("../icons/tray-mic.png"))
        };
        if let Ok(icon) = icon {
            if let Err(e) = tray.set_icon_with_as_template(Some(icon), false) {
                eprintln!("[Kazamo] Failed to update tray icon: {}", e);
            }
        }
        let tooltip = if recording { "Kazamo ● Recording" } else { "Kazamo" };
        if let Err(e) = tray.set_tooltip(Some(tooltip)) {
            eprintln!("[Kazamo] Failed to update tray tooltip: {}", e);
        }
        eprintln!("[Kazamo] Tray state: {}", if recording { "recording" } else { "idle" });
    } else {
        eprintln!("[Kazamo] Tray main-tray not found while setting recording={}", recording);
    }
}

#[derive(Serialize)]
pub struct ModelInfo { pub name: String, pub downloaded: bool, pub path: String, pub size_mb: u64 }

const SENSEVOICE_ONNX_MODEL: &str = "SenseVoice Small ONNX INT8";

fn is_aarch64() -> bool {
    std::env::consts::ARCH == "aarch64"
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
pub async fn save_settings(
    state: tauri::State<'_, AppState>,
    language: String,
    provider: String,
    hotkey: String,
    theme: String,
    sensevoice_model: Option<String>,
    paraformer_model: Option<String>,
) -> Result<(), String> {
    let mut s = state.settings.lock().await;
    s.language = language;
    s.provider = provider;
    s.hotkey = hotkey;
    s.theme = theme;
    if let Some(m) = sensevoice_model {
        s.sensevoice_model = m;
    }
    if let Some(m) = paraformer_model {
        s.paraformer_model = m;
    }
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
    let mut models = Vec::new();
    if is_aarch64() {
        models.push(check_model(SENSEVOICE_ONNX_MODEL, &cache.join("sensevoice-onnx-models").join("sense-voice-small-int8"), &["model.int8.onnx", "tokens.txt"]));
    } else {
        models.extend([
            check_model("SenseVoice Small Q3_K", &cache.join("sensevoice-models"), &["sense-voice-small-q3_k.gguf"]),
            check_model("SenseVoice Small Q4_0", &cache.join("sensevoice-models"), &["sense-voice-small-q4_0.gguf"]),
            check_model("SenseVoice Small Q4_1", &cache.join("sensevoice-models"), &["sense-voice-small-q4_1.gguf"]),
            check_model("SenseVoice Small Q4_K", &cache.join("sensevoice-models"), &["sense-voice-small-q4_k.gguf"]),
            check_model("SenseVoice Small Q5_0", &cache.join("sensevoice-models"), &["sense-voice-small-q5_0.gguf"]),
            check_model("SenseVoice Small Q5_K", &cache.join("sensevoice-models"), &["sense-voice-small-q5_k.gguf"]),
            check_model("SenseVoice Small Q6_K", &cache.join("sensevoice-models"), &["sense-voice-small-q6_k.gguf"]),
            check_model("SenseVoice Small Q8_0", &cache.join("sensevoice-models"), &["sense-voice-small-q8_0.gguf"]),
            check_model("SenseVoice Small FP16", &cache.join("sensevoice-models"), &["sense-voice-small-fp16.gguf"]),
            check_model("SenseVoice Small FP32", &cache.join("sensevoice-models"), &["sense-voice-small-fp32.gguf"]),
        ]);
    }
    models.push(check_model("Paraformer-Large", &cache.join("paraformer-models").join("paraformer-large-zh"), &["model.onnx", "tokens.txt"]));
    Ok(models)
}

fn check_model(name: &str, dir: &std::path::Path, files: &[&str]) -> ModelInfo {
    if files.is_empty() { return ModelInfo { name: name.to_string(), downloaded: false, path: dir.to_string_lossy().to_string(), size_mb: 0 }; }
    let is_downloaded = files.iter().all(|f| dir.join(f).exists());
    if is_downloaded {
        if let Some(path) = files.iter().map(|f| dir.join(f)).find(|p| p.exists()) {
            let size = std::fs::metadata(&path).map(|m| m.len() / 1_000_000).unwrap_or(0);
            return ModelInfo { name: name.to_string(), downloaded: true, path: path.to_string_lossy().to_string(), size_mb: size };
        }
    }
    ModelInfo { name: name.to_string(), downloaded: false, path: dir.to_string_lossy().to_string(), size_mb: 0 }
}

#[tauri::command]
pub async fn delete_model(name: String) -> Result<String, String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    let (dir, files) = match name.as_str() {
        SENSEVOICE_ONNX_MODEL => (cache.join("sensevoice-onnx-models").join("sense-voice-small-int8"), vec!["model.int8.onnx", "tokens.txt"]),
        "SenseVoice Small Q3_K" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q3_k.gguf"]),
        "SenseVoice Small Q4_0" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q4_0.gguf"]),
        "SenseVoice Small Q4_1" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q4_1.gguf"]),
        "SenseVoice Small Q4_K" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q4_k.gguf"]),
        "SenseVoice Small Q5_0" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q5_0.gguf"]),
        "SenseVoice Small Q5_K" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q5_k.gguf"]),
        "SenseVoice Small Q6_K" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q6_k.gguf"]),
        "SenseVoice Small Q8_0" => (cache.join("sensevoice-models"), vec!["sense-voice-small-q8_0.gguf"]),
        "SenseVoice Small FP16" => (cache.join("sensevoice-models"), vec!["sense-voice-small-fp16.gguf"]),
        "SenseVoice Small FP32" => (cache.join("sensevoice-models"), vec!["sense-voice-small-fp32.gguf"]),
        "Paraformer-Large" => (cache.join("paraformer-models").join("paraformer-large-zh"), vec!["model.onnx", "tokens.txt"]),
        _ => return Err(format!("Unknown model: {}", name)),
    };
    if !dir.exists() { return Err("Model directory not found".into()); }
    
    let mut total = 0u64;
    for file in &files {
        let path = dir.join(file);
        if path.exists() {
            total += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    
    // Clean up empty directories
    if let Ok(mut entries) = std::fs::read_dir(&dir) {
        if entries.next().is_none() {
            let _ = std::fs::remove_dir(&dir);
        }
    }
    
    Ok(format!("Deleted {} (freed {}MB)", name, total / 1_000_000))
}

#[tauri::command]
pub async fn open_model_dir(name: String) -> Result<(), String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    let dir = if name == SENSEVOICE_ONNX_MODEL {
        cache.join("sensevoice-onnx-models").join("sense-voice-small-int8")
    } else if name.starts_with("SenseVoice") {
        cache.join("sensevoice-models")
    } else if name == "Paraformer-Large" {
        cache.join("paraformer-models").join("paraformer-large-zh")
    } else {
        return Err(format!("Unknown model: {}", name));
    };
    let _ = std::fs::create_dir_all(&dir);
    opener::open(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transcribe_audio(state: tauri::State<'_, AppState>, audio_data: Vec<u8>) -> Result<serde_json::Value, String> {
    let result = transcribe_audio_inner(&state, audio_data).await;
    Ok(serde_json::json!({ "success": result.success, "text": result.text, "error": result.error }))
}

async fn transcribe_audio_inner(state: &tauri::State<'_, AppState>, audio_data: Vec<u8>) -> transcription::TranscriptionResult {
    let settings = state.settings.lock().await.clone();
    let res_dir = state.resource_dir.clone();
    match settings.provider.as_str() {
        "sensevoice" => {
            let model = find_model("sensevoice", &settings.sensevoice_model).await;
            match model {
                Some(m) if settings.sensevoice_model == SENSEVOICE_ONNX_MODEL || is_aarch64() => {
                    match crate::paraformer::transcribe_sensevoice_onnx(&audio_data, &m, &settings.language, &res_dir).await {
                        Ok(text) => transcription::TranscriptionResult { success: true, text, error: None },
                        Err(e) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some(e) },
                    }
                }
                Some(m) => {
                    let binary = find_binary(&["sense-voice-main"], &res_dir).await;
                    match binary {
                        Some(b) => transcription::transcribe_sensevoice(&audio_data, &m, &b, &settings.language, &res_dir).await,
                        None => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("sense-voice-main not found.".into()) },
                    }
                }
                None => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("SenseVoice model not downloaded.".into()) },
            }
        }
        "paraformer" => {
            let model = find_model("paraformer", &settings.paraformer_model).await;
            eprintln!("[Kazamo] Paraformer: model={:?}, res_dir={}", model, res_dir.display());
            match model {
                Some(m) => {
                    match crate::paraformer::transcribe_paraformer(&audio_data, &m, "", &res_dir).await {
                        Ok(text) => transcription::TranscriptionResult { success: true, text, error: None },
                        Err(e) => transcription::TranscriptionResult { success: false, text: String::new(), error: Some(e) },
                    }
                }
                None => transcription::TranscriptionResult { success: false, text: String::new(), error: Some("Paraformer model not downloaded.".into()) },
            }
        }
        _ => transcription::TranscriptionResult { success: false, text: String::new(), error: Some(format!("Unknown provider: {}", settings.provider)) },
    }
}

pub async fn toggle_tray_dictation(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if !state.recorder._is_recording() {
        state.recorder.start()?;
        set_tray_icon(&app, true);
        return Ok(());
    }

    let data = state.recorder.stop()?;
    set_tray_icon(&app, false);
    let result = transcribe_audio_inner(&state, data).await;
    if !result.success {
        return Err(result.error.unwrap_or_else(|| "Transcription failed".into()));
    }

    copy_and_paste(&result.text).await
}

async fn copy_and_paste(text: &str) -> Result<(), String> {
    copy_to_clipboard(text).await?;
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    if try_wtype_paste().await.is_ok() || try_wtype_text(text).await.is_ok() || try_ydotool_paste().await.is_ok() {
        return Ok(());
    }
    let wtype_err = try_wtype_paste().await.err().unwrap_or_else(|| "not attempted".into());
    let ydotool_err = try_ydotool_paste().await.err().unwrap_or_else(|| "not attempted".into());
    eprintln!("[Kazamo] Auto paste failed: wtype={}, ydotool={}", wtype_err, ydotool_err);
    Err("Copied to clipboard, but automatic paste failed. Install/use wtype or start ydotoold.".into())
}

async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut child = tokio::process::Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start wl-copy: {}", e))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes()).await.map_err(|e| format!("Failed to write clipboard: {}", e))?;
    }
    let status = child.wait().await.map_err(|e| format!("wl-copy failed: {}", e))?;
    if status.success() { Ok(()) } else { Err(format!("wl-copy exited with {}", status)) }
}

async fn try_wtype_paste() -> Result<(), String> {
    let output = tokio::process::Command::new("wtype")
        .args(["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"])
        .output()
        .await
        .map_err(|e| format!("wtype failed to start: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("wtype exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr).trim()))
    }
}

async fn try_wtype_text(text: &str) -> Result<(), String> {
    let output = tokio::process::Command::new("wtype")
        .arg(text)
        .output()
        .await
        .map_err(|e| format!("wtype text failed to start: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("wtype text exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr).trim()))
    }
}

async fn try_ydotool_paste() -> Result<(), String> {
    let socket = std::env::var("YDOTOOL_SOCKET").unwrap_or_else(|_| {
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::geteuid() }));
        format!("{}/.ydotool_socket", runtime)
    });
    if !std::path::Path::new(&socket).exists() {
        return Err(format!("ydotool socket not found: {}", socket));
    }
    let output = tokio::process::Command::new("ydotool")
        .args(["key", "29:1", "47:1", "47:0", "29:0"])
        .output()
        .await
        .map_err(|e| format!("ydotool failed to start: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("ydotool exited with {}: {}", output.status, String::from_utf8_lossy(&output.stderr).trim()))
    }
}

async fn find_binary(names: &[&str], resource_dir: &PathBuf) -> Option<String> {
    for &name in names {
        let bundled = resource_dir.join("bin").join(name);
        if tokio::fs::metadata(&bundled).await.is_ok() { return Some(bundled.to_string_lossy().to_string()); }
        let bundled2 = resource_dir.join("resources").join("bin").join(name);
        if tokio::fs::metadata(&bundled2).await.is_ok() { return Some(bundled2.to_string_lossy().to_string()); }
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

async fn find_model(model_type: &str, active_model_name: &str) -> Option<String> {
    let home = dirs::home_dir().unwrap_or_default();
    let cache = home.join(".cache").join("chordvoxmini");
    let paths: Vec<PathBuf> = match (model_type, active_model_name) {
        ("sensevoice", SENSEVOICE_ONNX_MODEL) => vec![cache.join("sensevoice-onnx-models").join("sense-voice-small-int8").join("model.int8.onnx")],
        ("sensevoice", _) if is_aarch64() => vec![cache.join("sensevoice-onnx-models").join("sense-voice-small-int8").join("model.int8.onnx")],
        // SenseVoice variants - match the exact names from list_models and UI selection
        ("sensevoice", "SenseVoice Small Q3_K") => vec![cache.join("sensevoice-models").join("sense-voice-small-q3_k.gguf")],
        ("sensevoice", "SenseVoice Small Q4_0") => vec![cache.join("sensevoice-models").join("sense-voice-small-q4_0.gguf")],
        ("sensevoice", "SenseVoice Small Q4_1") => vec![cache.join("sensevoice-models").join("sense-voice-small-q4_1.gguf")],
        ("sensevoice", "SenseVoice Small Q4_K") => vec![cache.join("sensevoice-models").join("sense-voice-small-q4_k.gguf")],
        ("sensevoice", "SenseVoice Small Q5_0") => vec![cache.join("sensevoice-models").join("sense-voice-small-q5_0.gguf")],
        ("sensevoice", "SenseVoice Small Q5_K") => vec![cache.join("sensevoice-models").join("sense-voice-small-q5_k.gguf")],
        ("sensevoice", "SenseVoice Small Q6_K") => vec![cache.join("sensevoice-models").join("sense-voice-small-q6_k.gguf")],
        ("sensevoice", "SenseVoice Small Q8_0") => vec![cache.join("sensevoice-models").join("sense-voice-small-q8_0.gguf")],
        ("sensevoice", "SenseVoice Small FP16") => vec![cache.join("sensevoice-models").join("sense-voice-small-fp16.gguf")],
        ("sensevoice", "SenseVoice Small FP32") => vec![cache.join("sensevoice-models").join("sense-voice-small-fp32.gguf")],
        ("sensevoice", _) => {
            // Fallback for unknown or legacy names: prefer Q4_K then Q8_0
            let d = cache.join("sensevoice-models");
            vec![d.join("sense-voice-small-q4_k.gguf"), d.join("sense-voice-small-q8_0.gguf")]
        }
        ("paraformer", _) => vec![cache.join("paraformer-models").join("paraformer-large-zh").join("model.onnx")],
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
            if model_type == "sensevoice" {
                if let Some(parent) = p.parent() {
                    let tokens = parent.join("tokens.txt");
                    if tokio::fs::metadata(&tokens).await.is_err() {
                        continue; // missing tokens.txt, not ready
                    }
                    if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".onnx")) {
                        return Some(parent.to_string_lossy().to_string());
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
        SENSEVOICE_ONNX_MODEL => {
            let dir = cache.join("sensevoice-onnx-models").join("sense-voice-small-int8");
            return download_sensevoice_onnx(&dir, &app).await;
        }
        "SenseVoice Small Q3_K" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q3_k.gguf", "sense-voice-small-q3_k.gguf"),
        "SenseVoice Small Q4_0" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q4_0.gguf", "sense-voice-small-q4_0.gguf"),
        "SenseVoice Small Q4_1" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q4_1.gguf", "sense-voice-small-q4_1.gguf"),
        "SenseVoice Small Q4_K" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q4_k.gguf", "sense-voice-small-q4_k.gguf"),
        "SenseVoice Small Q5_0" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q5_0.gguf", "sense-voice-small-q5_0.gguf"),
        "SenseVoice Small Q5_K" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q5_k.gguf", "sense-voice-small-q5_k.gguf"),
        "SenseVoice Small Q6_K" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q6_k.gguf", "sense-voice-small-q6_k.gguf"),
        "SenseVoice Small Q8_0" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-q8_0.gguf", "sense-voice-small-q8_0.gguf"),
        "SenseVoice Small FP16" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-fp16.gguf", "sense-voice-small-fp16.gguf"),
        "SenseVoice Small FP32" => (cache.join("sensevoice-models"), "https://huggingface.co/lovemefan/sense-voice-gguf/resolve/main/sense-voice-small-fp32.gguf", "sense-voice-small-fp32.gguf"),
        "Paraformer-Large" => {
            let dir = cache.join("paraformer-models").join("paraformer-large-zh");
            return download_paraformer(&dir, &app).await;
        }
        _ => return Err(format!("Unknown model: {}", name)),
    };

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(filename);
    if dest.exists() {
        // Ensure tokens.txt is also present for SenseVoice models
        let client = reqwest::Client::new();
        ensure_sensevoice_tokens(&dir, &client).await?;
        return Ok(format!("{} already downloaded", name));
    }

    let temp = dir.join(format!("{}.part", filename));
    // Clean any previous partial
    let _ = std::fs::remove_file(&temp);

    eprintln!("[Kazamo] Downloading {} to {} (via {})", name, dest.display(), temp.display());
    let _ = app.emit("download-progress", serde_json::json!({ "model": name, "status": "downloading", "percent": 0 }));

    let client = reqwest::Client::new();
    let resp = client.get(url).send().await.map_err(|e| format!("Download failed: {}", e))?;
    let total = resp.content_length().unwrap_or(0);

    let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
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

    // Atomic move: only now the final file appears
    std::fs::rename(&temp, &dest).map_err(|e| e.to_string())?;

    // Also download tokens.txt for SenseVoice models
    ensure_sensevoice_tokens(&dir, &client).await?;

    let _ = app.emit("download-progress", serde_json::json!({ "model": name, "status": "complete", "percent": 100 }));
    Ok(format!("{} downloaded ({}MB)", name, downloaded / 1_000_000))
}

/// Download tokens.txt for SenseVoice GGUF models (shared across all variants)
async fn ensure_sensevoice_tokens(dir: &std::path::Path, client: &reqwest::Client) -> Result<(), String> {
    use std::io::Write;
    let tokens_dest = dir.join("tokens.txt");
    if tokens_dest.exists() { return Ok(()); }

    let url = "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt";
    let resp = client.get(url).send().await.map_err(|e| format!("Download tokens.txt failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Download tokens.txt failed: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("Download tokens.txt error: {}", e))?;
    let mut file = std::fs::File::create(&tokens_dest).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    eprintln!("[Kazamo] Downloaded tokens.txt to {}", tokens_dest.display());
    Ok(())
}

async fn download_sensevoice_onnx(dir: &std::path::Path, app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Emitter;
    use std::io::Write;
    use futures_util::StreamExt;

    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let files = [
        (
            "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.int8.onnx",
            "model.int8.onnx",
        ),
        (
            "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt",
            "tokens.txt",
        ),
    ];
    let client = reqwest::Client::new();

    for (url, filename) in files {
        let dest = dir.join(filename);
        if dest.exists() { continue; }

        let temp = dir.join(format!("{}.part", filename));
        let _ = std::fs::remove_file(&temp);
        eprintln!("[Kazamo] Downloading SenseVoice ONNX/{}", filename);
        let _ = app.emit("download-progress", serde_json::json!({ "model": SENSEVOICE_ONNX_MODEL, "status": "downloading", "file": filename }));

        let resp = client.get(url).send().await.map_err(|e| format!("Download {} failed: {}", filename, e))?;
        if !resp.status().is_success() {
            return Err(format!("Download {} failed: HTTP {}", filename, resp.status()));
        }
        let total = resp.content_length().unwrap_or(0);
        let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let pct = (downloaded * 100 / total) as u32;
                let _ = app.emit("download-progress", serde_json::json!({ "model": SENSEVOICE_ONNX_MODEL, "status": "downloading", "file": filename, "percent": pct }));
            }
        }
        std::fs::rename(&temp, &dest).map_err(|e| e.to_string())?;
    }

    let _ = app.emit("download-progress", serde_json::json!({ "model": SENSEVOICE_ONNX_MODEL, "status": "complete", "percent": 100 }));
    Ok("SenseVoice ONNX downloaded".to_string())
}

async fn download_paraformer(dir: &std::path::Path, app: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Emitter;
    use std::io::Write;
    use futures_util::StreamExt;

    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    // For simplicity, download model.onnx directly if available
    let model_url = "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2023-09-14/resolve/main/model.int8.onnx";
    let tokens_url = "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2023-09-14/resolve/main/tokens.txt";

    let client = reqwest::Client::new();

    for (url, filename) in vec![(model_url, "model.onnx"), (tokens_url, "tokens.txt")] {
        let dest = dir.join(filename);
        if dest.exists() { continue; }

        let temp = dir.join(format!("{}.part", filename));
        let _ = std::fs::remove_file(&temp);

        eprintln!("[Kazamo] Downloading Paraformer/{}", filename);
        let _ = app.emit("download-progress", serde_json::json!({ "model": "Paraformer-Large", "status": "downloading", "file": filename }));

        let resp = client.get(url).send().await.map_err(|e| format!("Download {} failed: {}", filename, e))?;
        if !resp.status().is_success() {
            return Err(format!("Download {} failed: HTTP {}", filename, resp.status()));
        }
        let total = resp.content_length().unwrap_or(0);
        let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk).map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let pct = (downloaded * 100 / total) as u32;
                let _ = app.emit("download-progress", serde_json::json!({ "model": "Paraformer-Large", "status": "downloading", "file": filename, "percent": pct }));
            }
        }

        // Only promote to final name when this file is fully written
        std::fs::rename(&temp, &dest).map_err(|e| e.to_string())?;
    }

    let _ = app.emit("download-progress", serde_json::json!({ "model": "Paraformer-Large", "status": "complete", "percent": 100 }));
    Ok("Paraformer downloaded".to_string())
}
