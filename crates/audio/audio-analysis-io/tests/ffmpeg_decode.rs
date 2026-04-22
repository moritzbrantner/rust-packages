use audio_analysis_io::{open_audio_input, AudioFrameSource, AudioInput, AudioInputOptions};
use audio_analysis_test_support::{sine, write_pcm16_wav};
use video_analysis_core::{AudioBuffer, AudioSampleFormat};
use video_analysis_ffmpeg::{is_ffmpeg_available, is_ffprobe_available};

#[test]
fn decodes_generated_pcm16_wav_when_ffmpeg_is_available() {
    let required = std::env::var("FFMPEG_EXTERNAL_TESTS").ok().as_deref() == Some("1");
    if !required {
        eprintln!("skipping FFmpeg decode test; set FFMPEG_EXTERNAL_TESTS=1");
        return;
    }
    if !(is_ffmpeg_available() && is_ffprobe_available()) {
        panic!("FFMPEG_EXTERNAL_TESTS=1 but ffmpeg/ffprobe is unavailable");
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tone.wav");
    write_pcm16_wav(&path, 8_000, 1, &sine(440.0, 8_000, 0.1)).unwrap();

    let mut source = open_audio_input(
        AudioInput::File(path.clone()),
        AudioInputOptions::recorded().samples_per_chunk(128),
    )
    .unwrap();

    assert_eq!(source.metadata().sample_rate, 8_000);
    assert_eq!(source.metadata().channels, 1);
    assert_eq!(
        source.source_info().audio[0].sample_format,
        AudioSampleFormat::F32
    );

    let mut total_samples = 0;
    let mut saw_signal = false;
    while let Some(frame) = source.next_audio_frame().unwrap() {
        assert_eq!(frame.sample_rate, 8_000);
        assert_eq!(frame.channels, 1);
        let AudioBuffer::F32(samples) = frame.data else {
            panic!("expected f32 decoded audio");
        };
        total_samples += samples.len();
        saw_signal |= samples.iter().any(|sample| sample.abs() > 0.01);
    }

    assert_eq!(total_samples, 800);
    assert!(saw_signal);
}
