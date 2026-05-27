# image-analysis-io

Still-image PNG/JPEG/WebP loading and saving for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Runtime Surface

- `image.io.supportedFormats` lists PNG, JPEG, and WebP support.
- `image.io.inferFormat` infers format from a path extension.
- `image.io.plan` returns read/write support without reading or writing files.

## Example

```rust,no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use image_analysis_core::OwnedImage;
use image_analysis_io::{read_image, write_image};

let image = OwnedImage::new_rgb(2, 2, vec![255; 12])?;
write_image("preview.png", &image)?;
let roundtrip = read_image("preview.png")?;
assert_eq!(roundtrip.width, 2);
# Ok(())
# }
```

## Related crates

- `image-analysis-core`
- `image-analysis-processing`
