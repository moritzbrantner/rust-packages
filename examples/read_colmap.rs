use std::env;

use video_analysis::radiance_io::{colmap_to_view_set, read_colmap_text_dir};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = env::args()
        .nth(1)
        .unwrap_or_else(|| "colmap/sparse/0".to_string());
    let dataset = read_colmap_text_dir(&dir)?;
    let views = colmap_to_view_set(&dataset)?;

    println!("cameras: {}", dataset.cameras.len());
    println!("images: {}", dataset.images.len());
    println!("points: {}", dataset.points.len());
    println!("views: {}", views.view_count());
    Ok(())
}
