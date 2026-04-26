use std::path::PathBuf;

use video_analysis_use_cases::image_person_edit::{run_image_person_edit, ImagePersonEditRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_image_person_edit(ImagePersonEditRequest {
        input: PathBuf::from("input.png"),
        work_dir: PathBuf::from("use-case-output/image-person-edit"),
        prompt: "replace the detected person with a marble statue".to_string(),
        model: "flux1-dev.safetensors".to_string(),
        person_detector_command: PathBuf::from("python3"),
        person_detector_args: vec!["scripts/opencv_person_detector.py".to_string()],
        ..ImagePersonEditRequest::default()
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
