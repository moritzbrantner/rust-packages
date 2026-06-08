#![doc = include_str!("../README.md")]

use std::path::Path;

#[cfg(feature = "onnxruntime")]
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnnxRuntimeError {
    #[error("ONNX Runtime support is unavailable in this build")]
    Unavailable,
    #[error("invalid ONNX runtime argument: {0}")]
    InvalidArgument(String),
    #[error("invalid ONNX tensor shape: {0}")]
    InvalidTensorShape(String),
    #[error("unsupported ONNX tensor type: {0}")]
    UnsupportedTensorType(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("ONNX Runtime source error: {0}")]
    Source(String),
}

pub type Result<T> = std::result::Result<T, OnnxRuntimeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnnxTensorElementType {
    F32,
    I64,
    I32,
    U8,
}

pub trait OnnxTensorElement: Clone + std::fmt::Debug + Send + Sync + 'static {
    fn validate_values(_values: &[Self]) -> Result<()> {
        Ok(())
    }
}

impl OnnxTensorElement for f32 {
    fn validate_values(values: &[Self]) -> Result<()> {
        if values.iter().any(|value| !value.is_finite()) {
            return Err(OnnxRuntimeError::InvalidArgument(
                "f32 tensor values must be finite".to_string(),
            ));
        }
        Ok(())
    }
}

impl OnnxTensorElement for i64 {}
impl OnnxTensorElement for i32 {}
impl OnnxTensorElement for u8 {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnnxTensor<T> {
    pub shape: Vec<usize>,
    pub values: Vec<T>,
}

impl<T: OnnxTensorElement> OnnxTensor<T> {
    pub fn new(shape: Vec<usize>, values: Vec<T>) -> Result<Self> {
        let expected = element_count(&shape)?;
        if expected != values.len() {
            return Err(OnnxRuntimeError::InvalidTensorShape(format!(
                "shape {shape:?} expects {expected} value(s), got {}",
                values.len()
            )));
        }
        T::validate_values(&values)?;
        Ok(Self { shape, values })
    }
}

pub type OnnxF32Tensor = OnnxTensor<f32>;
pub type OnnxI64Tensor = OnnxTensor<i64>;
pub type OnnxI32Tensor = OnnxTensor<i32>;
pub type OnnxU8Tensor = OnnxTensor<u8>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OnnxTensorValue {
    F32(OnnxF32Tensor),
    I64(OnnxI64Tensor),
    I32(OnnxI32Tensor),
    U8(OnnxU8Tensor),
}

impl OnnxTensorValue {
    pub fn element_type(&self) -> OnnxTensorElementType {
        match self {
            Self::F32(_) => OnnxTensorElementType::F32,
            Self::I64(_) => OnnxTensorElementType::I64,
            Self::I32(_) => OnnxTensorElementType::I32,
            Self::U8(_) => OnnxTensorElementType::U8,
        }
    }

