#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use tensor_data::F32Tensor;
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatentImageSize {
    pub width: u32,
    pub height: u32,
}

impl LatentImageSize {
    pub const SCALE_FACTOR: u32 = 8;

    pub fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(DetectError::InvalidDimensions { width, height });
        }
        Ok(Self { width, height })
    }

    pub fn from_latent_dimensions(latent_height: usize, latent_width: usize) -> Result<Self> {
        let width = u32::try_from(latent_width)
            .map_err(|_| invalid_argument("latent width exceeds u32"))?
            .checked_mul(Self::SCALE_FACTOR)
            .ok_or_else(|| invalid_argument("image width overflowed u32"))?;
        let height = u32::try_from(latent_height)
            .map_err(|_| invalid_argument("latent height exceeds u32"))?
            .checked_mul(Self::SCALE_FACTOR)
            .ok_or_else(|| invalid_argument("image height overflowed u32"))?;
        Self::new(width, height)
    }

    pub fn latent_dimensions(self) -> Result<(usize, usize)> {
        if !self.width.is_multiple_of(Self::SCALE_FACTOR)
            || !self.height.is_multiple_of(Self::SCALE_FACTOR)
        {
            return Err(invalid_argument(
                "image size must be divisible by the latent scale factor",
            ));
        }
        Ok((
            (self.height / Self::SCALE_FACTOR) as usize,
            (self.width / Self::SCALE_FACTOR) as usize,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatentMask {
    tensor: F32Tensor,
}

impl LatentMask {
    pub fn new(tensor: F32Tensor) -> Result<Self> {
        let mask = Self { tensor };
        mask.validate()?;
        Ok(mask)
    }

    pub fn from_image_mask(mask: F32Tensor, image_size: LatentImageSize) -> Result<Self> {
        mask.validate()?;
        let dims = mask.shape().dimensions();
        if dims.len() != 2 {
            return Err(invalid_argument(
                "image masks must be rank 2 [H,W] tensors before latent conversion",
            ));
        }
        if dims[0] != image_size.height as usize || dims[1] != image_size.width as usize {
            return Err(invalid_argument(format!(
                "image mask dimensions [{}, {}] do not match image size [{}, {}]",
                dims[0], dims[1], image_size.height, image_size.width
            )));
        }
        let (latent_height, latent_width) = image_size.latent_dimensions()?;
        let mut values = Vec::with_capacity(latent_height * latent_width);
        for latent_y in 0..latent_height {
            for latent_x in 0..latent_width {
                let mut pooled = 0.0_f32;
                for image_y in latent_y * LatentImageSize::SCALE_FACTOR as usize
                    ..(latent_y + 1) * LatentImageSize::SCALE_FACTOR as usize
                {
                    for image_x in latent_x * LatentImageSize::SCALE_FACTOR as usize
                        ..(latent_x + 1) * LatentImageSize::SCALE_FACTOR as usize
                    {
                        let index = image_y * image_size.width as usize + image_x;
                        pooled = pooled.max(mask.values()[index]);
                    }
                }
                values.push(pooled);
            }
        }
        Self::new(F32Tensor::from_dims([latent_height, latent_width], values)?)
    }

    pub fn tensor(&self) -> &F32Tensor {
        &self.tensor
    }

    pub fn rank(&self) -> usize {
        self.tensor.shape().rank()
    }

    pub fn spatial_dimensions(&self) -> (usize, usize) {
        let dims = self.tensor.shape().dimensions();
        match dims {
            [height, width] => (*height, *width),
            [_, _, height, width] => (*height, *width),
            _ => (0, 0),
        }
    }

    pub fn compatible_with(&self, batch: &LatentBatch) -> bool {
        let dims = self.tensor.shape().dimensions();
        match dims {
            [height, width] => *height == batch.latent_height() && *width == batch.latent_width(),
            [mask_batch, channels, height, width] => {
                (*mask_batch == 1 || *mask_batch == batch.batch_size())
                    && (*channels == 1 || *channels == batch.channel_count())
                    && *height == batch.latent_height()
                    && *width == batch.latent_width()
            }
            _ => false,
        }
    }

    fn validate(&self) -> Result<()> {
        self.tensor.validate()?;
        match self.tensor.shape().dimensions() {
            [_, _] => Ok(()),
            [_, channels, _, _] if *channels > 0 => Ok(()),
            [_, _, _, _] => Err(invalid_argument(
                "latent mask 4D tensors must have a positive channel dimension",
            )),
            _ => Err(invalid_argument(
                "latent mask tensors must be rank 2 [H,W] or rank 4 [B,C,H,W]",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatentBatch {
    samples: F32Tensor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mask: Option<LatentMask>,
}

impl LatentBatch {
    pub fn new(samples: F32Tensor) -> Result<Self> {
        let batch = Self {
            samples,
            mask: None,
        };
        batch.validate()?;
        Ok(batch)
    }

    pub fn samples(&self) -> &F32Tensor {
        &self.samples
    }

    pub fn mask(&self) -> Option<&LatentMask> {
        self.mask.as_ref()
    }

    pub fn with_mask(mut self, mask: LatentMask) -> Result<Self> {
        if !mask.compatible_with(&self) {
            return Err(invalid_argument(
                "latent mask shape is incompatible with latent batch shape",
            ));
        }
        self.mask = Some(mask);
        Ok(self)
    }

    pub fn batch_size(&self) -> usize {
        self.samples.shape().dimensions()[0]
    }

    pub fn channel_count(&self) -> usize {
        self.samples.shape().dimensions()[1]
    }

    pub fn latent_height(&self) -> usize {
        self.samples.shape().dimensions()[2]
    }

    pub fn latent_width(&self) -> usize {
        self.samples.shape().dimensions()[3]
    }

    pub fn image_size(&self) -> Result<LatentImageSize> {
        LatentImageSize::from_latent_dimensions(self.latent_height(), self.latent_width())
    }

    pub fn validate(&self) -> Result<()> {
        self.samples.validate()?;
        if self.samples.shape().rank() != 4 {
            return Err(invalid_argument(
                "latent batches must use rank 4 [B,C,H,W] tensors",
            ));
        }
        if let Some(mask) = &self.mask {
            mask.validate()?;
            if !mask.compatible_with(self) {
                return Err(invalid_argument(
                    "latent mask shape is incompatible with latent batch shape",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tensor_data::F32Tensor;

    #[test]
    fn accepts_valid_latent_batch() {
        let batch =
            LatentBatch::new(F32Tensor::from_dims([1, 4, 64, 64], vec![0.0; 16_384]).unwrap())
                .unwrap();
        assert_eq!(batch.channel_count(), 4);
    }

    #[test]
    fn rejects_invalid_rank() {
        let error =
            LatentBatch::new(F32Tensor::from_dims([64, 64], vec![0.0; 4096]).unwrap()).unwrap_err();
        assert!(matches!(error, DetectError::InvalidArgument(_)));
    }

    #[test]
    fn rejects_incompatible_mask() {
        let batch =
            LatentBatch::new(F32Tensor::from_dims([1, 4, 64, 64], vec![0.0; 16_384]).unwrap())
                .unwrap();
        let mask = LatentMask::new(F32Tensor::from_dims([1, 1, 32, 32], vec![1.0; 1024]).unwrap())
            .unwrap();
        assert!(batch.with_mask(mask).is_err());
    }

    #[test]
    fn converts_latent_dimensions_into_image_size() {
        let size = LatentImageSize::from_latent_dimensions(64, 96).unwrap();
        assert_eq!(size.width, 768);
        assert_eq!(size.height, 512);
        assert_eq!(size.latent_dimensions().unwrap(), (64, 96));
    }

    #[test]
    fn converts_image_mask_into_pooled_latent_mask() {
        let mut values = vec![0.0_f32; 16 * 16];
        values[7 * 16 + 7] = 1.0;
        values[8 * 16 + 8] = 0.5;
        let mask = LatentMask::from_image_mask(
            F32Tensor::from_dims([16, 16], values).unwrap(),
            LatentImageSize::new(16, 16).unwrap(),
        )
        .unwrap();
        assert_eq!(mask.tensor().shape().dimensions(), &[2, 2]);
        assert_eq!(mask.tensor().values(), &[1.0, 0.0, 0.0, 0.5]);
    }
}
