# tensor-data

Small finite `f32` tensor contracts and metadata for `video-analysis`.

## Highlights

- Checked tensor shapes with finite-value validation
- Borrowed and owned tensor views
- Lightweight metadata for interop-oriented tensor payloads

## Example

```rust,no_run
use tensor_data::{F32Tensor, TensorShape};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tensor = F32Tensor::new(TensorShape::new([1, 4, 64, 64])?, vec![0.0; 16_384])?;
    assert_eq!(tensor.shape().rank(), 4);
    assert_eq!(tensor.shape().element_count()?, 16_384);
    Ok(())
}
```

## Related crates

- `comfyui-latents`
- `audio-analysis-core`
- `image-analysis-core`
