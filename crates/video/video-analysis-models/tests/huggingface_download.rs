#[cfg(feature = "external-tests")]
mod external {
    use video_analysis_models::{HuggingFaceDownloader, HuggingFaceModelSpec, ModelTask};

    #[test]
    #[ignore = "requires network access to Hugging Face"]
    fn downloads_tiny_huggingface_model_files() {
        let dir = tempfile::tempdir().unwrap();
        let spec = HuggingFaceModelSpec::new(
            "hf-internal-testing/tiny-random-bert",
            ModelTask::TextEmbedding,
        )
        .name("tiny-random-bert")
        .file("config.json")
        .file("model.safetensors");

        let downloaded = HuggingFaceDownloader::new()
            .cache_dir(dir.path())
            .progress(false)
            .download(&spec)
            .unwrap();

        video_analysis_test_support::assert_nonempty_file(&downloaded.files["config.json"]);
        video_analysis_test_support::assert_nonempty_file(&downloaded.files["model.safetensors"]);
    }
}
