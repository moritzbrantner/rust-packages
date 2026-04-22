#[cfg(feature = "external-tests")]
mod external {
    use video_analysis_radiance_pipeline::{VideoToRadiancePipeline, VideoToRadianceRequest};

    #[test]
    #[ignore = "requires ffmpeg, colmap, and Nerfstudio CLI commands"]
    fn real_external_radiance_pipeline_reaches_process_data_stage() {
        video_analysis_test_support::require_command("ffmpeg");
        video_analysis_test_support::require_command("colmap");
        video_analysis_test_support::require_command("ns-process-data");

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("two-scenes.mp4");
        video_analysis_ffmpeg::write_two_scene_test_video(&input).unwrap();

        let mut request = VideoToRadianceRequest::new(&input, dir.path().join("radiance"));
        request.frame_sample_every = 2;
        request.max_frames = Some(4);
        request.run_training = false;

        let result = VideoToRadiancePipeline::run(request).unwrap();
        assert!(result.completed.contains(&"frame_extraction".to_string()));
        assert!(result.completed.contains(&"colmap".to_string()));
        assert!(result
            .completed
            .contains(&"nerfstudio_process_data".to_string()));
    }
}
