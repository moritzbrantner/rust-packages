#[cfg(feature = "external-tests")]
mod external {
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, Scene};
    use video_analysis_split::{split_video_ffmpeg, SplitOptions};

    #[test]
    #[ignore = "requires real ffmpeg and ffprobe"]
    fn real_ffmpeg_splits_generated_two_scene_video() {
        video_analysis_test_support::require_command("ffmpeg");
        video_analysis_test_support::require_command("ffprobe");

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("two-scenes.mp4");
        video_analysis_ffmpeg::write_two_scene_test_video(&input).unwrap();

        let rate = Rational64::new(10, 1);
        let scenes = vec![
            Scene {
                start: FramePosition::from_frame_index(0, rate),
                end: FramePosition::from_frame_index(4, rate),
            },
            Scene {
                start: FramePosition::from_frame_index(4, rate),
                end: FramePosition::from_frame_index(8, rate),
            },
        ];
        let options = SplitOptions {
            output_dir: dir.path().join("splits"),
            ..SplitOptions::default()
        };

        let outputs = split_video_ffmpeg(&input, &scenes, &options).unwrap();
        assert_eq!(outputs.len(), 2);
        for output in outputs {
            video_analysis_test_support::assert_nonempty_file(output);
        }
    }
}