    pub fn shape(&self) -> &[usize] {
        match self {
            Self::F32(tensor) => &tensor.shape,
            Self::I64(tensor) => &tensor.shape,
            Self::I32(tensor) => &tensor.shape,
            Self::U8(tensor) => &tensor.shape,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnnxNamedTensor {
    pub name: String,
    pub tensor: OnnxTensorValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnnxDimension {
    Fixed(usize),
    Symbolic(String),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnnxIoInfo {
    pub name: String,
    pub element_type: Option<OnnxTensorElementType>,
    pub dimensions: Vec<OnnxDimension>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OnnxSessionMetadata {
    pub inputs: Vec<OnnxIoInfo>,
    pub outputs: Vec<OnnxIoInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnnxSessionOptions {
    pub graph_optimization: OnnxGraphOptimization,
    pub execution_provider: OnnxExecutionProvider,
}

impl Default for OnnxSessionOptions {
    fn default() -> Self {
        Self {
            graph_optimization: OnnxGraphOptimization::Default,
            execution_provider: OnnxExecutionProvider::Cpu,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnnxExecutionProvider {
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnnxGraphOptimization {
    Default,
    Disable,
}

pub trait OnnxRunner {
    fn metadata(&self) -> Result<OnnxSessionMetadata>;
    fn run(&mut self, inputs: Vec<OnnxNamedTensor>) -> Result<Vec<OnnxNamedTensor>>;
}

#[derive(Debug)]
pub struct OnnxSession {
    #[cfg(feature = "onnxruntime")]
    session: Mutex<ort::session::Session>,
}

impl OnnxSession {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_file_with_options(path, OnnxSessionOptions::default())
    }

    pub fn from_file_with_options(
        path: impl AsRef<Path>,
        options: OnnxSessionOptions,
    ) -> Result<Self> {
        from_file_impl(path.as_ref(), options)
    }
}

#[cfg(not(feature = "onnxruntime"))]
fn from_file_impl(_path: &Path, _options: OnnxSessionOptions) -> Result<OnnxSession> {
    Err(OnnxRuntimeError::Unavailable)
}

#[cfg(feature = "onnxruntime")]
fn from_file_impl(path: &Path, options: OnnxSessionOptions) -> Result<OnnxSession> {
    if !path.is_file() {
        return Err(OnnxRuntimeError::InvalidArgument(format!(
            "ONNX model file `{}` does not exist",
            path.display()
        )));
    }

    let builder = ort::session::Session::builder().map_err(ort_error)?;
    let builder = match options.graph_optimization {
        OnnxGraphOptimization::Default => builder,
        OnnxGraphOptimization::Disable => builder
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Disable)
            .map_err(ort_error)?,
    };
    let mut builder = match options.execution_provider {
        OnnxExecutionProvider::Cpu => builder
            .with_no_environment_execution_providers()
            .and_then(|builder| {
                builder.with_execution_providers([ort::ep::CPUExecutionProvider::default().build()])
            })
            .map_err(ort_error)?,
    };
    let session = builder.commit_from_file(path).map_err(ort_error)?;
    Ok(OnnxSession {
        session: Mutex::new(session),
    })
}

#[cfg(not(feature = "onnxruntime"))]
impl OnnxRunner for OnnxSession {
    fn metadata(&self) -> Result<OnnxSessionMetadata> {
        Err(OnnxRuntimeError::Unavailable)
    }

    fn run(&mut self, _inputs: Vec<OnnxNamedTensor>) -> Result<Vec<OnnxNamedTensor>> {
        Err(OnnxRuntimeError::Unavailable)
    }
}

#[cfg(feature = "onnxruntime")]
impl OnnxRunner for OnnxSession {
    fn metadata(&self) -> Result<OnnxSessionMetadata> {
        let session = self
            .session
            .lock()
            .map_err(|_| OnnxRuntimeError::Source("ONNX session mutex was poisoned".to_string()))?;
        Ok(session_metadata(&session))
    }

    fn run(&mut self, inputs: Vec<OnnxNamedTensor>) -> Result<Vec<OnnxNamedTensor>> {
        use std::borrow::Cow;

        use ort::session::SessionInputValue;
        use ort::value::Tensor;

        let mut ort_inputs = Vec::<(Cow<'_, str>, SessionInputValue<'_>)>::new();
        for input in inputs {
            let name = Cow::from(input.name);
            let value: SessionInputValue<'_> = match input.tensor {
                OnnxTensorValue::F32(tensor) => {
                    Tensor::<f32>::from_array((shape_to_i64(&tensor.shape)?, tensor.values))
                        .map_err(ort_error)?
                        .into()
                }
                OnnxTensorValue::I64(tensor) => {
                    Tensor::<i64>::from_array((shape_to_i64(&tensor.shape)?, tensor.values))
                        .map_err(ort_error)?
                        .into()
                }
                OnnxTensorValue::I32(tensor) => {
                    Tensor::<i32>::from_array((shape_to_i64(&tensor.shape)?, tensor.values))
                        .map_err(ort_error)?
                        .into()
                }
                OnnxTensorValue::U8(tensor) => {
                    Tensor::<u8>::from_array((shape_to_i64(&tensor.shape)?, tensor.values))
                        .map_err(ort_error)?
                        .into()
                }
            };
            ort_inputs.push((name, value));
        }

        let mut session = self
            .session
            .lock()
            .map_err(|_| OnnxRuntimeError::Source("ONNX session mutex was poisoned".to_string()))?;
        let outputs = session.run(ort_inputs).map_err(ort_error)?;
        let mut named = Vec::with_capacity(outputs.len());
        for (name, value) in outputs {
            named.push(OnnxNamedTensor {
                name: name.to_string(),
                tensor: extract_tensor_value(&value)?,
            });
        }
        Ok(named)
    }
}

pub fn input_name(metadata: &OnnxSessionMetadata, index: usize) -> Result<&str> {
    metadata
        .inputs
        .get(index)
        .map(|input| input.name.as_str())
        .ok_or_else(|| OnnxRuntimeError::InvalidArgument(format!("missing ONNX input #{index}")))
}

pub fn output_name(metadata: &OnnxSessionMetadata, index: usize) -> Result<&str> {
    metadata
        .outputs
        .get(index)
        .map(|output| output.name.as_str())
        .ok_or_else(|| OnnxRuntimeError::InvalidArgument(format!("missing ONNX output #{index}")))
}

pub fn first_f32_output(outputs: &[OnnxNamedTensor]) -> Result<&OnnxF32Tensor> {
    f32_output_by_name_or_index(outputs, "", 0)
}

pub fn f32_output_by_name_or_index<'a>(
    outputs: &'a [OnnxNamedTensor],
    name: &str,
    index: usize,
) -> Result<&'a OnnxF32Tensor> {
    match output_by_name_or_index(outputs, name, index)? {
        OnnxTensorValue::F32(tensor) => Ok(tensor),
        other => Err(OnnxRuntimeError::UnsupportedTensorType(format!(
            "expected f32 output `{name}`/#{index}, got {:?}",
            other.element_type()
        ))),
    }
}

pub fn i64_output_by_name_or_index<'a>(
    outputs: &'a [OnnxNamedTensor],
    name: &str,
    index: usize,
) -> Result<&'a OnnxI64Tensor> {
    match output_by_name_or_index(outputs, name, index)? {
        OnnxTensorValue::I64(tensor) => Ok(tensor),
        other => Err(OnnxRuntimeError::UnsupportedTensorType(format!(
            "expected i64 output `{name}`/#{index}, got {:?}",
            other.element_type()
        ))),
    }
}

pub fn single_f32_input(
    name: impl Into<String>,
    shape: Vec<usize>,
    values: Vec<f32>,
) -> Result<OnnxNamedTensor> {
    Ok(OnnxNamedTensor {
        name: name.into(),
        tensor: OnnxTensorValue::F32(OnnxTensor::new(shape, values)?),
    })
}

pub fn single_i64_input(
    name: impl Into<String>,
    shape: Vec<usize>,
    values: Vec<i64>,
) -> Result<OnnxNamedTensor> {
    Ok(OnnxNamedTensor {
        name: name.into(),
        tensor: OnnxTensorValue::I64(OnnxTensor::new(shape, values)?),
    })
}

fn output_by_name_or_index<'a>(
    outputs: &'a [OnnxNamedTensor],
    name: &str,
    index: usize,
) -> Result<&'a OnnxTensorValue> {
    if !name.is_empty() {
        if let Some(output) = outputs.iter().find(|output| output.name == name) {
            return Ok(&output.tensor);
        }
    }
    outputs
        .get(index)
        .map(|output| &output.tensor)
        .ok_or_else(|| {
            OnnxRuntimeError::InvalidArgument(format!(
                "missing ONNX output `{name}` and fallback index #{index}"
            ))
        })
}

fn element_count(shape: &[usize]) -> Result<usize> {
    shape.iter().try_fold(1usize, |acc, dim| {
        acc.checked_mul(*dim).ok_or_else(|| {
            OnnxRuntimeError::InvalidTensorShape(format!(
                "shape {shape:?} element count overflowed"
            ))
        })
    })
}

#[cfg(feature = "onnxruntime")]
fn session_metadata(session: &ort::session::Session) -> OnnxSessionMetadata {
    OnnxSessionMetadata {
        inputs: session.inputs().iter().map(io_info).collect(),
        outputs: session.outputs().iter().map(io_info).collect(),
    }
}

#[cfg(feature = "onnxruntime")]
fn io_info(outlet: &ort::value::Outlet) -> OnnxIoInfo {
    match outlet.dtype() {
        ort::value::ValueType::Tensor {
            ty,
            shape,
            dimension_symbols,
        } => OnnxIoInfo {
            name: outlet.name().to_string(),
            element_type: element_type_from_ort(*ty),
            dimensions: shape
                .iter()
                .enumerate()
                .map(|(index, dim)| {
                    if *dim >= 0 {
                        OnnxDimension::Fixed(*dim as usize)
                    } else {
                        dimension_symbols
                            .get(index)
                            .filter(|symbol| !symbol.is_empty())
                            .cloned()
                            .map(OnnxDimension::Symbolic)
                            .unwrap_or(OnnxDimension::Unknown)
                    }
                })
                .collect(),
        },
        _ => OnnxIoInfo {
            name: outlet.name().to_string(),
            element_type: None,
            dimensions: Vec::new(),
        },
    }
}

#[cfg(feature = "onnxruntime")]
fn extract_tensor_value(value: &ort::value::DynValue) -> Result<OnnxTensorValue> {
    match value.dtype() {
        ort::value::ValueType::Tensor { ty, .. } => match ty {
            ort::value::TensorElementType::Float32 => {
                let (shape, values) = value.try_extract_tensor::<f32>().map_err(ort_error)?;
                Ok(OnnxTensorValue::F32(OnnxTensor::new(
                    shape_from_ort(shape)?,
                    values.to_vec(),
                )?))
            }
            ort::value::TensorElementType::Int64 => {
                let (shape, values) = value.try_extract_tensor::<i64>().map_err(ort_error)?;
                Ok(OnnxTensorValue::I64(OnnxTensor::new(
                    shape_from_ort(shape)?,
                    values.to_vec(),
                )?))
            }
            ort::value::TensorElementType::Int32 => {
                let (shape, values) = value.try_extract_tensor::<i32>().map_err(ort_error)?;
                Ok(OnnxTensorValue::I32(OnnxTensor::new(
                    shape_from_ort(shape)?,
                    values.to_vec(),
                )?))
            }
            ort::value::TensorElementType::Uint8 => {
                let (shape, values) = value.try_extract_tensor::<u8>().map_err(ort_error)?;
                Ok(OnnxTensorValue::U8(OnnxTensor::new(
                    shape_from_ort(shape)?,
                    values.to_vec(),
                )?))
            }
            other => Err(OnnxRuntimeError::UnsupportedTensorType(format!("{other}"))),
        },
        other => Err(OnnxRuntimeError::UnsupportedTensorType(format!(
            "{other:?}"
        ))),
    }
}

#[cfg(feature = "onnxruntime")]
fn shape_from_ort(shape: &ort::value::Shape) -> Result<Vec<usize>> {
    shape
        .iter()
        .map(|dim| {
            usize::try_from(*dim).map_err(|_| {
                OnnxRuntimeError::InvalidTensorShape(format!(
                    "ONNX output shape contains negative dimension {dim}"
                ))
            })
        })
        .collect()
}

#[cfg(feature = "onnxruntime")]
fn shape_to_i64(shape: &[usize]) -> Result<Vec<i64>> {
    shape
        .iter()
        .map(|dim| {
            i64::try_from(*dim).map_err(|_| {
                OnnxRuntimeError::InvalidTensorShape(format!(
                    "ONNX input shape dimension {dim} does not fit in i64"
                ))
            })
        })
        .collect()
}

#[cfg(feature = "onnxruntime")]
fn element_type_from_ort(ty: ort::value::TensorElementType) -> Option<OnnxTensorElementType> {
    match ty {
        ort::value::TensorElementType::Float32 => Some(OnnxTensorElementType::F32),
        ort::value::TensorElementType::Int64 => Some(OnnxTensorElementType::I64),
        ort::value::TensorElementType::Int32 => Some(OnnxTensorElementType::I32),
        ort::value::TensorElementType::Uint8 => Some(OnnxTensorElementType::U8),
        _ => None,
    }
}

#[cfg(feature = "onnxruntime")]
fn ort_error<T>(error: ort::Error<T>) -> OnnxRuntimeError {
    OnnxRuntimeError::Source(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_shape_validation_rejects_mismatched_element_counts() {
        let err = OnnxF32Tensor::new(vec![2, 2], vec![1.0, 2.0, 3.0]).unwrap_err();
        assert!(matches!(err, OnnxRuntimeError::InvalidTensorShape(_)));
    }

    #[test]
    fn f32_tensor_validation_rejects_nan_and_inf() {
        assert!(OnnxF32Tensor::new(vec![1], vec![f32::NAN]).is_err());
        assert!(OnnxF32Tensor::new(vec![1], vec![f32::INFINITY]).is_err());
    }

    #[test]
    fn named_tensor_lookup_by_name_and_index_is_deterministic() {
        let first = single_f32_input("first", vec![1], vec![1.0]).unwrap();
        let second = single_f32_input("second", vec![1], vec![2.0]).unwrap();
        let outputs = vec![first, second];
        assert_eq!(
            f32_output_by_name_or_index(&outputs, "second", 0)
                .unwrap()
                .values,
            vec![2.0]
        );
        assert_eq!(
            f32_output_by_name_or_index(&outputs, "missing", 0)
                .unwrap()
                .values,
            vec![1.0]
        );
    }

    #[test]
    fn unsupported_output_dtype_returns_typed_error_through_helpers() {
        let outputs = vec![single_i64_input("ids", vec![1], vec![42]).unwrap()];
        let err = f32_output_by_name_or_index(&outputs, "ids", 0).unwrap_err();
        assert!(matches!(err, OnnxRuntimeError::UnsupportedTensorType(_)));
    }

    #[cfg(not(feature = "onnxruntime"))]
    #[test]
    fn no_feature_session_construction_returns_unavailable() {
        let err = OnnxSession::from_file("missing.onnx").unwrap_err();
        assert!(matches!(err, OnnxRuntimeError::Unavailable));
    }

    #[cfg(feature = "onnxruntime")]
    #[test]
    fn missing_model_path_returns_typed_error() {
        let err = OnnxSession::from_file("missing.onnx").unwrap_err();
        assert!(matches!(err, OnnxRuntimeError::InvalidArgument(_)));
    }
}
