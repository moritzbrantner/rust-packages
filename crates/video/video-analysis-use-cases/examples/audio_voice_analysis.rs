use std::path::PathBuf;

use video_analysis_use_cases::audio_voice_analysis::{
    run_audio_voice_analysis, AudioVoiceAnalysisRequest,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_audio_voice_analysis(AudioVoiceAnalysisRequest {
        input: PathBuf::from("input.wav"),
        work_dir: PathBuf::from("use-case-output/audio-voice-analysis"),
        ..AudioVoiceAnalysisRequest::default()
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
