use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub success: bool,
    pub text: String,
    pub error: Option<String>,
}

struct SenseVoiceProc {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

static SENSEVOICE_PROC: LazyLock<Arc<Mutex<Option<SenseVoiceProc>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

async fn get_or_start_sv_proc(
    model_path: &str,
    tokens_path: &str,
    language: &str,
    resource_dir: &Path,
) -> Result<Arc<Mutex<Option<SenseVoiceProc>>>, String> {
    {
        let proc = SENSEVOICE_PROC.lock().await;
        if proc.is_some() {
            return Ok(Arc::clone(&SENSEVOICE_PROC));
        }
    }

    let script_name = "sensevoice-offline.py";
    let mut script_candidates = vec![resource_dir.join("bin").join(script_name)];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            script_candidates.push(dir.join("resources").join("bin").join(script_name));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        script_candidates.push(cwd.join("resources").join("bin").join(script_name));
        script_candidates.push(cwd.join("src-tauri").join("resources").join("bin").join(script_name));
    }
    let script = script_candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("sensevoice-offline.py not found (searched: {:?})", script_candidates))?;

    let mut child = Command::new("python3")
        .arg(script)
        .arg(model_path)
        .arg(tokens_path)
        .arg(language)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start sensevoice-offline.py: {}", e))?;

    let stdin = child.stdin.take().ok_or("No stdin")?;
    let stdout = child.stdout.take().ok_or("No stdout")?;
    let mut stdout = BufReader::new(stdout);

    let mut line = String::new();
    stdout.read_line(&mut line).await.map_err(|e| format!("Read ready: {}", e))?;
    let line = line.trim();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if json.get("error").is_some() {
            let _ = child.kill().await;
            return Err(format!("SenseVoice init failed: {}", line));
        }
    }

    eprintln!("[Kazamo] SenseVoice: process ready");
    let mut proc = SENSEVOICE_PROC.lock().await;
    *proc = Some(SenseVoiceProc { child, stdin, stdout });
    drop(proc);
    Ok(Arc::clone(&SENSEVOICE_PROC))
}

pub async fn transcribe_sensevoice(
    audio_data: &[u8],
    model_path: &str,
    _binary_path: &str,
    language: &str,
    resource_dir: &Path,
) -> TranscriptionResult {
    let tmp_in = format!("/tmp/kazamo-sv-in-{}.wav", std::process::id());
    let tmp_out = format!("/tmp/kazamo-sv-16k-{}.wav", std::process::id());

    if let Err(e) = tokio::fs::write(&tmp_in, audio_data).await {
        return err(&format!("Write failed: {}", e));
    }

    let status = Command::new("ffmpeg")
        .args(["-y", "-i", &tmp_in, "-ar", "16000", "-ac", "1", "-af", "volume=20dB", "-f", "wav", &tmp_out])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .status().await;

    let _ = tokio::fs::remove_file(&tmp_in).await;

    match status {
        Ok(s) if !s.success() => {
            let _ = tokio::fs::remove_file(&tmp_out).await;
            return err("ffmpeg conversion failed");
        }
        Err(e) => return err(&format!("ffmpeg not found: {}", e)),
        _ => {}
    }

    // Find tokens.txt in model directory
    let model_dir = Path::new(model_path).parent().unwrap_or(Path::new("."));
    let tokens_path = model_dir.join("tokens.txt");
    if !tokens_path.exists() {
        let _ = tokio::fs::remove_file(&tmp_out).await;
        return err(&format!("tokens.txt not found in {:?}", model_dir));
    }

    let lang = match language { "zh"|"en"|"ja"|"ko"|"yue" => language, _ => "auto" };

    let proc_ref = match get_or_start_sv_proc(
        model_path,
        &tokens_path.to_string_lossy(),
        lang,
        resource_dir,
    ).await {
        Ok(r) => r,
        Err(e) => { let _ = tokio::fs::remove_file(&tmp_out).await; return err(&e); }
    };

    let mut proc = proc_ref.lock().await;
    let proc = match proc.as_mut() {
        Some(p) => p,
        None => { let _ = tokio::fs::remove_file(&tmp_out).await; return err("Process not started"); }
    };

    if let Err(e) = proc.stdin.write_all(format!("{}\n", tmp_out).as_bytes()).await {
        let _ = tokio::fs::remove_file(&tmp_out).await;
        return err(&format!("Write stdin: {}", e));
    }

    let mut line = String::new();
    if let Err(e) = proc.stdout.read_line(&mut line).await {
        let _ = tokio::fs::remove_file(&tmp_out).await;
        return err(&format!("Read stdout: {}", e));
    }

    let _ = tokio::fs::remove_file(&tmp_out).await;

    let line = line.trim();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(err_msg) = json.get("error").and_then(|e| e.as_str()) {
            return err(err_msg);
        }
        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
            let text = text.trim().to_string();
            if text.is_empty() {
                return err("No speech detected");
            }
            return TranscriptionResult { success: true, text, error: None };
        }
    }

    err(&format!("Unexpected output: {}", line))
}

fn err(msg: &str) -> TranscriptionResult {
    TranscriptionResult { success: false, text: String::new(), error: Some(msg.to_string()) }
}
