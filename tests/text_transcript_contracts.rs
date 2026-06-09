use text_core::{AsTextSegmentContract, TextSegmentContract};
use text_transcripts::{
    parse_plain_lines, parse_srt, parse_webvtt, parse_whisper_json, parse_whisperx_json,
    segment_to_owned_text_segment, TranscriptSegmentContract, TranscriptWordContract,
    TranscriptionContract,
};

#[test]
fn transcript_parsers_convert_to_generic_text_contracts_without_losing_core_fields(
) -> Result<(), Box<dyn std::error::Error>> {
    let whisper = parse_whisper_json(include_bytes!("fixtures/whisper-sample.json"))?;
    let whisper_contract = TranscriptionContract::from(whisper);
    whisper_contract.validate()?;
    assert!(!whisper_contract.segments.is_empty());

    let first = &whisper_contract.segments[0];
    let text_segment = first.as_text_segment_contract();
    assert_eq!(text_segment.segment_index, first.index);
    assert_eq!(text_segment.text, first.text);
    assert_eq!(text_segment.language, first.language);
    assert_eq!(
        text_segment
            .timestamp
            .map(|timestamp| (timestamp.seconds() * 1_000.0).round() / 1_000.0),
        first
            .start_seconds
            .map(|seconds| (seconds * 1_000.0).round() / 1_000.0)
    );

    let owned = TextSegmentContract::from(first).to_owned_text_segment();
    assert_eq!(owned.segment_index, first.index);
    assert_eq!(owned.text, first.text);
    assert_eq!(owned.language, first.language);

    let legacy = segment_to_owned_text_segment(&first.clone().into());
    assert_eq!(legacy.segment_index, first.index);
    assert_eq!(legacy.text, first.text);

    let srt = parse_srt("1\n00:00:01,000 --> 00:00:02,500\nHello from SRT.\n")?;
    let srt_segment = TranscriptSegmentContract::from(srt.segments[0].clone());
    srt_segment.validate()?;
    assert_eq!(srt_segment.start_seconds, Some(1.0));
    assert_eq!(srt_segment.end_seconds, Some(2.5));

    let webvtt = parse_webvtt("WEBVTT\n\n00:00:03.000 --> 00:00:04.000\nHello from VTT.\n")?;
    let webvtt_segment = TranscriptSegmentContract::from(webvtt.segments[0].clone());
    assert_eq!(webvtt_segment.start_seconds, Some(3.0));
    assert_eq!(webvtt_segment.duration_seconds(), Some(1.0));

    let plain = parse_plain_lines("first line\n\nsecond line\n");
    let plain_contract = TranscriptionContract::from(plain);
    assert_eq!(plain_contract.segments.len(), 2);
    assert_eq!(plain_contract.joined_text(), "first line second line");

    Ok(())
}

#[test]
fn transcript_timing_survives_rich_text_contract_round_trip() {
    let mut segment = TranscriptSegmentContract::new(4, "camera pans across Berlin");
    segment.start_seconds = Some(12.5);
    segment.end_seconds = Some(15.0);
    segment.language = Some("en".to_string());
    segment
        .attributes
        .insert("speaker".to_string(), "narrator".to_string());

    let mut text_segment = segment.as_text_segment_contract();
    text_segment.stream_id = Some("video-subtitles".to_string());
    let document = text_segment.to_text_document_contract();
    let restored_segment = document.to_text_segment_contract(4);

    assert_eq!(document.id, "video-subtitles:4");
    assert_eq!(document.language.as_deref(), Some("en"));
    assert_eq!(document.timestamp.unwrap().seconds(), 12.5);
    assert_eq!(
        document
            .source
            .as_ref()
            .and_then(|source| source.duration_seconds),
        Some(2.5)
    );
    assert_eq!(document.attributes["speaker"], "narrator");
    assert_eq!(restored_segment.timestamp.unwrap().seconds(), 12.5);
    assert_eq!(restored_segment.duration_seconds, Some(2.5));
    assert_eq!(restored_segment.attributes["speaker"], "narrator");
}

#[test]
fn transcript_contract_validation_rejects_invalid_ranges() {
    let mut segment = TranscriptSegmentContract::new(0, "invalid");
    segment.start_seconds = Some(3.0);
    segment.end_seconds = Some(2.0);

    assert!(segment.validate().is_err());
}

