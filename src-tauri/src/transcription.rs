use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub success: bool,
    pub text: String,
    pub error: Option<String>,
}

pub async fn transcribe_sensevoice(
    audio_data: &[u8],
    model_path: &str,
    binary_path: &str,
    language: &str,
    resource_dir: &Path,
) -> TranscriptionResult {
    use tokio::process::Command;

    let tmp_in = format!("/tmp/kazamo-in-{}.wav", std::process::id());
    let tmp_out = format!("/tmp/kazamo-16k-{}.wav", std::process::id());

    if let Err(e) = tokio::fs::write(&tmp_in, audio_data).await {
        return err(&format!("Write failed: {}", e));
    }

    // Convert to 16kHz mono WAV with volume boost (for low-gain microphones)
    let status = Command::new("ffmpeg")
        .args(["-y", "-i", &tmp_in, "-ar", "16000", "-ac", "1", "-af", "volume=20dB", "-f", "wav", &tmp_out])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;

    let _ = tokio::fs::remove_file(&tmp_in).await;

    match status {
        Ok(s) if !s.success() => {
            let _ = tokio::fs::remove_file(&tmp_out).await;
            return err("ffmpeg conversion failed");
        }
        Err(e) => return err(&format!("ffmpeg not found: {}", e)),
        _ => {}
    }

    // Build LD_LIBRARY_PATH: resources/bin + binary dir + fallback directories + existing
    let bin_dir = Path::new(binary_path).parent().unwrap_or(Path::new("."));
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

    let lang = match language { "zh"|"en"|"ja"|"ko"|"yue" => language, _ => "auto" };

    let output = Command::new(binary_path)
        .args(["-m", model_path, "-l", lang, "-itn", &tmp_out])
        .env("LD_LIBRARY_PATH", &ld_path)
        .current_dir("/tmp")
        .output()
        .await;

    let _ = tokio::fs::remove_file(&tmp_out).await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let merged = format!("{}\n{}", stdout, stderr);

            eprintln!("[Kazamo] SenseVoice stderr: {}", stderr.chars().take(500).collect::<String>());

            let text = extract_text(&merged);
            if text.is_empty() {
                let debug: String = stderr.lines().take(3).collect::<Vec<_>>().join(" | ");
                TranscriptionResult {
                    success: false, text: String::new(),
                    error: Some(format!("No speech detected. [{}]", debug.chars().take(200).collect::<String>())),
                }
            } else {
                TranscriptionResult { success: true, text, error: None }
            }
        }
        Err(e) => err(&format!("Process error: {}", e)),
    }
}

fn err(msg: &str) -> TranscriptionResult {
    TranscriptionResult { success: false, text: String::new(), error: Some(msg.to_string()) }
}

fn extract_text(output: &str) -> String {
    let mut texts = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if let Some(end) = line.find(']') {
            if line.starts_with('[') && line[end..].contains(|c: char| c.is_alphanumeric()) {
                let t = line[end + 1..].trim();
                if !t.is_empty() {
                    let cleaned: String = t.chars().fold((false, String::new()), |(in_tag, mut acc), c| {
                        if c == '<' { (true, acc) }
                        else if c == '>' { (false, acc) }
                        else if c == '|' { (in_tag, acc) }
                        else if !in_tag { acc.push(c); (false, acc) }
                        else { (true, acc) }
                    }).1;
                    let cleaned = cleaned.trim().to_string();
                    if !cleaned.is_empty() { texts.push(cleaned); }
                }
            }
        }
    }
    texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}
