use audio_analysis_io::{open_audio_input, AudioFrameSource, AudioInput, AudioInputOptions};
use video_analysis_core::{AudioBuffer, AudioSampleFormat};
use video_analysis_ffmpeg::{is_ffmpeg_available, is_ffprobe_available};

fn sine(freq_hz: f32, sample_rate: u32, seconds: f32) -> Vec<f32> {
    let samples = (sample_rate as f32 * seconds) as usize;
    (0..samples)
        .map(|index| {
            let t = index as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq_hz * t).sin()
        })
        .collect()
}

fn write_pcm16_wav(
    path: impl AsRef<std::path::Path>,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    let data_len = samples.len() as u32 * 2;
    let byte_rate = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;

    use std::io::Write;

    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_len).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&16_u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_len.to_le_bytes())?;
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        file.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

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
