use std::process::Stdio;
use tokio::process::Command;

/// Transcribe audio using sherpa-onnx Python API (Paraformer)
pub async fn transcribe_paraformer(
    audio_data: &[u8],
    model_dir: &str,
    _binary_path: &str,
    resource_dir: &std::path::Path,
) -> Result<String, String> {
    // Convert to 16kHz mono WAV (with volume boost)
    let tmp_wav = format!("/tmp/kazamo-pf-16k-{}.wav", std::process::id());

    // Write input to temp file for ffmpeg
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

    // Find the Python script in resource_dir/bin
    let script = resource_dir.join("bin").join("paraformer-offline.py");
    if !script.exists() {
        let _ = tokio::fs::remove_file(&tmp_wav).await;
        return Err("paraformer-offline.py not found".into());
    }

    let model_path = format!("{}/model.onnx", model_dir);
    let tokens_path = format!("{}/tokens.txt", model_dir);

    // Run paraformer via sherpa-onnx Python API
    let output = Command::new("python3")
        .arg(&script)
        .arg(&model_path)
        .arg(&tokens_path)
        .arg(&tmp_wav)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().await;

    let _ = tokio::fs::remove_file(&tmp_wav).await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);

            if !out.status.success() {
                return Err(format!("paraformer-offline failed: {}", stderr.trim()));
            }

            // Parse JSON output {"text": "..."}
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        return Err("No speech detected".into());
                    }
                    return Ok(text);
                }
                if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
                    return Err(err.to_string());
                }
            }

            Err(format!("Unexpected output: {}", stdout.trim()))
        }
        Err(e) => Err(format!("Failed to run python3: {}", e)),
    }
}
