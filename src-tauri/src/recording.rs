use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

pub struct Recorder {
    process: Mutex<Option<Child>>,
    tmp_path: Mutex<Option<String>>,
    start_time: Mutex<Option<Instant>>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            process: Mutex::new(None),
            tmp_path: Mutex::new(None),
            start_time: Mutex::new(None),
        }
    }

    pub fn start(&self) -> Result<(), String> {
        let mut proc = self.process.lock().unwrap();
        if proc.is_some() {
            return Err("Already recording".into());
        }

        let path = format!("/tmp/kazamo-rec-{}.wav", std::process::id());

        let child = Command::new("parecord")
            .args([
                "--format=s16le",
                "--rate=16000",
                "--channels=1",
                "--file-format=wav",
                "--latency-msec=10",
                &path,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start parecord: {}", e))?;

        *proc = Some(child);
        *self.tmp_path.lock().unwrap() = Some(path);
        *self.start_time.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    pub fn stop(&self) -> Result<Vec<u8>, String> {
        let mut proc = self.process.lock().unwrap();
        let path = self.tmp_path.lock().unwrap().clone();
        let start = self.start_time.lock().unwrap().take();

        if let Some(mut child) = proc.take() {
            // Ensure minimum recording duration (500ms)
            if let Some(t) = start {
                let elapsed = t.elapsed();
                if elapsed.as_millis() < 500 {
                    std::thread::sleep(std::time::Duration::from_millis(500) - elapsed);
                }
            }

            // Send SIGINT for graceful shutdown (parecord flushes WAV header)
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
            // Wait up to 2 seconds for graceful exit
            let deadline = Instant::now() + std::time::Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if Instant::now() > deadline {
                            let _ = child.kill();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => { let _ = child.kill(); break; }
                }
            }
            let _ = child.wait();
            std::thread::sleep(std::time::Duration::from_millis(100));
        } else {
            return Err("Not recording".into());
        }

        if let Some(path) = path {
            let data = std::fs::read(&path).map_err(|e| format!("Read WAV failed: {}", e))?;
            let _ = std::fs::remove_file(&path);

            if data.len() < 100 {
                return Err("Recording too short (no audio captured)".into());
            }

            Ok(data)
        } else {
            Err("No temp file path".into())
        }
    }

    pub fn is_recording(&self) -> bool {
        self.process.lock().unwrap().is_some()
    }
}
