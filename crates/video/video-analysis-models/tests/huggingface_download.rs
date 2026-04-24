#[cfg(feature = "external-tests")]
mod external {
    use std::path::Path;

    use video_analysis_models::{
        HuggingFaceDownloader, HuggingFaceModelSpec, ModelBundleStore, ModelTask,
    };

    fn assert_nonempty_file(path: impl AsRef<Path>) {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)
            .unwrap_or_else(|err| panic!("expected `{}` metadata: {err}", path.display()));
        assert!(
            metadata.is_file() && metadata.len() > 0,
            "expected `{}` to be a non-empty file",
            path.display()
        );
    }

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

        assert_nonempty_file(bundle.manifest_path());
        assert_nonempty_file(bundle.file_path("config.json").unwrap());
        assert_nonempty_file(bundle.file_path("model.safetensors").unwrap());
    }
}
