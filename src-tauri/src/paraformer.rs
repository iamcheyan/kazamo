use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;

/// 全局常驻的 Paraformer 进程
static PARAFORMER_PROC: LazyLock<Arc<Mutex<Option<ParaformerProc>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

struct ParaformerProc {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

async fn get_or_start_proc(
    model_dir: &str,
    resource_dir: &std::path::Path,
) -> Result<&'static Arc<Mutex<Option<ParaformerProc>>>, String> {
    let mut proc = PARAFORMER_PROC.lock().await;
    if proc.is_some() {
        return Ok(&*Box::leak(Box::new(Arc::clone(&PARAFORMER_PROC))));
    }

    let script_name = "paraformer-offline.py";
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
        .ok_or_else(|| format!("paraformer-offline.py not found (searched: {:?})", script_candidates))?;

    let model_path = format!("{}/model.onnx", model_dir);
    let tokens_path = format!("{}/tokens.txt", model_dir);

    let mut child = Command::new("python3")
        .arg(script)
        .arg(&model_path)
        .arg(&tokens_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start paraformer-offline.py: {}", e))?;

    let stdin = child.stdin.take().ok_or("No stdin")?;
    let stdout = child.stdout.take().ok_or("No stdout")?;
    let mut stdout = BufReader::new(stdout);

    // 等待 "ready" 信号
    let mut line = String::new();
    stdout.read_line(&mut line).await.map_err(|e| format!("Read ready: {}", e))?;
    let line = line.trim();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if json.get("error").is_some() {
            let _ = child.kill().await;
            return Err(format!("Paraformer init failed: {}", line));
        }
    }

    eprintln!("[Kazamo] Paraformer: process ready");
    *proc = Some(ParaformerProc { child, stdin, stdout });
    drop(proc);
    Ok(&*Box::leak(Box::new(Arc::clone(&PARAFORMER_PROC))))
}

/// Transcribe audio using sherpa-onnx Python API (Paraformer)
pub async fn transcribe_paraformer(
    audio_data: &[u8],
    model_dir: &str,
    _binary_path: &str,
    resource_dir: &std::path::Path,
) -> Result<String, String> {
    // Convert to 16kHz mono WAV
    let tmp_wav = format!("/tmp/kazamo-pf-16k-{}.wav", std::process::id());
    let tmp_in = format!("/tmp/kazamo-pf-in-{}.wav", std::process::id());
    tokio::fs::write(&tmp_in, audio_data).await.map_err(|e| format!("Write: {}", e))?;

    let status = Command::new("ffmpeg")
        .args(["-y", "-i", &tmp_in, "-ar", "16000", "-ac", "1", "-af", "volume=20dB", "-f", "wav", &tmp_wav])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .status().await;
    let _ = tokio::fs::remove_file(&tmp_in).await;

    match status {
        Ok(s) if !s.success() => { let _ = tokio::fs::remove_file(&tmp_wav).await; return Err("ffmpeg failed".into()); }
        Err(e) => return Err(format!("ffmpeg: {}", e)),
        _ => {}
    }

    // Get or start the persistent process
    let proc_ref = get_or_start_proc(model_dir, resource_dir).await?;
    let mut proc = proc_ref.lock().await;
    let proc = proc.as_mut().ok_or("Process not started")?;

    // Send WAV path via stdin
    proc.stdin
        .write_all(format!("{}\n", tmp_wav).as_bytes())
        .await
        .map_err(|e| format!("Write stdin: {}", e))?;

    // Read result from stdout
    let mut line = String::new();
    proc.stdout
        .read_line(&mut line)
        .await
        .map_err(|e| format!("Read stdout: {}", e))?;

    let _ = tokio::fs::remove_file(&tmp_wav).await;

    let line = line.trim();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }
        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
            let text = text.trim().to_string();
            if text.is_empty() {
                return Err("No speech detected".into());
            }
            return Ok(text);
        }
    }

    Err(format!("Unexpected output: {}", line))
}
