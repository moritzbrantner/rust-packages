use std::env;
use std::path::PathBuf;

use video_analysis::radiance_pipeline::{VideoToRadiancePipeline, VideoToRadianceRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = env::args()
        .nth(1)
        .unwrap_or_else(|| "input.mp4".to_string());
    let request = VideoToRadianceRequest {
        input: PathBuf::from(input),
        work_dir: PathBuf::from("use-case-output/radiance-scene"),
        frame_sample_every: 10,
        run_training: false,
        ..VideoToRadianceRequest::default()
    };

    let frame_args = VideoToRadiancePipeline::build_frame_extraction_args(&request)?;
    let colmap_steps = VideoToRadiancePipeline::build_colmap_args(&request)?;

    println!("ffmpeg args: {:?}", frame_args);
    println!("colmap steps: {}", colmap_steps.len());
    println!(
        "run the full workflow with: cargo run -p video-analysis-use-cases -- radiance-scene --input <video> --run-training"
    );
    Ok(())
}