#[test]
fn transcript_contract_normalizes_segments_words_and_confidence(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut segment = TranscriptSegmentContract::new(0, " hello ");
    segment.confidence = Some(3.0);
    segment.words = vec![
        TranscriptWordContract {
            text: " hello ".to_string(),
            start_seconds: Some(0.0),
            end_seconds: Some(0.5),
            confidence: Some(-1.0),
            speaker: None,
            attributes: Default::default(),
        },
        TranscriptWordContract {
            text: "   ".to_string(),
            start_seconds: None,
            end_seconds: None,
            confidence: Some(0.5),
            speaker: None,
            attributes: Default::default(),
        },
    ];

    let normalized = TranscriptionContract::new(vec![segment]).normalized()?;

    assert_eq!(normalized.text.as_deref(), Some("hello"));
    assert_eq!(normalized.segments[0].text, "hello");
    assert_eq!(normalized.segments[0].confidence, Some(1.0));
    assert_eq!(normalized.segments[0].words.len(), 1);
    assert_eq!(normalized.segments[0].words[0].text, "hello");
    assert_eq!(normalized.segments[0].words[0].confidence, Some(0.0));

    Ok(())
}

#[test]
fn transcript_contract_strict_validation_rejects_empty_segments() {
    let contract = TranscriptionContract::new(vec![TranscriptSegmentContract::new(0, "   ")]);

    assert!(contract.validate().is_ok());
    assert!(contract.validate_strict().is_err());
}

#[test]
fn transcript_contract_strict_validation_rejects_non_monotonic_starts() {
    let mut first = TranscriptSegmentContract::new(0, "first");
    first.start_seconds = Some(2.0);
    first.end_seconds = Some(3.0);
    let mut second = TranscriptSegmentContract::new(1, "second");
    second.start_seconds = Some(1.0);
    second.end_seconds = Some(4.0);

    let contract = TranscriptionContract::new(vec![first, second]);

    assert!(contract.validate().is_ok());
    assert!(contract.validate_strict().is_err());
}

#[test]
fn transcript_contract_strict_validation_rejects_word_timings_outside_parent_segment() {
    let mut segment = TranscriptSegmentContract::new(0, "hello world");
    segment.start_seconds = Some(1.0);
    segment.end_seconds = Some(2.0);
    segment.words = vec![TranscriptWordContract {
        text: "world".to_string(),
        start_seconds: Some(1.5),
        end_seconds: Some(2.5),
        confidence: Some(0.8),
        speaker: None,
        attributes: Default::default(),
    }];

    let contract = TranscriptionContract::new(vec![segment]);

    assert!(contract.validate().is_ok());
    assert!(contract.validate_strict().is_err());
}

#[test]
fn whisperx_import_preserves_words_speakers_confidence_and_unknown_fields(
) -> Result<(), Box<dyn std::error::Error>> {
    let contract = parse_whisperx_json(
        br#"{
            "language": "en",
            "source": "fixture.wav",
            "segments": [{
                "start": 0.0,
                "end": 1.2,
                "text": " hello world ",
                "custom_segment": "kept",
                "words": [
                    {"word": "hello", "start": 0.0, "end": 0.4, "score": 0.95, "speaker": "SPEAKER_00", "custom_word": "left"},
                    {"word": "world", "start": 0.5, "end": 1.0, "score": 0.90, "speaker": "SPEAKER_00"}
                ]
            }]
        }"#,
    )?;

    assert_eq!(contract.text.as_deref(), Some("hello world"));
    assert_eq!(contract.language.as_deref(), Some("en"));
    assert_eq!(contract.source.as_deref(), Some("fixture.wav"));
    assert_eq!(contract.segments[0].speaker.as_deref(), Some("SPEAKER_00"));
    assert_eq!(
        contract.segments[0]
            .attributes
            .get("custom_segment")
            .map(String::as_str),
        Some("kept")
    );
    assert_eq!(contract.segments[0].words[0].confidence, Some(0.95));
    assert_eq!(
        contract.segments[0].words[0]
            .attributes
            .get("custom_word")
            .map(String::as_str),
        Some("left")
    );

    Ok(())
}
