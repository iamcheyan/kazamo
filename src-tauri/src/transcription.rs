use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub success: bool,
    pub text: String,
    pub error: Option<String>,
}

/// Find a binary in resource directories
fn find_binary_local(names: &[&str], resource_dir: &Path) -> Option<String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    for &name in names {
        candidates.push(resource_dir.join("bin").join(name));
        candidates.push(resource_dir.join("resources").join("bin").join(name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for &name in names {
                candidates.push(dir.join("resources").join("bin").join(name));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for &name in names {
            candidates.push(cwd.join("resources").join("bin").join(name));
            candidates.push(cwd.join("src-tauri").join("resources").join("bin").join(name));
        }
    }
    candidates.iter().find(|p| p.exists()).map(|p| p.to_string_lossy().to_string())
}

pub async fn transcribe_sensevoice(
    audio_data: &[u8],
    model_path: &str,
    binary_path: &str,
    language: &str,
    resource_dir: &Path,
) -> TranscriptionResult {
    // Convert to 16kHz mono WAV
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

    // Find sense-voice-main binary
    let binary = if !binary_path.is_empty() && Path::new(binary_path).exists() {
        binary_path.to_string()
    } else {
        match find_binary_local(&["sense-voice-main"], resource_dir) {
            Some(b) => b,
            None => {
                let _ = tokio::fs::remove_file(&tmp_out).await;
                return err("sense-voice-main binary not found");
            }
        }
    };
    if let Some(message) = binary_arch_error(&binary).await {
        let _ = tokio::fs::remove_file(&tmp_out).await;
        return err(&message);
    }

    let lang = match language { "zh"|"en"|"ja"|"ko"|"yue" => language, _ => "auto" };

    // Run sense-voice-main
    eprintln!("[Kazamo] SenseVoice: running {} -m {} -l {} {}", binary, model_path, lang, tmp_out);
    let output = Command::new(&binary)
        .args(["-m", model_path, "-l", lang, &tmp_out])
        .output().await;

    let _ = tokio::fs::remove_file(&tmp_out).await;

    match output {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);

            // Log stderr (model loading info)
            for line in stderr.lines() {
                if !line.is_empty() {
                    eprintln!("[Kazamo/sensevoice] {}", line);
                }
            }

            if !out.status.success() {
                return err(&format!("sense-voice-main exited with {:?}", out.status.code()));
            }

            // Parse stdout: format is "[start-end] text" per segment
            let text = stdout.trim();
            if text.is_empty() {
                return err("No speech detected");
            }

            // Extract text from "[timestamp] text" format, join multiple segments
            let result: String = text.lines()
                .filter_map(|line| {
                    let line = line.trim();
                    // Strip timestamp prefix: [0.00-1.23] text
                    if let Some(bracket_end) = line.find(']') {
                        let after = line[bracket_end + 1..].trim();
                        if !after.is_empty() { Some(after) } else { None }
                    } else if !line.is_empty() {
                        Some(line)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            // Strip SenseVoice tags like <|zh|><|NEUTRAL|><|Speech|>
            let cleaned = strip_sensevoice_tags(&result);

            if cleaned.trim().is_empty() {
                return err("No speech detected");
            }

            TranscriptionResult { success: true, text: cleaned.trim().to_string(), error: None }
        }
        Err(e) => err(&format!("Failed to run sense-voice-main: {}", e)),
    }
}

fn err(msg: &str) -> TranscriptionResult {
    TranscriptionResult { success: false, text: String::new(), error: Some(msg.to_string()) }
}

async fn binary_arch_error(path: &str) -> Option<String> {
    let bytes = tokio::fs::read(path).await.ok()?;
    if bytes.len() < 20 || &bytes[0..4] != b"\x7FELF" {
        return None;
    }

    let machine = u16::from_le_bytes([bytes[18], bytes[19]]);
    let binary_arch = match machine {
        62 => "x86_64",
        183 => "aarch64",
        3 => "x86",
        40 => "arm",
        _ => return None,
    };

    let host_arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "arm" => "arm",
        "x86" | "i686" => "x86",
        _ => return None,
    };

    if binary_arch == host_arch {
        None
    } else {
        Some(format!(
            "SenseVoice binary architecture mismatch: bundled binary is {}, but this machine is {}. Install or build a native sense-voice-main binary for {}.",
            binary_arch, host_arch, host_arch
        ))
    }
}

/// Strip SenseVoice tags like <|zh|>, <|NEUTRAL|>, <|Speech|>, <|woitn|>, etc.
fn strip_sensevoice_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            // Check if this starts a <|...|> tag
            let rest: String = chars.clone().take_while(|&ch| ch != '>').collect();
            if rest.starts_with('|') && rest.ends_with('|') {
                // Skip past the closing '>'
                for _ in 0..=rest.len() { chars.next(); }
                continue;
            }
        }
        result.push(c);
    }
    result
}
