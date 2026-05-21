#[cfg(feature = "external-tests")]
mod external {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use text_transcripts::{Transcriber, WhisperCliTranscriber};

    fn require_command(command: &str) {
        find_command(command)
            .unwrap_or_else(|| panic!("required command `{command}` is unavailable"));
    }

    fn find_command(command: &str) -> Option<PathBuf> {
        let path = Path::new(command);
        if path.components().count() > 1 && path.is_file() {
            return Some(path.to_path_buf());
        }
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(command))
                .find(|candidate| candidate.is_file())
        })
    }

    #[test]
    #[ignore = "requires real ffmpeg with flite filter and whisper CLI"]
    fn real_whisper_transcribes_generated_speech_audio() {
        require_command("ffmpeg");
        require_command("whisper");

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("speech.wav");
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "flite=text=hello from rust testing",
                "-ar",
                "16000",
                "-ac",
                "1",
            ])
            .arg(&input)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "ffmpeg failed to synthesize speech fixture"
        );

        let mut transcriber = WhisperCliTranscriber::new("whisper")
            .args([
                "--model".to_string(),
                "tiny".to_string(),
                "--language".to_string(),
                "en".to_string(),
            ])
            .output_dir(dir.path().join("transcript"));
        let transcript = transcriber.transcribe(&input).unwrap();

        assert!(!transcript.segments.is_empty());
        assert!(transcript
            .segments
            .windows(2)
            .all(|pair| pair[0].start_seconds <= pair[1].start_seconds));
        assert!(!transcript
            .text
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty());
    }
}
