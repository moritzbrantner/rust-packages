#![doc = include_str!("../README.md")]

pub mod surface;
use tensor_data::{F32Tensor, F32TensorView, TensorShape};
use vector_analysis_core::{cosine_similarity, dot, DenseVector};
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for matrix shape.
pub struct MatrixShape {
    /// The rows value.
    pub rows: usize,
    /// The cols value.
    pub cols: usize,
}

impl MatrixShape {
    /// Creates a new value.
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        let shape = Self { rows, cols };
        shape.validate()?;
        Ok(shape)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(invalid_argument(
                "matrix rows and cols must be greater than zero",
            ));
        }
        let _ = self.element_count()?;
        Ok(())
    }

    /// Returns element count.
    pub fn element_count(self) -> Result<usize> {
        self.rows
            .checked_mul(self.cols)
            .ok_or_else(|| invalid_argument("matrix element count overflowed usize"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing matrix layout.
pub enum MatrixLayout {
    /// The row major variant.
    RowMajor,
    /// The column major variant.
    ColumnMajor,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for f32 matrix.
pub struct F32Matrix {
    shape: MatrixShape,
    layout: MatrixLayout,
    values: Vec<f32>,
}

impl F32Matrix {
    /// Creates a new value.
    pub fn new(shape: MatrixShape, values: Vec<f32>) -> Result<Self> {
        let matrix = Self {
            shape,
            layout: MatrixLayout::RowMajor,
            values,
        };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Builds this value from rows.
    pub fn from_rows<const R: usize, const C: usize>(rows: [[f32; C]; R]) -> Result<Self> {
        let mut values = Vec::with_capacity(R * C);
        for row in rows {
            values.extend(row);
        }
        Self::new(MatrixShape::new(R, C)?, values)
    }

    /// Returns shape.
    pub fn shape(&self) -> MatrixShape {
        self.shape
    }

    /// Returns layout.
    pub fn layout(&self) -> MatrixLayout {
        self.layout
    }

    /// Returns values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Consumes this value into a values.
    pub fn into_values(self) -> Vec<f32> {
        self.values
    }

    /// Borrows this value as a view.
    pub fn as_view(&self) -> F32MatrixView<'_> {
        F32MatrixView {
            shape: self.shape,
            layout: self.layout,
            values: &self.values,
        }
    }

    /// Returns row.
    pub fn row(&self, index: usize) -> Result<RowView<'_>> {
        self.as_view().row(index)
    }

    /// Returns column.
    pub fn column(&self, index: usize) -> Result<ColumnView<'_>> {
        self.as_view().column(index)
    }

    /// Returns matmul.
    pub fn matmul(&self, right: &F32MatrixView<'_>) -> Result<Self> {
        self.as_view().matmul(right)
    }

    /// Returns matvec.
    pub fn matvec(&self, vector: &[f32]) -> Result<DenseVector> {
        self.as_view().matvec(vector)
    }

    /// Returns transpose view.
    pub fn transpose_view(&self) -> F32MatrixView<'_> {
        self.as_view().transpose()
    }

    /// Returns l2 normalize rows.
    pub fn l2_normalize_rows(&self) -> Result<Self> {
        self.as_view().l2_normalize_rows()
    }

    /// Returns l2 normalize columns.
    pub fn l2_normalize_columns(&self) -> Result<Self> {
        self.as_view().l2_normalize_columns()
    }

    /// Returns pairwise row cosine.
    pub fn pairwise_row_cosine(&self, right: &F32MatrixView<'_>) -> Result<Self> {
        self.as_view().pairwise_row_cosine(right)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        self.shape.validate()?;
        if self.values.len() != self.shape.element_count()? {
            return Err(invalid_argument(format!(
                "matrix shape expects {} values but matrix has {}",
                self.shape.element_count()?,
                self.values.len()
            )));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("matrix values must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for f32 matrix view.
pub struct F32MatrixView<'a> {
    shape: MatrixShape,
    layout: MatrixLayout,
    values: &'a [f32],
}

impl<'a> F32MatrixView<'a> {
    /// Creates a new value.
    pub fn new(shape: MatrixShape, values: &'a [f32]) -> Result<Self> {
        let view = Self {
            shape,
            layout: MatrixLayout::RowMajor,
            values,
        };
        view.validate()?;
        Ok(view)
    }

    /// Returns shape.
    pub fn shape(&self) -> MatrixShape {
        self.shape
    }

    /// Returns layout.
    pub fn layout(&self) -> MatrixLayout {
        self.layout
    }

    /// Returns values.
    pub fn values(&self) -> &'a [f32] {
        self.values
    }

    /// Returns transpose.
    pub fn transpose(self) -> Self {
        Self {
            shape: MatrixShape {
                rows: self.shape.cols,
                cols: self.shape.rows,
            },
            layout: match self.layout {
                MatrixLayout::RowMajor => MatrixLayout::ColumnMajor,
                MatrixLayout::ColumnMajor => MatrixLayout::RowMajor,
            },
            values: self.values,
        }
    }

    /// Returns row.
    pub fn row(self, index: usize) -> Result<RowView<'a>> {
        if index >= self.shape.rows {
            return Err(invalid_argument(format!(
                "row index {index} is out of bounds for {} rows",
                self.shape.rows
            )));
        }
        let (offset, stride) = match self.layout {
            MatrixLayout::RowMajor => (index * self.shape.cols, 1),
            MatrixLayout::ColumnMajor => (index, self.shape.rows),
        };
        Ok(RowView {
            values: self.values,
            len: self.shape.cols,
            offset,
            stride,
        })
    }

    /// Returns column.
    pub fn column(self, index: usize) -> Result<ColumnView<'a>> {
        if index >= self.shape.cols {
            return Err(invalid_argument(format!(
                "column index {index} is out of bounds for {} cols",
                self.shape.cols
            )));
        }
        let (offset, stride) = match self.layout {
            MatrixLayout::RowMajor => (index, self.shape.cols),
            MatrixLayout::ColumnMajor => (index * self.shape.rows, 1),
        };
        Ok(ColumnView {
            values: self.values,
            len: self.shape.rows,
            offset,
            stride,
        })
    }

    /// Returns get.
    pub fn get(self, row: usize, col: usize) -> Result<f32> {
        if row >= self.shape.rows || col >= self.shape.cols {
            return Err(invalid_argument("matrix indices are out of bounds"));
        }
        let index = match self.layout {
            MatrixLayout::RowMajor => row * self.shape.cols + col,
            MatrixLayout::ColumnMajor => col * self.shape.rows + row,
        };
        Ok(self.values[index])
    }

    /// Returns matmul.
    pub fn matmul(self, right: &F32MatrixView<'_>) -> Result<F32Matrix> {
        self.validate()?;
        right.validate()?;
        if self.shape.cols != right.shape.rows {
            return Err(invalid_argument("matrix multiply shapes are incompatible"));
        }
        let shape = MatrixShape::new(self.shape.rows, right.shape.cols)?;
        let mut values = vec![0.0; shape.element_count()?];
        for row in 0..self.shape.rows {
            for col in 0..right.shape.cols {
                let mut acc = 0.0;
                for inner in 0..self.shape.cols {
                    acc += self.get(row, inner)? * right.get(inner, col)?;
                }
                values[row * shape.cols + col] = acc;
            }
        }
        F32Matrix::new(shape, values)
    }

    /// Returns matvec.
    pub fn matvec(self, vector: &[f32]) -> Result<DenseVector> {
        self.validate()?;
        if vector.len() != self.shape.cols {
            return Err(invalid_argument(
                "matrix/vector dimensions are incompatible",
            ));
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("matrix/vector values must be finite"));
        }
        let mut output = vec![0.0; self.shape.rows];
        for (row, output_value) in output.iter_mut().enumerate() {
            *output_value = dot(self.row(row)?.as_slice().as_slice(), vector)?;
        }
        DenseVector::new(output)
    }

    /// Returns row sums.
    pub fn row_sums(self) -> Result<Vec<f32>> {
        (0..self.shape.rows)
            .map(|index| Ok(self.row(index)?.iter().sum()))
            .collect()
    }

    /// Returns column sums.
    pub fn column_sums(self) -> Result<Vec<f32>> {
        (0..self.shape.cols)
            .map(|index| Ok(self.column(index)?.iter().sum()))
            .collect()
    }

    /// Returns l2 normalize rows.
    pub fn l2_normalize_rows(self) -> Result<F32Matrix> {
        let mut values = Vec::with_capacity(self.values.len());
        for row in 0..self.shape.rows {
            let row_view = self.row(row)?;
            let vector = DenseVector::new(row_view.as_slice())?.l2_normalized()?;
            values.extend(vector.into_values());
        }
        F32Matrix::new(self.shape, values)
    }

    /// Returns l2 normalize columns.
    pub fn l2_normalize_columns(self) -> Result<F32Matrix> {
        let mut values = vec![0.0; self.shape.element_count()?];
        let normalized = (0..self.shape.cols)
            .map(|col| DenseVector::new(self.column(col)?.as_slice())?.l2_normalized())
            .collect::<Result<Vec<_>>>()?;
        for row in 0..self.shape.rows {
            for col in 0..self.shape.cols {
                values[row * self.shape.cols + col] = normalized[col].as_slice()[row];
            }
        }
        F32Matrix::new(self.shape, values)
    }

    /// Returns pairwise row dot.
    pub fn pairwise_row_dot(self, right: &F32MatrixView<'_>) -> Result<F32Matrix> {
        if self.shape.cols != right.shape.cols {
            return Err(invalid_argument(
                "row pairwise dot requires equal column counts",
            ));
        }
        let shape = MatrixShape::new(self.shape.rows, right.shape.rows)?;
        let mut values = vec![0.0; shape.element_count()?];
        for row in 0..self.shape.rows {
            let left = self.row(row)?;
            for other_row in 0..right.shape.rows {
                let right_row = right.row(other_row)?;
                values[row * shape.cols + other_row] =
                    dot(left.as_slice().as_slice(), right_row.as_slice().as_slice())?;
            }
        }
        F32Matrix::new(shape, values)
    }

    /// Returns pairwise row cosine.
    pub fn pairwise_row_cosine(self, right: &F32MatrixView<'_>) -> Result<F32Matrix> {
        if self.shape.cols != right.shape.cols {
            return Err(invalid_argument(
                "row pairwise cosine requires equal column counts",
            ));
        }
        let shape = MatrixShape::new(self.shape.rows, right.shape.rows)?;
        let mut values = vec![0.0; shape.element_count()?];
        for row in 0..self.shape.rows {
            let left = self.row(row)?;
            for other_row in 0..right.shape.rows {
                let right_row = right.row(other_row)?;
                values[row * shape.cols + other_row] =
                    cosine_similarity(left.as_slice().as_slice(), right_row.as_slice().as_slice())?;
            }
        }
        F32Matrix::new(shape, values)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        self.shape.validate()?;
        if self.values.len() != self.shape.element_count()? {
            return Err(invalid_argument(format!(
                "matrix shape expects {} values but matrix view has {}",
                self.shape.element_count()?,
                self.values.len()
            )));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("matrix values must be finite"));
        }
        Ok(())
    }

    /// Consumes this value into an owned.
    pub fn into_owned(self) -> Result<F32Matrix> {
        F32Matrix::new(self.shape, self.values.to_vec())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for row view.
pub struct RowView<'a> {
    values: &'a [f32],
    len: usize,
    offset: usize,
    stride: usize,
}

