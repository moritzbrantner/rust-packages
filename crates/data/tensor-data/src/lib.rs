#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorShape {
    dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: impl Into<Vec<usize>>) -> Result<Self> {
        let shape = Self { dims: dims.into() };
        shape.validate()?;
        Ok(shape)
    }

    pub fn dimensions(&self) -> &[usize] {
        &self.dims
    }

    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    pub fn element_count(&self) -> Result<usize> {
        self.dims.iter().try_fold(1_usize, |count, dimension| {
            count
                .checked_mul(*dimension)
                .ok_or_else(|| invalid_argument("tensor shape element count overflowed usize"))
        })
    }

    pub fn reshape(&self, dims: impl Into<Vec<usize>>) -> Result<Self> {
        let reshaped = Self::new(dims)?;
        if reshaped.element_count()? != self.element_count()? {
            return Err(invalid_argument(format!(
                "cannot reshape tensor with {} elements into {} elements",
                self.element_count()?,
                reshaped.element_count()?
            )));
        }
        Ok(reshaped)
    }

    fn validate(&self) -> Result<()> {
        if self.dims.is_empty() {
            return Err(invalid_argument(
                "tensor shape must have at least one dimension",
            ));
        }
        if self.dims.iter().any(|dimension| *dimension == 0) {
            return Err(invalid_argument(
                "tensor shape dimensions must be greater than zero",
            ));
        }
        let _ = self.element_count()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct F32Tensor {
    shape: TensorShape,
    values: Vec<f32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    metadata: BTreeMap<String, Value>,
}

impl F32Tensor {
    pub fn new(shape: TensorShape, values: Vec<f32>) -> Result<Self> {
        let tensor = Self {
            shape,
            values,
            metadata: BTreeMap::new(),
        };
        tensor.validate()?;
        Ok(tensor)
    }

    pub fn from_dims(dims: impl Into<Vec<usize>>, values: Vec<f32>) -> Result<Self> {
        Self::new(TensorShape::new(dims)?, values)
    }

    pub fn shape(&self) -> &TensorShape {
        &self.shape
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn into_values(self) -> Vec<f32> {
        self.values
    }

    pub fn metadata(&self) -> &BTreeMap<String, Value> {
        &self.metadata
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn reshape(mut self, dims: impl Into<Vec<usize>>) -> Result<Self> {
        self.shape = self.shape.reshape(dims)?;
        Ok(self)
    }

    pub fn as_view(&self) -> F32TensorView<'_> {
        F32TensorView {
            shape: self.shape.clone(),
            values: &self.values,
            metadata: self.metadata.clone(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        let expected = self.shape.element_count()?;
        if expected != self.values.len() {
            return Err(invalid_argument(format!(
                "tensor shape expects {expected} elements but tensor has {}",
                self.values.len()
            )));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("tensor values must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct F32TensorView<'a> {
    shape: TensorShape,
    values: &'a [f32],
    metadata: BTreeMap<String, Value>,
}

impl<'a> F32TensorView<'a> {
    pub fn new(shape: TensorShape, values: &'a [f32]) -> Result<Self> {
        let view = Self {
            shape,
            values,
            metadata: BTreeMap::new(),
        };
        view.validate()?;
        Ok(view)
    }

    pub fn from_dims(dims: impl Into<Vec<usize>>, values: &'a [f32]) -> Result<Self> {
        Self::new(TensorShape::new(dims)?, values)
    }

    pub fn shape(&self) -> &TensorShape {
        &self.shape
    }

    pub fn values(&self) -> &'a [f32] {
        self.values
    }

    pub fn metadata(&self) -> &BTreeMap<String, Value> {
        &self.metadata
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn reshape(mut self, dims: impl Into<Vec<usize>>) -> Result<Self> {
        self.shape = self.shape.reshape(dims)?;
        Ok(self)
    }

    pub fn into_owned(self) -> Result<F32Tensor> {
        let mut tensor = F32Tensor::new(self.shape, self.values.to_vec())?;
        tensor.metadata = self.metadata;
        Ok(tensor)
    }

    pub fn validate(&self) -> Result<()> {
        let expected = self.shape.element_count()?;
        if expected != self.values.len() {
            return Err(invalid_argument(format!(
                "tensor shape expects {expected} elements but tensor view has {}",
                self.values.len()
            )));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("tensor values must be finite"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_or_zero_dimension_shapes() {
        assert!(TensorShape::new(Vec::<usize>::new()).is_err());
        assert!(TensorShape::new([1, 0, 2]).is_err());
    }

    #[test]
    fn rejects_wrong_element_count() {
        let error = F32Tensor::from_dims([2, 2], vec![0.0; 3]).unwrap_err();
        assert!(matches!(error, DetectError::InvalidArgument(_)));
    }

    #[test]
    fn rejects_non_finite_values() {
        let error = F32Tensor::from_dims([1, 2], vec![0.0, f32::NAN]).unwrap_err();
        assert!(matches!(error, DetectError::InvalidArgument(_)));
    }

    #[test]
    fn reshapes_when_element_counts_match() {
        let tensor = F32Tensor::from_dims([1, 4], vec![0.0; 4]).unwrap();
        let reshaped = tensor.reshape([2, 2]).unwrap();
        assert_eq!(reshaped.shape().dimensions(), &[2, 2]);
    }

    #[test]
    fn view_round_trips_into_owned_tensor() {
        let view = F32TensorView::from_dims([1, 1, 2], &[0.25, 0.75]).unwrap();
        let owned = view.into_owned().unwrap();
        assert_eq!(owned.values(), &[0.25, 0.75]);
    }
}
