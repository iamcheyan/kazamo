use std::process::Stdio;
use tokio::process::Command;

/// Transcribe audio using sherpa-onnx-ws (Paraformer)
/// This starts the WS server, sends audio, and returns the result
pub async fn transcribe_paraformer(
    audio_data: &[u8],
    model_dir: &str,
    binary_path: &str,
    resource_dir: &std::path::Path,
) -> Result<String, String> {
    // Convert to 16kHz mono WAV (with volume boost, matching sensevoice behavior)
    let tmp_in = format!("/tmp/kazamo-pf-in-{}.wav", std::process::id());
    let tmp_wav = format!("/tmp/kazamo-pf-16k-{}.wav", std::process::id());

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

    let audio_to_send = match tokio::fs::read(&tmp_wav).await {
        Ok(b) => b,
        Err(e) => { let _ = tokio::fs::remove_file(&tmp_wav).await; return Err(format!("Read converted wav: {}", e)); }
    };

    // Set LD_LIBRARY_PATH: resources/bin + binary dir + fallback directories + existing
    let bin_dir = std::path::Path::new(binary_path).parent().unwrap_or(std::path::Path::new("."));
    let res_bin = resource_dir.join("bin");
    
    let mut ld_paths = vec![res_bin.to_string_lossy().to_string(), bin_dir.to_string_lossy().to_string()];
    
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // target/debug/resources/bin
            ld_paths.push(dir.join("resources").join("bin").to_string_lossy().to_string());
            // src-tauri/resources/bin
            if let Some(base) = dir.parent().and_then(|p| p.parent()) {
                ld_paths.push(base.join("resources").join("bin").to_string_lossy().to_string());
            }
        }
    }
    
    let existing = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
    if !existing.is_empty() {
        ld_paths.push(existing);
    }
    let ld_path = ld_paths.join(":");

    // Find free port
    let port = find_free_port().await;

    let model_path = format!("{}/model.onnx", model_dir);
    let tokens_path = format!("{}/tokens.txt", model_dir);

    // Start sherpa-onnx-ws server
    let mut server = Command::new(binary_path)
        .args([
            &format!("--paraformer={}", model_path),
            &format!("--tokens={}", tokens_path),
            &format!("--port={}", port),
            "--num-threads=4",
            "--model-type=paraformer",
        ])
        .env("LD_LIBRARY_PATH", &ld_path)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start sherpa-onnx-ws: {}", e))?;

    // Wait for server to be ready
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
    let mut ready = false;
    while tokio::time::Instant::now() < deadline {
        if let Ok(mut client) = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port)).await {
            use tokio::io::AsyncWriteExt;
            let _ = client.write_all(b"").await;
            ready = true;
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    if !ready {
        let _ = server.kill().await;
        return Err("sherpa-onnx-ws failed to start".into());
    }

    // Send audio via WebSocket (use the converted/boosted 16k WAV)
    let ws_url = format!("ws://127.0.0.1:{}", port);
    let result = send_audio_ws(&ws_url, &audio_to_send, 16000).await;

    // Cleanup
    let _ = server.kill().await;
    let _ = tokio::fs::remove_file(&tmp_wav).await;

    result
}

async fn send_audio_ws(url: &str, audio_data: &[u8], sample_rate: u32) -> Result<String, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;

    let (mut ws, _) = connect_async(url).await.map_err(|e| format!("WS connect: {}", e))?;

    // Build message: sample_rate (i32 LE) + data_length (i32 LE) + audio_data
    let msg = {
        let mut buf = Vec::with_capacity(8 + audio_data.len());
        buf.extend_from_slice(&(sample_rate as i32).to_le_bytes());
        buf.extend_from_slice(&(audio_data.len() as i32).to_le_bytes());
        buf.extend_from_slice(audio_data);
        buf
    };

    ws.send(tokio_tungstenite::tungstenite::Message::Binary(msg))
        .await.map_err(|e| format!("WS send: {}", e))?;

    let mut result = String::new();
    while let Some(msg) = ws.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                result.push_str(&text);
                // Send Done to signal end
                let _ = ws.send(tokio_tungstenite::tungstenite::Message::Text("Done".into())).await;
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    // Parse JSON result
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result) {
        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
            return Ok(text.trim().to_string());
        }
    }

    if result.trim().is_empty() {
        Err("No speech detected".into())
    } else {
        Ok(result.trim().to_string())
    }
}

async fn find_free_port() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port.to_string()
}