impl<'a> RowView<'a> {
    /// Returns len.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns iter.
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        (0..self.len).map(|index| self.values[self.offset + index * self.stride])
    }

    /// Borrows this value as a slice.
    pub fn as_slice(&self) -> Vec<f32> {
        self.iter().collect()
    }

    /// Converts this value to dense vector.
    pub fn to_dense_vector(&self) -> Result<DenseVector> {
        DenseVector::new(self.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for column view.
pub struct ColumnView<'a> {
    values: &'a [f32],
    len: usize,
    offset: usize,
    stride: usize,
}

impl<'a> ColumnView<'a> {
    /// Returns len.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns iter.
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        (0..self.len).map(|index| self.values[self.offset + index * self.stride])
    }

    /// Borrows this value as a slice.
    pub fn as_slice(&self) -> Vec<f32> {
        self.iter().collect()
    }

    /// Converts this value to dense vector.
    pub fn to_dense_vector(&self) -> Result<DenseVector> {
        DenseVector::new(self.as_slice())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for kernel2d.
pub struct Kernel2d {
    width: usize,
    height: usize,
    values: Vec<f32>,
}

impl Kernel2d {
    /// Creates a new value.
    pub fn new(width: usize, height: usize, values: impl Into<Vec<f32>>) -> Result<Self> {
        let kernel = Self {
            width,
            height,
            values: values.into(),
        };
        kernel.validate()?;
        Ok(kernel)
    }

    /// Returns width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns height.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(invalid_argument(
                "kernel width and height must be greater than zero",
            ));
        }
        if self.values.len()
            != self
                .width
                .checked_mul(self.height)
                .ok_or_else(|| invalid_argument("kernel element count overflowed usize"))?
        {
            return Err(invalid_argument("kernel dimensions do not match values"));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("kernel values must be finite"));
        }
        Ok(())
    }

    /// Returns identity 3x3.
    pub fn identity_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Returns sharpen 3x3.
    pub fn sharpen_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0],
        }
    }

    /// Returns edge 3x3.
    pub fn edge_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0],
        }
    }

    /// Returns blur 3x3.
    pub fn blur_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![1.0; 9],
        }
    }

    /// Borrows this value as an array 3x3.
    pub fn as_array_3x3(&self) -> Result<[f32; 9]> {
        if self.width != 3 || self.height != 3 {
            return Err(invalid_argument("kernel is not 3x3"));
        }
        Ok(self
            .values
            .clone()
            .try_into()
            .expect("kernel length is validated"))
    }
}

