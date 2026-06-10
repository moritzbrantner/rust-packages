use audio_analysis_transcription::{
    import_whisperx_json, transcribe, AlignedWord, AlignmentOptions, AlignmentRequest,
    AlignmentResponse, AsrRequest, AsrResponse, AudioTranscriptionProvider, CandleWhisperOptions,
    DiarizationOptions, ForcedAlignmentProvider, LoadedAudio, NativeDevicePreference,
    SpeakerDiarizationResponse, SpeakerSegmentPrediction, SpeechActivitySegment,
    TranscriptDiarizationProvider, TranscriptionPipelineRequest, TranscriptionProviderSelection,
    TranscriptionSource, TranscriptionVadProvider, VadOptions, VadRequest, VadResponse,
};
use text_transcripts::{TranscriptSegmentContract, TranscriptWordContract, TranscriptionContract};
use video_analysis_core::{DetectError, Result};

const WHISPERX_PARITY_FIXTURE: &[u8] = include_bytes!("fixtures/whisperx-parity-sample.json");

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
            diagnostics: vec!["fixed alignment completed".to_string()],
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

fn assert_close(left: Option<f64>, right: Option<f64>, tolerance_seconds: f64, label: &str) {
    match (left, right) {
        (Some(left), Some(right)) => assert!(
            (left - right).abs() <= tolerance_seconds,
            "{label} differs: left={left}, right={right}, tolerance={tolerance_seconds}"
        ),
        _ => assert_eq!(left, right, "{label} presence differs"),
    }
}

fn assert_confidence_close(left: Option<f32>, right: Option<f32>, tolerance: f32, label: &str) {
    match (left, right) {
        (Some(left), Some(right)) => assert!(
            (left - right).abs() <= tolerance,
            "{label} differs: left={left}, right={right}, tolerance={tolerance}"
        ),
        _ => assert_eq!(left, right, "{label} presence differs"),
    }
}

fn assert_transcripts_close(
    left: &TranscriptionContract,
    right: &TranscriptionContract,
    timing_tolerance_seconds: f64,
) {
    assert_eq!(left.text, right.text);
    assert_eq!(left.language, right.language);
    assert_eq!(left.source, right.source);
    assert_eq!(left.segments.len(), right.segments.len());
    for (segment_index, (left_segment, right_segment)) in
        left.segments.iter().zip(&right.segments).enumerate()
    {
        assert_close(
            left_segment.start_seconds,
            right_segment.start_seconds,
            timing_tolerance_seconds,
            &format!("segment {segment_index} start"),
        );
        assert_close(
            left_segment.end_seconds,
            right_segment.end_seconds,
            timing_tolerance_seconds,
            &format!("segment {segment_index} end"),
        );
        assert_eq!(left_segment.text, right_segment.text);
        assert_eq!(left_segment.speaker, right_segment.speaker);
        assert_eq!(left_segment.words.len(), right_segment.words.len());
        for (word_index, (left_word, right_word)) in left_segment
            .words
            .iter()
            .zip(&right_segment.words)
            .enumerate()
        {
            let label = format!("segment {segment_index} word {word_index}");
            assert_eq!(left_word.text, right_word.text, "{label} text");
            assert_close(
                left_word.start_seconds,
                right_word.start_seconds,
                timing_tolerance_seconds,
                &format!("{label} start"),
            );
            assert_close(
                left_word.end_seconds,
                right_word.end_seconds,
                timing_tolerance_seconds,
                &format!("{label} end"),
            );
            assert_eq!(left_word.speaker, right_word.speaker, "{label} speaker");
            assert_confidence_close(
                left_word.confidence,
                right_word.confidence,
                timing_tolerance_seconds as f32,
                &format!("{label} confidence"),
            );
        }
    }
}

fn fixed_native_shape_transcript() -> std::result::Result<TranscriptionContract, String> {
    let mut first = TranscriptSegmentContract::new(0, "hello world");
    first.start_seconds = Some(0.0);
    first.end_seconds = Some(1.2);
    first.speaker = Some("SPEAKER_00".to_string());
    first.words = vec![
        TranscriptWordContract {
            text: "hello".to_string(),
            start_seconds: Some(0.0),
            end_seconds: Some(0.45),
            confidence: Some(0.95),
            speaker: Some("SPEAKER_00".to_string()),
            attributes: Default::default(),
        },
        TranscriptWordContract {
            text: "world".to_string(),
            start_seconds: Some(0.55),
            end_seconds: Some(1.1),
            confidence: Some(0.91),
            speaker: Some("SPEAKER_00".to_string()),
            attributes: Default::default(),
        },
    ];
    let mut second = TranscriptSegmentContract::new(1, "second speaker");
    second.start_seconds = Some(1.35);
    second.end_seconds = Some(2.4);
    second.speaker = Some("SPEAKER_01".to_string());
    second.words = vec![
        TranscriptWordContract {
            text: "second".to_string(),
            start_seconds: Some(1.35),
            end_seconds: Some(1.8),
            confidence: Some(0.88),
            speaker: Some("SPEAKER_01".to_string()),
            attributes: Default::default(),
        },
        TranscriptWordContract {
            text: "speaker".to_string(),
            start_seconds: Some(1.9),
            end_seconds: Some(2.35),
            confidence: Some(0.86),
            speaker: Some("SPEAKER_01".to_string()),
            attributes: Default::default(),
        },
    ];
    TranscriptionContract::from_segments(
        Some("inline".to_string()),
        Some("en".to_string()),
        vec![first, second],
    )
    .map_err(|error| error.to_string())
}

