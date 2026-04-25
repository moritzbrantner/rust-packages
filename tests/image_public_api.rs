use tempfile::tempdir;

use image_analysis_core::{ImagePixelFormat, ImageView, OwnedImage};
use image_analysis_io::{read_image, write_image};
use image_analysis_segmentation::ImageSegmentationRequest;

#[test]
fn image_public_api_covers_core_defaults_and_io() -> Result<(), Box<dyn std::error::Error>> {
    let rgb = OwnedImage::new_rgb(2, 1, vec![255, 0, 0, 0, 255, 0])?;
    let gray = OwnedImage::new_gray(2, 1, vec![0, 255])?;
    let view = ImageView::packed(2, 1, ImagePixelFormat::Gray8, &[0, 255])?;
    assert_eq!(rgb.pixel_format, ImagePixelFormat::Rgb24);
    assert_eq!(gray.pixel_format, ImagePixelFormat::Gray8);
    assert_eq!(view.stride, 2);

    let default_request = ImageSegmentationRequest::default();
    assert!(!default_request.prompt.automatic_mask_generation);
    let automatic = ImageSegmentationRequest::automatic_mask_generation();
    assert!(automatic.prompt.automatic_mask_generation);

    let temp = tempdir()?;
    let path = temp.path().join("roundtrip.png");
    write_image(&path, &rgb)?;
    let roundtrip = read_image(&path)?;
    assert_eq!(roundtrip, rgb);

    Ok(())
}
