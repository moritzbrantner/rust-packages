use std::path::PathBuf;

use video_analysis_use_cases::video_red_cars::{run_video_red_cars, VideoRedCarsRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_video_red_cars(VideoRedCarsRequest {
        input: PathBuf::from("tests/fixtures/me-at-the-zoo-jNQXAC9IVRw.webm"),
        work_dir: PathBuf::from("use-case-output/video-red-cars"),
        vehicle_detector_command: PathBuf::from("python3"),
        vehicle_detector_args: vec!["scripts/opencv_red_car_detector.py".to_string()],
        ..VideoRedCarsRequest::default()
    })?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
