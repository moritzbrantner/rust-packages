#[cfg(feature = "external-tests")]
mod external {
    use std::path::{Path, PathBuf};

    use video_analysis_radiance_pipeline::{VideoToRadiancePipeline, VideoToRadianceRequest};

    fn require_command(command: &str) {
        find_command(command)
            .unwrap_or_else(|| panic!("required command `{command}` is unavailable"));
    }

    fn find_command(command: &str) -> Option<PathBuf> {
        let path = Path::new(command);
        if path.components().count() > 1 && path.is_file() {
            return Some(path.to_path_buf());
        }
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(command))
                .find(|candidate| candidate.is_file())
        })
    }

    #[test]
    #[ignore = "requires ffmpeg, colmap, and Nerfstudio CLI commands"]
    fn real_external_radiance_pipeline_reaches_process_data_stage() {
        require_command("ffmpeg");
        require_command("colmap");
        require_command("ns-process-data");

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
