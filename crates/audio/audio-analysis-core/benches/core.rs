use audio_analysis_core::{
    interleaved_to_mono, normalized_samples, ChannelMix, StreamingFrameBuffer, StreamingFrameConfig,
};
use audio_analysis_test_support::{interleaved_stereo, sine};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use video_analysis_core::{AudioBuffer, AudioFrame, Timebase, Timestamp};

fn bench_core(c: &mut Criterion) {
    let f32_samples = sine(440.0, 48_000, 10.0);
    let i16_samples = f32_samples
        .iter()
        .map(|sample| (*sample * i16::MAX as f32) as i16)
        .collect::<Vec<_>>();
    let stereo = interleaved_stereo(&f32_samples, &f32_samples);

    c.bench_function("normalize_f32_10s", |b| {
        b.iter(|| normalized_samples(black_box(&AudioBuffer::F32(f32_samples.clone()))))
    });
    c.bench_function("normalize_i16_10s", |b| {
        b.iter(|| normalized_samples(black_box(&AudioBuffer::I16(i16_samples.clone()))))
    });
    c.bench_function("stereo_to_mono_10s", |b| {
        b.iter(|| {
            interleaved_to_mono(
                black_box(&AudioBuffer::F32(stereo.clone())),
                2,
                ChannelMix::Average,
            )
            .unwrap()
        })
    });
    c.bench_function("streaming_frame_buffer_10s", |b| {
        b.iter(|| {
            let config = StreamingFrameConfig::new(2048, 512).unwrap();
            let mut buffer = StreamingFrameBuffer::new(config).unwrap();
            let audio = AudioBuffer::F32(f32_samples.clone());
            let frame = AudioFrame::new(
                Timestamp::new(0, Timebase::new(1, 48_000)),
                48_000,
                1,
                &audio,
            )
            .unwrap();
            buffer.push_frame(black_box(&frame)).unwrap()
        })
    });
}

criterion_group!(benches, bench_core);
criterion_main!(benches);
