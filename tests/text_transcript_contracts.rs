use text_core::{AsTextSegmentContract, TextSegmentContract};
use text_transcripts::{
    parse_plain_lines, parse_srt, parse_webvtt, parse_whisper_json, segment_to_owned_text_segment,
    TranscriptSegmentContract, TranscriptionContract,
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
fn transcript_contract_validation_rejects_invalid_ranges() {
    let mut segment = TranscriptSegmentContract::new(0, "invalid");
    segment.start_seconds = Some(3.0);
    segment.end_seconds = Some(2.0);

    assert!(segment.validate().is_err());
}
