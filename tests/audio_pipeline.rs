use audio_analysis_core::{interleaved_to_mono, ChannelMix, FrameSpec, WindowFunction};
use audio_analysis_fourier::{spectrogram, FourierTransform, SpectralAnalyzer, StftConfig};
use audio_analysis_pitch::AutocorrelationPitchDetector;
use audio_analysis_processing::{AudioProcessor, BiquadKind, BiquadSpec, NoiseGateSpec};
use audio_analysis_recognition::{
    AudioRecognitionAnalyzer, AudioReferenceLibrary, SpectralAudioEmbedder, SpectralEmbeddingConfig,
};
use audio_analysis_rhythm::{
    detect_onsets, estimate_tempo, onset_envelope, OnsetDetectorConfig, TempoEstimatorConfig,
};
use audio_analysis_test_support::{
    assert_approx_eq, click_track, interleaved_stereo, owned_f32_frame, sine,
};
use video_analysis_core::{AudioAnalyzer, AudioBuffer, AudioPipeline, Timebase, Timestamp};

#[test]
fn audio_packages_work_together_on_synthetic_signal() {
    let sample_rate = 48_000;
    let left = sine(440.0, sample_rate, 0.2);
    let right = sine(440.0, sample_rate, 0.2);
    let stereo = interleaved_stereo(&left, &right);
    let mono =
        interleaved_to_mono(&AudioBuffer::F32(stereo.clone()), 2, ChannelMix::Average).unwrap();
    assert_eq!(mono.len(), left.len());

    let input = owned_f32_frame(
        Timestamp::new(0, Timebase::new(1, sample_rate as i32)),
        sample_rate,
        2,
        stereo,
    )
    .unwrap();
    let mut processor = AudioProcessor::new()
        .gain(0.8)
        .mono(ChannelMix::Average)
        .hard_clip(-0.9, 0.9)
        .biquad(BiquadSpec {
            kind: BiquadKind::LowPass,
            cutoff_hz: 2_000.0,
            q: 0.707,
        })
        .noise_gate(NoiseGateSpec {
            threshold: 0.001,
            attenuation: 0.0,
        });
    let processed = processor.process_frame(input).unwrap().unwrap();
    assert_eq!(processed.channels, 1);
    assert_eq!(processed.sample_rate, sample_rate);

    let processed_samples = match &processed.data {
        AudioBuffer::F32(samples) => samples,
        _ => panic!("expected f32 processing output"),
    };
    assert!(processed_samples.iter().all(|sample| sample.is_finite()));
    assert!(processed_samples
        .iter()
        .all(|sample| (-0.9..=0.9).contains(sample)));

    let transform = FourierTransform::with_window(4096, WindowFunction::Rectangular).unwrap();
    let spectrum = transform
        .analyze_samples(&processed_samples[..4096], sample_rate)
        .unwrap();
    let dominant = spectrum.dominant_frequency_hz().unwrap();
    assert!((dominant - 440.0).abs() <= sample_rate as f32 / 4096.0);

    let pitch = AutocorrelationPitchDetector::default()
        .estimate_samples(&processed_samples[..4096], sample_rate)
        .unwrap();
    assert_approx_eq(pitch.frequency_hz.unwrap(), 440.0, 440.0 * 0.02);

    let click_track = click_track(2_000, 120.0, 4.0);
    let envelope = onset_envelope(&click_track, 2_000, FrameSpec::new(80, 20).unwrap()).unwrap();
    let onsets = detect_onsets(
        &envelope,
        OnsetDetectorConfig {
            strength_threshold: 0.05,
            min_interval_seconds: 0.1,
        },
    )
    .unwrap();
    let tempo = estimate_tempo(&onsets, TempoEstimatorConfig::default()).unwrap();
    assert_approx_eq(tempo.bpm.unwrap(), 120.0, 2.0);

    let embedder =
        SpectralAudioEmbedder::new(SpectralEmbeddingConfig::new(512, 256, 8).unwrap()).unwrap();
    let mut library = AudioReferenceLibrary::new();
    library
        .add_reference_samples("a4", "A4", &sine(440.0, 8_000, 0.5), 8_000, &embedder)
        .unwrap();
    let mut recognition = AudioRecognitionAnalyzer::new(
        "audio_identity",
        embedder.clone(),
        library,
        embedder.streaming_config(ChannelMix::Average).unwrap(),
    )
    .unwrap();
    let recognition_frame = owned_f32_frame(
        Timestamp::new(0, Timebase::new(1, 8_000)),
        8_000,
        1,
        sine(440.0, 8_000, 0.5),
    )
    .unwrap();
    let recognition_events = recognition
        .process_frame(&recognition_frame.as_frame().unwrap())
        .unwrap();
    assert!(recognition_events
        .iter()
        .any(|event| event.label.starts_with("audio:recognized:a4:A4")));

    let mut pipeline = AudioPipeline::builder()
        .analyzer(SpectralAnalyzer::new(FourierTransform::new(4096).unwrap()).min_magnitude(0.001))
        .analyzer(AutocorrelationPitchDetector::default())
        .build()
        .unwrap();
    let analysis = pipeline.process_frame(processed.clone()).unwrap();
    assert!(analysis
        .events
        .iter()
        .any(|event| event.label.starts_with("audio:dominant_frequency:")));
    assert!(analysis
        .events
        .iter()
        .any(|event| event.label.starts_with("audio:pitch:")));
    let result = pipeline.finish_analysis().unwrap();
    assert_eq!(result.frames_processed, 1);
    assert!(result.events.iter().all(|event| event.timestamp.is_some()));
}

#[test]
fn stft_integration_reports_monotonic_frame_timestamps() {
    let samples = sine(880.0, 16_000, 0.2);
    let frames = spectrogram(&samples, 16_000, &StftConfig::new(512, 128).unwrap()).unwrap();
    assert!(frames.len() > 4);
    assert!(frames
        .windows(2)
        .all(|pair| pair[0].start_seconds < pair[1].start_seconds));
}
