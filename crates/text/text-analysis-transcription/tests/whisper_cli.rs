#[cfg(feature = "external-tests")]
mod external {
    use std::process::Command;

    use text_analysis_transcription::{Transcriber, WhisperCliTranscriber};

    #[test]
    #[ignore = "requires real ffmpeg with flite filter and whisper CLI"]
    fn real_whisper_transcribes_generated_speech_audio() {
        video_analysis_test_support::require_command("ffmpeg");
        video_analysis_test_support::require_command("whisper");

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
