#[cfg(feature = "external-tests")]
mod external {
    use video_analysis_models::{
        HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundleStore, ModelTask,
    };

    #[test]
    #[ignore = "requires network access to Hugging Face"]
    fn downloads_tiny_huggingface_model_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let spec = HuggingFaceModelSpec::new(
            "hf-internal-testing/tiny-random-bert",
            ModelTask::TextEmbedding,
        )
        .name("tiny-random-bert")
        .file("config.json")
        .file("model.safetensors");

        let bundle = ModelBundleStore::new(dir.path().join("bundles"))
            .downloader(
                HuggingFaceDownloader::new()
                    .cache_dir(dir.path().join("cache"))
                    .progress(false),
            )
            .download(&spec)
            .unwrap();

        video_analysis_test_support::assert_nonempty_file(bundle.manifest_path());
        video_analysis_test_support::assert_nonempty_file(bundle.file_path("config.json").unwrap());
        video_analysis_test_support::assert_nonempty_file(
            bundle.file_path("model.safetensors").unwrap(),
        );
    }
}
