use std::env;

use video_analysis_core::{Result, VideoAnalysisPipeline};
use video_analysis_models::{
    ExternalCommandModel, HuggingFaceModelSpec, ModelBundleStore, ModelPreset, ModelVideoAnalyzer,
};

fn main() -> Result<()> {
    let spec = HuggingFaceModelSpec::from_preset(ModelPreset::DetrResnet50);
    let downloaded = ModelBundleStore::new(".video-analysis-models")
        .download(&spec)?
        .to_downloaded_model();

    let model_name = downloaded.spec.name.clone();
    let command =
        env::var("VISION_MODEL_COMMAND").unwrap_or_else(|_| "scripts/detect-objects".to_string());
    let backend = ExternalCommandModel::new(command, downloaded).persistent();

    let analyzer = ModelVideoAnalyzer::new(model_name, backend);
    let _pipeline = VideoAnalysisPipeline::builder().analyzer(analyzer).build()?;
    Ok(())
}
