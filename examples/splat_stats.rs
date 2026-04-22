use std::env;

use video_analysis::radiance_io::read_gaussian_splat_ply;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "export/splat.ply".to_string());
    let scene = read_gaussian_splat_ply(path)?;
    let stats = scene.stats()?;

    println!("splats: {}", stats.count);
    println!("mean opacity: {:.4}", stats.mean_opacity);
    if let Some(bounds) = stats.bounds {
        println!(
            "bounds min=({:.3}, {:.3}, {:.3}) max=({:.3}, {:.3}, {:.3})",
            bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z
        );
    }
    Ok(())
}
