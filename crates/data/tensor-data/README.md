# tensor-data

Small finite `f32` tensor contracts and metadata for `video-analysis`.

## Highlights

- Checked tensor shapes with finite-value validation
- Borrowed and owned tensor views
- Lightweight metadata for interop-oriented tensor payloads
- Reshape helpers that preserve element counts

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

## Behavior

`TensorShape` rejects empty shapes and zero-sized dimensions. Element counts are
computed with checked `usize` multiplication, so extremely large shapes fail
instead of wrapping.

`F32Tensor` and `F32TensorView` require the number of values to exactly match
the shape element count. All tensor values must be finite; `NaN`, positive
infinity, and negative infinity are rejected during construction and validation.

Reshaping changes only the shape metadata. It succeeds when the new dimensions
produce the same element count and fails otherwise. Tensor values stay in their
existing contiguous order.

Metadata is optional JSON metadata carried alongside tensors for transport and
interop. It is not interpreted by this crate and is preserved when converting a
view into an owned tensor.

## Related crates

- `comfyui-latents`
- `audio-analysis-core`
- `image-analysis-core`
