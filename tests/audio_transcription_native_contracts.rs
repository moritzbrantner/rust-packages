use audio_analysis_transcription::{
    AlignedWord, AlignmentOptions, AlignmentRequest, AlignmentResponse, AsrRequest, AsrResponse,
    AudioTranscriptionProvider, CandleWhisperOptions, DiarizationOptions, ForcedAlignmentProvider,
    LoadedAudio, NativeDevicePreference, SpeakerDiarizationResponse, SpeakerSegmentPrediction,
    SpeechActivitySegment, TranscriptDiarizationProvider, TranscriptionPipelineRequest,
    TranscriptionProviderSelection, TranscriptionSource, TranscriptionVadProvider, VadOptions,
    VadRequest, VadResponse,
};
use text_transcripts::{TranscriptSegmentContract, TranscriptionContract};
use video_analysis_core::{DetectError, Result};

struct FixedVad;

impl TranscriptionVadProvider for FixedVad {
    fn provider_id(&self) -> &str {
        "fixed-vad"
    }

    fn detect_speech(&mut self, _request: VadRequest) -> Result<VadResponse> {
        Ok(VadResponse {
            segments: vec![SpeechActivitySegment::new(0.25, 1.25, 0.8)?],
            diagnostics: Vec::new(),
        })
    }
}

struct FixedAsr;

impl AudioTranscriptionProvider for FixedAsr {
    fn provider_id(&self) -> &str {
        "fixed-asr"
    }

    fn transcribe(&mut self, _request: AsrRequest) -> Result<AsrResponse> {
        let mut segment = TranscriptSegmentContract::new(0, " hello world ");
        segment.start_seconds = Some(0.0);
        segment.end_seconds = Some(1.0);
        Ok(AsrResponse {
            model_id: "mock-whisper".to_string(),
            language: Some("en".to_string()),
            transcript: TranscriptionContract::from_segments(
                Some("inline".to_string()),
                Some("en".to_string()),
                vec![segment],
            )
            .map_err(|error| DetectError::InvalidArgument(error.to_string()))?,
            diagnostics: Vec::new(),
        })
    }
}

struct FixedAligner;

impl ForcedAlignmentProvider for FixedAligner {
    fn provider_id(&self) -> &str {
        "fixed-aligner"
    }

    fn align(&mut self, request: AlignmentRequest) -> Result<AlignmentResponse> {
        Ok(AlignmentResponse {
            model_id: request.model_id,
            words: vec![
                AlignedWord {
                    segment_index: 0,
                    word_index: 0,
                    text: "hello".to_string(),
                    start_seconds: 0.30,
                    end_seconds: 0.60,
                    confidence: Some(0.93),
                },
                AlignedWord {
                    segment_index: 0,
                    word_index: 1,
                    text: "world".to_string(),
                    start_seconds: 0.70,
                    end_seconds: 1.00,
                    confidence: Some(0.91),
                },
            ],
            diagnostics: Vec::new(),
        })
    }
}

struct FixedDiarizer;

impl TranscriptDiarizationProvider for FixedDiarizer {
    fn provider_id(&self) -> &str {
        "fixed-diarizer"
    }

    fn diarize(
        &mut self,
        _audio: LoadedAudio,
        _transcript: &TranscriptionContract,
        _options: &DiarizationOptions,
    ) -> Result<SpeakerDiarizationResponse> {
        Ok(SpeakerDiarizationResponse {
            accepted: true,
            operation: "audio.speakers.diarize".to_string(),
            model_id: "mock-speakers".to_string(),
            runtime: "mock".to_string(),
            segments: vec![SpeakerSegmentPrediction {
                speaker: "SPEAKER_00".to_string(),
                start_seconds: 0.0,
                end_seconds: 2.0,
                score: Some(0.88),
            }],
        })
    }
}

#[test]
fn mock_native_asr_alignment_and_diarization_round_trip_into_transcript_contract() -> Result<()> {
    let mut vad = FixedVad;
    let mut asr = FixedAsr;
    let mut aligner = FixedAligner;
    let mut diarizer = FixedDiarizer;
    let request = TranscriptionPipelineRequest {
        source: TranscriptionSource::Samples {
            samples: vec![0.1; 16_000],
            sample_rate: 16_000,
            channels: 1,
            source: Some("inline".to_string()),
        },
        provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions {
            device: NativeDevicePreference::Cpu,
            ..CandleWhisperOptions::default()
        }),
        vad: VadOptions::default(),
        alignment: AlignmentOptions {
            enabled: true,
            ..AlignmentOptions::default()
        },
        diarization: DiarizationOptions {
            enabled: true,
            ..DiarizationOptions::default()
        },
        output: Default::default(),
    };

    let response = audio_analysis_transcription::run_transcription_pipeline(
        request,
        &mut vad,
        &mut asr,
        Some(&mut aligner),
        Some(&mut diarizer),
    )?;

    assert_eq!(response.provider, "candle-whisper");
    assert_eq!(response.vad_segments[0].start_seconds, 0.25);
    assert_eq!(response.transcript.text.as_deref(), Some("hello world"));
    assert_eq!(response.transcript.segments[0].start_seconds, Some(0.25));
    assert_eq!(response.transcript.segments[0].words.len(), 2);
    assert_eq!(
        response.transcript.segments[0].words[0].speaker.as_deref(),
        Some("SPEAKER_00")
    );
    assert_eq!(
        response.transcript.segments[0].speaker.as_deref(),
        Some("SPEAKER_00")
    );

    Ok(())
}