impl From<[f32; 9]> for Kernel2d {
    fn from(value: [f32; 9]) -> Self {
        Self {
            width: 3,
            height: 3,
            values: value.to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for kernel1d.
pub struct Kernel1d {
    values: Vec<f32>,
}

impl Kernel1d {
    /// Creates a new value.
    pub fn new(values: impl Into<Vec<f32>>) -> Result<Self> {
        let kernel = Self {
            values: values.into(),
        };
        kernel.validate()?;
        Ok(kernel)
    }

    /// Returns values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.values.is_empty() {
            return Err(invalid_argument("1D kernel must not be empty"));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("1D kernel values must be finite"));
        }
        Ok(())
    }
}

impl TryFrom<&F32Tensor> for F32Matrix {
    type Error = DetectError;

    fn try_from(value: &F32Tensor) -> Result<Self> {
        if value.shape().rank() != 2 {
            return Err(invalid_argument(
                "tensor-to-matrix conversion requires rank 2",
            ));
        }
        let dims = value.shape().dimensions();
        Self::new(MatrixShape::new(dims[0], dims[1])?, value.values().to_vec())
    }
}

impl TryFrom<F32Tensor> for F32Matrix {
    type Error = DetectError;

    fn try_from(value: F32Tensor) -> Result<Self> {
        if value.shape().rank() != 2 {
            return Err(invalid_argument(
                "tensor-to-matrix conversion requires rank 2",
            ));
        }
        let dims = value.shape().dimensions().to_vec();
        Self::new(MatrixShape::new(dims[0], dims[1])?, value.into_values())
    }
}