#[test]
fn whisperx_fixture_imports_through_audio_transcription_contract(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let contract = import_whisperx_json(WHISPERX_PARITY_FIXTURE)?;

    contract.validate_strict()?;
    assert_eq!(contract.text.as_deref(), Some("hello world second speaker"));
    assert_eq!(contract.language.as_deref(), Some("en"));
    assert_eq!(
        contract.source.as_deref(),
        Some("tests/fixtures/native-whisperx-parity.wav")
    );
    assert_eq!(contract.segments.len(), 2);
    assert_eq!(contract.segments[0].speaker.as_deref(), Some("SPEAKER_00"));
    assert_eq!(contract.segments[1].speaker.as_deref(), Some("SPEAKER_01"));
    assert_eq!(contract.segments[0].words.len(), 2);
    assert_eq!(contract.segments[1].words.len(), 2);
    assert_eq!(contract.segments[0].words[0].confidence, Some(0.95));
    assert_eq!(contract.segments[1].words[1].confidence, Some(0.86));
    assert_eq!(
        contract.segments[0]
            .attributes
            .get("custom_segment")
            .map(String::as_str),
        Some("kept-left")
    );
    assert_eq!(
        contract.segments[0].words[0]
            .attributes
            .get("custom_word")
            .map(String::as_str),
        Some("kept-word")
    );

    Ok(())
}

#[test]
fn mock_whisperx_command_output_matches_imported_fixture(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let command = temp.path().join("mock-whisperx.sh");
    let output_dir = temp.path().join("out");
    std::fs::write(
        &command,
        format!(
            "#!/usr/bin/env bash\nmkdir -p \"{}\"\ncat > \"{}/sample.json\" <<'JSON'\n{}\nJSON\n",
            output_dir.display(),
            output_dir.display(),
            std::str::from_utf8(WHISPERX_PARITY_FIXTURE)?
        ),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&command)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&command, permissions)?;
    }

    let response = transcribe(TranscriptionPipelineRequest {
        source: TranscriptionSource::Path {
            path: temp.path().join("speech.wav"),
        },
        provider: TranscriptionProviderSelection::ExternalWhisperX(
            audio_analysis_transcription::WhisperXCommandOptions {
                command,
                output_dir: Some(output_dir),
                ..audio_analysis_transcription::WhisperXCommandOptions::default()
            },
        ),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    })?;
    let imported = import_whisperx_json(WHISPERX_PARITY_FIXTURE)?;

    assert_transcripts_close(&response.transcript, &imported, 0.001);
    response.transcript.validate_strict()?;

    Ok(())
}

#[test]
fn native_pipeline_output_preserves_whisperx_contract_shape(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    struct ShapeVad;
    impl TranscriptionVadProvider for ShapeVad {
        fn provider_id(&self) -> &str {
            "shape-vad"
        }

        fn detect_speech(&mut self, _request: VadRequest) -> Result<VadResponse> {
            Ok(VadResponse {
                segments: vec![SpeechActivitySegment::new(0.0, 2.5, 1.0)?],
                diagnostics: Vec::new(),
            })
        }
    }

    struct ShapeAsr;
    impl AudioTranscriptionProvider for ShapeAsr {
        fn provider_id(&self) -> &str {
            "shape-asr"
        }

        fn transcribe(&mut self, _request: AsrRequest) -> Result<AsrResponse> {
            Ok(AsrResponse {
                model_id: "shape-whisper".to_string(),
                language: Some("en".to_string()),
                transcript: fixed_native_shape_transcript()
                    .map_err(DetectError::InvalidArgument)?,
                diagnostics: Vec::new(),
            })
        }
    }

    let mut vad = ShapeVad;
    let mut asr = ShapeAsr;
    let request = TranscriptionPipelineRequest {
        source: TranscriptionSource::Samples {
            samples: vec![0.1; 40_000],
            sample_rate: 16_000,
            channels: 1,
            source: Some("inline".to_string()),
        },
        provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions {
            device: NativeDevicePreference::Cpu,
            ..CandleWhisperOptions::default()
        }),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    };

    let response = audio_analysis_transcription::run_transcription_pipeline(
        request, &mut vad, &mut asr, None, None,
    )?;

    assert_eq!(
        response.transcript.text.as_deref(),
        Some("hello world second speaker")
    );
    assert_eq!(response.transcript.segments.len(), 2);
    assert_eq!(
        response.transcript.segments[0].speaker.as_deref(),
        Some("SPEAKER_00")
    );
    assert_eq!(
        response.transcript.segments[1].speaker.as_deref(),
        Some("SPEAKER_01")
    );
    assert_eq!(response.transcript.segments[0].words.len(), 2);
    assert_eq!(response.transcript.segments[1].words.len(), 2);
    assert!(response
        .transcript
        .segments
        .iter()
        .all(|segment| segment.start_seconds.is_some()
            && segment.end_seconds.is_some()
            && !segment.words.is_empty()
            && segment.words.iter().all(|word| word.start_seconds.is_some()
                && word.end_seconds.is_some()
                && word.speaker.is_some())));
    response.transcript.validate_strict()?;

    Ok(())
}

