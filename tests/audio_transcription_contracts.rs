#![allow(deprecated)]

use std::collections::BTreeMap;

use audio_analysis_recognition::{
    transcribe, transcribe_audio, AudioRuntime, AudioRuntimeSelection, SpeechRecognitionRequest,
    TranscriptionInput, TranscriptionRequest, TranscriptionRuntimeSelection,
};
use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};

#[test]
fn generic_audio_transcription_returns_transcription_contract_from_imported_segments(
) -> Result<(), Box<dyn std::error::Error>> {
    let response = transcribe(TranscriptionRequest {
        source: Some("fixture.wav".to_string()),
        language: Some("en".to_string()),
        input: TranscriptionInput::ImportedSegments {
            segments: vec![TranscriptSegmentContract {
                index: 0,
                start_seconds: Some(0.0),
                end_seconds: Some(1.25),
                text: " hello ".to_string(),
                language: None,
                speaker: Some("speaker_0".to_string()),
                confidence: Some(2.0),
                is_final: true,
                words: Vec::new(),
                attributes: BTreeMap::from([("channel".to_string(), "left".to_string())]),
            }],
        },
        runtime: TranscriptionRuntimeSelection::default(),
    })?;

    assert!(response.accepted);
    assert_eq!(response.operation, "transcribe");
    assert_eq!(
        response.backend,
        model_runtime::ModelRuntimeBackend::Imported
    );
    assert_eq!(response.transcript.text.as_deref(), Some("hello"));
    assert_eq!(response.transcript.source.as_deref(), Some("fixture.wav"));
    assert_eq!(response.transcript.language.as_deref(), Some("en"));
    assert_eq!(response.transcript.segments.len(), 1);
    assert_eq!(response.transcript.segments[0].text, "hello");
    assert_eq!(
        response.transcript.segments[0].language.as_deref(),
        Some("en")
    );
    assert_eq!(response.transcript.segments[0].confidence, Some(1.0));

    Ok(())
}

#[test]
fn audio_asr_returns_transcription_contract_from_imported_segments(
) -> Result<(), Box<dyn std::error::Error>> {
    let response = transcribe_audio(SpeechRecognitionRequest {
        source: Some("fixture.wav".to_string()),
        language: Some("en".to_string()),
        model: AudioRuntimeSelection::default(),
        imported_segments: vec![
            TranscriptSegmentContract {
                index: 0,
                start_seconds: Some(0.0),
                end_seconds: Some(1.25),
                text: " hello ".to_string(),
                language: None,
                speaker: Some("speaker_0".to_string()),
                confidence: Some(2.0),
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
    assert_eq!(response.runtime, AudioRuntime::Imported);
    assert_eq!(response.text(), "hello world");
    assert_eq!(response.transcript.source.as_deref(), Some("fixture.wav"));
    assert_eq!(response.transcript.language.as_deref(), Some("en"));
    assert_eq!(response.segments().len(), 2);
    assert_eq!(response.segments()[0].index, 0);
    assert_eq!(response.segments()[0].text, "hello");
    assert_eq!(response.segments()[0].language.as_deref(), Some("en"));
    assert_eq!(response.segments()[0].confidence, Some(1.0));
    assert_eq!(
        response.segments()[0]
            .attributes
            .get("channel")
            .map(String::as_str),
        Some("left")
    );

    Ok(())
}

#[test]
fn speech_response_text_falls_back_to_joined_segments() -> Result<(), Box<dyn std::error::Error>> {
    let transcript = TranscriptionContract::new(vec![
        TranscriptSegmentContract::new(0, "hello"),
        TranscriptSegmentContract::new(1, "world"),
    ]);
    let response = audio_analysis_recognition::speech_recognition_response_from_transcription(
        &AudioRuntimeSelection::default(),
        transcript,
    )?;

    assert_eq!(response.text(), "hello world");
    assert_eq!(response.transcript.text.as_deref(), Some("hello world"));

    Ok(())
}

#[test]
fn audio_asr_rejects_invalid_imported_segment_ranges() {
    let mut segment = TranscriptSegmentContract::new(0, "invalid");
    segment.start_seconds = Some(2.0);
    segment.end_seconds = Some(1.0);

    let result = transcribe_audio(SpeechRecognitionRequest {
        source: None,
        language: None,
        model: AudioRuntimeSelection::default(),
        imported_segments: vec![segment],
    });

    assert!(result.is_err());
}
