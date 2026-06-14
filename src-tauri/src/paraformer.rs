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
    stderr_task: Option<tokio::task::JoinHandle<()>>,
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
    let mut script_candidates = vec![
        resource_dir.join("bin").join(script_name),
        resource_dir.join("resources").join("bin").join(script_name),
    ];
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

    let mut child = Command::new("python3");
    child.arg(script).arg(&model_path).arg(&tokens_path)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    // env_clear + explicit vars: prevents AppImage env from breaking Python's
    // module resolution (encodings, sherpa_onnx, etc.)
    child.env_clear();
    child.env("HOME", std::env::var("HOME").unwrap_or_default());
    child.env("PATH", std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()));
    child.env("USER", std::env::var("USER").unwrap_or_default());
    child.env("LANG", std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".into()));
    child.env("TERM", std::env::var("TERM").unwrap_or_default());
    if let Ok(v) = std::env::var("DISPLAY") { child.env("DISPLAY", v); }
    if let Ok(v) = std::env::var("WAYLAND_DISPLAY") { child.env("WAYLAND_DISPLAY", v); }
    if let Ok(v) = std::env::var("XDG_RUNTIME_DIR") { child.env("XDG_RUNTIME_DIR", v); }
    let mut child = child.spawn()
        .map_err(|e| format!("Failed to start paraformer-offline.py: {}", e))?;

    let stdin = child.stdin.take().ok_or("No stdin")?;
    let stdout = child.stdout.take().ok_or("No stdout")?;
    let stderr = child.stderr.take().ok_or("No stderr")?;
    let mut stdout = BufReader::new(stdout);

    // Drain stderr in background to prevent pipe buffer deadlock
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            eprintln!("[Kazamo/paraformer] {}", line.trim_end());
            line.clear();
        }
    });

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

    eprintln!("[Kazamo] Paraformer: process ready (pid={})", child.id().unwrap_or(0));
    eprintln!("[Kazamo] Paraformer: script={}", script.display());
    eprintln!("[Kazamo] Paraformer: model={}", model_path);
    *proc = Some(ParaformerProc { child, stdin, stdout, stderr_task: Some(stderr_task) });
    drop(proc);
    Ok(&*Box::leak(Box::new(Arc::clone(&PARAFORMER_PROC))))
}

/// Try one transcription attempt. Returns Ok(text), Err(retriable), or Err(fatal).
async fn try_transcribe(
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    tmp_wav: &str,
) -> Result<String, String> {
    eprintln!("[Kazamo] Paraformer: writing wav path: {}", tmp_wav);
    stdin.write_all(format!("{}\n", tmp_wav).as_bytes()).await
        .map_err(|e| { eprintln!("[Kazamo] Paraformer: write_all failed: {}", e); format!("Write stdin: {}", e) })?;

    let mut line = String::new();
    let n = stdout.read_line(&mut line).await
        .map_err(|e| format!("Read stdout: {}", e))?;
    if n == 0 { return Err("stdout closed".into()); }

    let line = line.trim();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
            return Err(err.to_string());
        }
        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
            let text = text.trim().to_string();
            if text.is_empty() { return Err("No speech detected".into()); }
            return Ok(text);
        }
    }
    Err(format!("Unexpected output: {}", line))
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

    // Try up to 2 times (restart process on first failure)
    let mut last_err = String::new();
    for attempt in 0..2u8 {
        let result = {
            let proc_ref = get_or_start_proc(model_dir, resource_dir).await?;
            let mut guard = proc_ref.lock().await;
            if let Some(proc) = guard.as_mut() {
                try_transcribe(&mut proc.stdin, &mut proc.stdout, &tmp_wav).await
            } else {
                Err("process not started".into())
            }
            // guard drops here, releasing the lock
        };

        match result {
            Ok(text) => { let _ = tokio::fs::remove_file(&tmp_wav).await; return Ok(text); }
            Err(e) => {
                last_err = e;
                eprintln!("[Kazamo] Paraformer: attempt {} failed: {}", attempt + 1, last_err);
                PARAFORMER_PROC.lock().await.take();
            }
        }
    }

    let _ = tokio::fs::remove_file(&tmp_wav).await;
    Err(last_err)
}