#[test]
#[ignore]
fn external_whisperx_parity_when_requested() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    if std::env::var("RUN_WHISPERX_PARITY_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping WhisperX parity test; set RUN_WHISPERX_PARITY_TESTS=1");
        return Ok(());
    }
    let audio_path = std::env::var_os("WHISPERX_AUDIO_PATH")
        .map(std::path::PathBuf::from)
        .expect("WHISPERX_AUDIO_PATH is required when RUN_WHISPERX_PARITY_TESTS=1");
    if std::env::var("WHISPERX_DIARIZE").as_deref() == Ok("1")
        && std::env::var_os("HF_TOKEN").is_none()
    {
        panic!("HF_TOKEN is required when WHISPERX_DIARIZE=1");
    }
    let output_temp = tempfile::tempdir()?;
    let output_dir = std::env::var_os("WHISPERX_OUTPUT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| output_temp.path().to_path_buf());
    let command = std::env::var_os("WHISPERX_COMMAND")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("whisperx"));
    let model = std::env::var("WHISPERX_MODEL").unwrap_or_else(|_| "large-v2".to_string());
    let language = std::env::var("WHISPERX_LANGUAGE").ok();
    let device = match std::env::var("WHISPERX_DEVICE")
        .unwrap_or_else(|_| "cpu".to_string())
        .as_str()
    {
        "cpu" => audio_analysis_transcription::WhisperXDevice::Cpu,
        "cuda" => audio_analysis_transcription::WhisperXDevice::Cuda,
        other => panic!("unsupported WHISPERX_DEVICE `{other}`"),
    };
    let compute_type = std::env::var("WHISPERX_COMPUTE_TYPE").ok();
    let diarize = std::env::var("WHISPERX_DIARIZE").as_deref() == Ok("1");

    let response = transcribe(TranscriptionPipelineRequest {
        source: TranscriptionSource::Path { path: audio_path },
        provider: TranscriptionProviderSelection::ExternalWhisperX(
            audio_analysis_transcription::WhisperXCommandOptions {
                command,
                model,
                language,
                device,
                compute_type,
                output_dir: Some(output_dir),
                diarize,
                hf_token_env: diarize.then(|| "HF_TOKEN".to_string()),
                ..audio_analysis_transcription::WhisperXCommandOptions::default()
            },
        ),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    })?;
    response.transcript.validate_strict()?;

    if let Some(expected_json) = std::env::var_os("WHISPERX_EXPECTED_JSON") {
        let expected = import_whisperx_json(&std::fs::read(expected_json)?)?;
        assert_transcripts_close(&response.transcript, &expected, 0.1);
    }

    Ok(())
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

#[test]
fn native_pipeline_reports_alignment_then_diarization_diagnostics() -> Result<()> {
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

    assert_eq!(
        response.alignment.as_ref().unwrap().provider,
        "fixed-aligner"
    );
    assert!(response.diarization.is_some());
    assert_eq!(
        response.transcript.segments[0].words[0].speaker.as_deref(),
        Some("SPEAKER_00")
    );
    let alignment_index = response
        .diagnostics
        .iter()
        .position(|item| item == "fixed alignment completed")
        .expect("alignment diagnostics should be present");
    let diarization_index = response
        .diagnostics
        .iter()
        .position(|item| item == "diarizationProvider=fixed-diarizer")
        .expect("diarization diagnostics should be present");
    assert!(alignment_index < diarization_index);
    response
        .transcript
        .validate_strict()
        .map_err(|error| DetectError::InvalidArgument(error.to_string()))?;

    Ok(())
}