impl<'a> TryFrom<F32TensorView<'a>> for F32MatrixView<'a> {
    type Error = DetectError;

    fn try_from(value: F32TensorView<'a>) -> Result<Self> {
        if value.shape().rank() != 2 {
            return Err(invalid_argument(
                "tensor view to matrix view conversion requires rank 2",
            ));
        }
        let dims = value.shape().dimensions();
        Self::new(MatrixShape::new(dims[0], dims[1])?, value.values())
    }
}

impl TryFrom<&F32Matrix> for F32Tensor {
    type Error = DetectError;

    fn try_from(value: &F32Matrix) -> Result<Self> {
        F32Tensor::new(
            TensorShape::new([value.shape.rows, value.shape.cols])?,
            value.values.clone(),
        )
    }
}

impl TryFrom<RowView<'_>> for DenseVector {
    type Error = DetectError;

    fn try_from(value: RowView<'_>) -> Result<Self> {
        DenseVector::new(value.as_slice())
    }
}

impl TryFrom<ColumnView<'_>> for DenseVector {
    type Error = DetectError;

    fn try_from(value: ColumnView<'_>) -> Result<Self> {
        DenseVector::new(value.as_slice())
    }
}

impl TryFrom<&DenseVector> for F32Matrix {
    type Error = DetectError;

    fn try_from(value: &DenseVector) -> Result<Self> {
        F32Matrix::new(
            MatrixShape::new(1, value.dimensions())?,
            value.as_slice().to_vec(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_shapes_and_stride_backed_views() {
        assert!(MatrixShape::new(0, 2).is_err());
        let matrix = F32Matrix::from_rows([[1.0, 2.0], [3.0, 4.0]]).unwrap();
        assert_eq!(
            matrix.transpose_view().row(0).unwrap().as_slice(),
            vec![1.0, 3.0]
        );
    }

    #[test]
    fn matrix_multiply_matches_expected_values() {
        let left = F32Matrix::from_rows([[1.0, 2.0], [3.0, 4.0]]).unwrap();
        let right = F32Matrix::from_rows([[2.0, 0.0], [1.0, 2.0]]).unwrap();
        let product = left.matmul(&right.as_view()).unwrap();
        assert_eq!(product.values(), &[4.0, 4.0, 10.0, 8.0]);
    }

    #[test]
    fn pairwise_row_cosine_and_kernel_helpers_work() {
        let matrix = F32Matrix::from_rows([[1.0, 0.0], [0.0, 1.0]]).unwrap();
        let cosine = matrix.pairwise_row_cosine(&matrix.as_view()).unwrap();
        assert_eq!(cosine.values(), &[1.0, 0.0, 0.0, 1.0]);
        assert_eq!(Kernel2d::sharpen_3x3().as_array_3x3().unwrap()[4], 5.0);
    }

    #[test]
    fn tensor_and_vector_bridges_round_trip() {
        let tensor = F32Tensor::from_dims([2, 2], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let matrix = F32Matrix::try_from(&tensor).unwrap();
        assert_eq!(
            matrix.row(1).unwrap().to_dense_vector().unwrap().as_slice(),
            &[3.0, 4.0]
        );
        let tensor_round_trip = F32Tensor::try_from(&matrix).unwrap();
        assert_eq!(tensor_round_trip.values(), tensor.values());
    }
}
