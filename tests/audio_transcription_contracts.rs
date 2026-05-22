use std::collections::BTreeMap;

use audio_analysis_models::{
    transcribe_audio, AudioModelSelection, AudioRuntime, SpeechRecognitionRequest,
};
use text_transcripts::TranscriptSegmentContract;

#[test]
fn audio_asr_returns_transcription_contract_from_imported_segments(
) -> Result<(), Box<dyn std::error::Error>> {
    let response = transcribe_audio(SpeechRecognitionRequest {
        source: Some("fixture.wav".to_string()),
        language: Some("en".to_string()),
        model: AudioModelSelection::default(),
        imported_segments: vec![
            TranscriptSegmentContract {
                index: 0,
                start_seconds: Some(0.0),
                end_seconds: Some(1.25),
                text: "hello".to_string(),
                language: Some("en".to_string()),
                speaker: Some("speaker_0".to_string()),
                confidence: Some(0.95),
                is_final: true,
                words: Vec::new(),
                attributes: BTreeMap::from([("channel".to_string(), "left".to_string())]),
            },
            TranscriptSegmentContract {
                index: 1,
                start_seconds: Some(1.25),
                end_seconds: Some(2.5),
                text: "world".to_string(),
                language: Some("en".to_string()),
                speaker: Some("speaker_0".to_string()),
                confidence: Some(0.9),
                is_final: true,
                words: Vec::new(),
                attributes: BTreeMap::new(),
            },
        ],
    })?;

    assert!(response.accepted);
    assert_eq!(response.operation, "transcribe");
    assert_eq!(response.runtime, AudioRuntime::ImportedPredictions);
    assert_eq!(response.text(), "hello world");
    assert_eq!(response.transcript.source.as_deref(), Some("fixture.wav"));
    assert_eq!(response.transcript.language.as_deref(), Some("en"));
    assert_eq!(response.segments().len(), 2);
    assert_eq!(response.segments()[0].index, 0);
    assert_eq!(response.segments()[0].confidence, Some(0.95));
    assert_eq!(
        response.segments()[0]
            .attributes
            .get("channel")
            .map(String::as_str),
        Some("left")
    );

    Ok(())
}
