mod support;

use std::collections::BTreeMap;

use audio_analysis_core::FrameSpec;
use audio_analysis_pitch::AutocorrelationPitchDetector;
use audio_analysis_processing::AudioProcessor;
use audio_analysis_recognition::{
    AudioEmbeddingExtractor, AudioMatchOptions, AudioReferenceLibrary, SpectralAudioEmbedder,
};
use audio_analysis_rhythm::{
    detect_onsets, estimate_tempo, onset_envelope, OnsetDetectorConfig, TempoEstimatorConfig,
};
use audio_analysis_separation::{
    is_demucs_available, DemucsModel, HtdemucsOptions, HtdemucsSeparator,
    SeparationOutputFormat, Stem, StemLayout,
};
use audio_analysis_synthesis::{synthesize_tone, AudioSynthesisConfig, ToneSpec};
use num_rational::Rational64;
use support::{click_track, owned_f32_frame, sine};
use tempfile::tempdir;
use text_analysis_linguistics::{analyze_text, LinguisticAnalysisOptions};
use text_analysis_models::{pool_embedding_output, softmax, PoolingStrategy};
use text_analysis_synthesis::{
    synthesize_from_terms, terms_from_counts, TermPrompt, TextSynthesisOptions,
};
use video_analysis_core::{AnalysisEvent, Timestamp};

#[test]
fn audio_and_text_packages_support_smoke_workflows() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 8_000;
    let tone = sine(440.0, sample_rate, 0.25);
    let detector = AutocorrelationPitchDetector::default();
    let estimate = detector.estimate_samples(&tone, sample_rate)?;
    assert!(estimate.frequency_hz.unwrap() > 430.0);

    let timestamp = Timestamp::new(0, video_analysis_core::Timebase::new(1, sample_rate as i32));
    let frame = owned_f32_frame(timestamp, sample_rate, 1, tone.clone())?;
    let mut processor = AudioProcessor::new().gain(0.5).hard_clip(-0.25, 0.25);
    let processed = processor.process_frame(frame)?.expect("processed frame");
    assert_eq!(processed.channels, 1);

    let embedder = SpectralAudioEmbedder::default();
    let embedding = embedder.embed_samples(&tone, sample_rate)?;
    let mut library = AudioReferenceLibrary::new();
    library.add_reference_embedding("tone-440", "A4", embedding.clone())?;
    let matches = library.search(&embedding, &AudioMatchOptions::default())?;
    assert_eq!(matches[0].reference_id, "tone-440");

    let envelope = onset_envelope(
        &click_track(sample_rate, 120.0, 2.0),
        sample_rate,
        FrameSpec::new(256, 128)?,
    )?;
    let onsets = detect_onsets(&envelope, OnsetDetectorConfig::default())?;
    let tempo = estimate_tempo(&onsets, TempoEstimatorConfig::default())?;
    assert!(tempo.bpm.unwrap() > 100.0);

    let output_dir = tempdir()?;
    let separator = HtdemucsSeparator::new(
        HtdemucsOptions::new(output_dir.path())
            .model(DemucsModel::Htdemucs)
            .two_stems(Stem::Vocals)
            .output_format(SeparationOutputFormat::Wav),
    )?;
    let args = separator.build_args("input.wav")?;
    assert!(args.iter().any(|arg| arg == "--two-stems"));
    assert_eq!(
        separator.expected_layout(),
        StemLayout::TwoStem {
            primary: Stem::Vocals,
            residual: Stem::NoVocals,
        }
    );
    let _ = is_demucs_available();

    let synthesized = synthesize_tone(
        ToneSpec::sine(220.0, 0.1),
        AudioSynthesisConfig::new(8_000, 1)?,
    )?;
    assert_eq!(synthesized.value.channels, 1);

    let pooled = pool_embedding_output(
        &[1.0, 0.0, 0.0, 1.0],
        &[2, 2],
        &[1, 1],
        PoolingStrategy::Mean,
        true,
    )?;
    assert_eq!(pooled.dimensions(), 2);
    assert_eq!(softmax(&[0.0, 1.0]).len(), 2);

    let synthesized_text = synthesize_from_terms(
        "doc-1",
        &[
            TermPrompt::new("rust", 2.0),
            TermPrompt::new("analysis", 1.0),
        ],
        TextSynthesisOptions::default(),
    )?;
    assert!(synthesized_text.value.text.contains("rust"));

    let counted_terms = terms_from_counts(&BTreeMap::from([
        ("video".to_string(), 2_usize),
        ("pipeline".to_string(), 1_usize),
    ]));
    assert_eq!(counted_terms.len(), 2);

    let event_terms = text_analysis_synthesis::terms_from_events(&[AnalysisEvent::new(
        "fixture",
        "text:keyword:rust",
    )
    .score(0.8)]);
    assert!(event_terms.iter().any(|term| term.term == "rust"));

    let linguistic = analyze_text(
        "Alice presented the tokenizer roadmap in Berlin.",
        &LinguisticAnalysisOptions::default(),
    )?;
    assert_eq!(
        linguistic
            .language
            .primary
            .as_ref()
            .map(|prediction| prediction.language.as_str()),
        Some("en")
    );
    assert!(linguistic
        .entities
        .iter()
        .any(|entity| entity.mention.text.contains("Alice")));

    let _ = Rational64::new(30, 1);
    Ok(())
}
