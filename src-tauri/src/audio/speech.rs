pub fn play_system_speech(text: &str) {
    let text_clean = text.trim().to_string();
    if text_clean.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("say")
                .args(["-v", "Zarvox", "-r", "140", &text_clean])
                .status();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = std::process::Command::new("espeak")
                .args(["-v", "en-us", "-p", "40", "-s", "130", &text_clean])
                .status();
        }
    });
}

pub fn play_pcm_audio(pcm_bytes: &[u8], sample_rate: u32) {
    let bytes_owned = pcm_bytes.to_vec();
    std::thread::spawn(move || {
        #[cfg(not(target_os = "macos"))]
        {
            use std::io::Write;
            if let Ok(mut child) = std::process::Command::new("aplay")
                .args([
                    "-r",
                    &sample_rate.to_string(),
                    "-f",
                    "S16_LE",
                    "-c",
                    "1",
                    "-q",
                ])
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(&bytes_owned);
                }
                let _ = child.wait();
            }
        }
    });
}
