#![doc = include_str!("../README.md")]

pub mod surface;
use tensor_data::{F32Tensor, F32TensorView, TensorShape};
use vector_analysis_core::{cosine_similarity, dot, DenseVector};
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Checked dense matrix dimensions.
pub struct MatrixShape {
    /// Number of matrix rows.
    pub rows: usize,
    /// Number of matrix columns.
    pub cols: usize,
}

impl MatrixShape {
    /// Creates a shape with non-zero rows and columns.
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        let shape = Self { rows, cols };
        shape.validate()?;
        Ok(shape)
    }

    /// Verifies non-zero dimensions and element-count overflow safety.
    pub fn validate(self) -> Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(invalid_argument(
                "matrix rows and cols must be greater than zero",
            ));
        }
        let _ = self.element_count()?;
        Ok(())
    }

    /// Multiplies rows by columns and fails on `usize` overflow.
    pub fn element_count(self) -> Result<usize> {
        self.rows
            .checked_mul(self.cols)
            .ok_or_else(|| invalid_argument("matrix element count overflowed usize"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Memory interpretation for a matrix or matrix view.
pub enum MatrixLayout {
    /// Contiguous rows, where adjacent values advance across columns.
    RowMajor,
    /// Contiguous columns, used by transpose views without copying.
    ColumnMajor,
}

#[derive(Debug, Clone, PartialEq)]
/// Owned finite `f32` matrix stored in row-major order.
pub struct F32Matrix {
    shape: MatrixShape,
    layout: MatrixLayout,
    values: Vec<f32>,
}

impl F32Matrix {
    /// Creates a row-major matrix after shape and finite-value validation.
    pub fn new(shape: MatrixShape, values: Vec<f32>) -> Result<Self> {
        let matrix = Self {
            shape,
            layout: MatrixLayout::RowMajor,
            values,
        };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Builds a matrix from compile-time-sized row arrays.
    pub fn from_rows<const R: usize, const C: usize>(rows: [[f32; C]; R]) -> Result<Self> {
        let mut values = Vec::with_capacity(R * C);
        for row in rows {
            values.extend(row);
        }
        Self::new(MatrixShape::new(R, C)?, values)
    }

    /// Returns the checked row and column dimensions.
    pub fn shape(&self) -> MatrixShape {
        self.shape
    }

    /// Returns the matrix storage layout.
    pub fn layout(&self) -> MatrixLayout {
        self.layout
    }

    /// Borrows the contiguous matrix values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Consumes the matrix and returns its contiguous values.
    pub fn into_values(self) -> Vec<f32> {
        self.values
    }

    /// Borrows this matrix as a view without copying values.
    pub fn as_view(&self) -> F32MatrixView<'_> {
        F32MatrixView {
            shape: self.shape,
            layout: self.layout,
            values: &self.values,
        }
    }

    /// Borrows one row, respecting the current matrix layout.
    pub fn row(&self, index: usize) -> Result<RowView<'_>> {
        self.as_view().row(index)
    }

    /// Borrows one column, respecting the current matrix layout.
    pub fn column(&self, index: usize) -> Result<ColumnView<'_>> {
        self.as_view().column(index)
    }

    /// Multiplies this matrix by `right`.
    pub fn matmul(&self, right: &F32MatrixView<'_>) -> Result<Self> {
        self.as_view().matmul(right)
    }

    /// Multiplies this matrix by a finite dense vector.
    pub fn matvec(&self, vector: &[f32]) -> Result<DenseVector> {
        self.as_view().matvec(vector)
    }

    /// Creates a transposed view without copying values.
    pub fn transpose_view(&self) -> F32MatrixView<'_> {
        self.as_view().transpose()
    }

    /// Returns a row-major matrix whose rows have unit L2 norm.
    pub fn l2_normalize_rows(&self) -> Result<Self> {
        self.as_view().l2_normalize_rows()
    }

    /// Returns a row-major matrix whose columns have unit L2 norm.
    pub fn l2_normalize_columns(&self) -> Result<Self> {
        self.as_view().l2_normalize_columns()
    }

    /// Computes all pairwise row cosine similarities against `right`.
    pub fn pairwise_row_cosine(&self, right: &F32MatrixView<'_>) -> Result<Self> {
        self.as_view().pairwise_row_cosine(right)
    }

    /// Verifies shape/value count agreement and rejects non-finite values.
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
/// Borrowed finite `f32` matrix values with shape and layout metadata.
pub struct F32MatrixView<'a> {
    shape: MatrixShape,
    layout: MatrixLayout,
    values: &'a [f32],
}

impl<'a> F32MatrixView<'a> {
    /// Creates a row-major matrix view after validating shape and values.
    pub fn new(shape: MatrixShape, values: &'a [f32]) -> Result<Self> {
        let view = Self {
            shape,
            layout: MatrixLayout::RowMajor,
            values,
        };
        view.validate()?;
        Ok(view)
    }

    /// Returns the checked row and column dimensions.
    pub fn shape(&self) -> MatrixShape {
        self.shape
    }

    /// Returns how the borrowed value slice is interpreted.
    pub fn layout(&self) -> MatrixLayout {
        self.layout
    }

    /// Borrows the underlying contiguous values.
    pub fn values(&self) -> &'a [f32] {
        self.values
    }

    /// Creates a transposed view by swapping dimensions and layout metadata.
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

    /// Borrows one logical row from this view.
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

    /// Borrows one logical column from this view.
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

    /// Reads one value by logical row and column.
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

    /// Multiplies this view by another matrix view.
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

    /// Multiplies this view by a finite dense vector.
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

    /// Sums each logical row.
    pub fn row_sums(self) -> Result<Vec<f32>> {
        (0..self.shape.rows)
            .map(|index| Ok(self.row(index)?.iter().sum()))
            .collect()
    }

    /// Sums each logical column.
    pub fn column_sums(self) -> Result<Vec<f32>> {
        (0..self.shape.cols)
            .map(|index| Ok(self.column(index)?.iter().sum()))
            .collect()
    }

    /// Returns a row-major matrix whose rows have unit L2 norm.
    pub fn l2_normalize_rows(self) -> Result<F32Matrix> {
        let mut values = Vec::with_capacity(self.values.len());
        for row in 0..self.shape.rows {
            let row_view = self.row(row)?;
            let vector = DenseVector::new(row_view.as_slice())?.l2_normalized()?;
            values.extend(vector.into_values());
        }
        F32Matrix::new(self.shape, values)
    }

    /// Returns a row-major matrix whose columns have unit L2 norm.
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

    /// Computes the dot product for every pair of rows in two matrices.
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

    /// Computes cosine similarity for every pair of rows in two matrices.
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

    /// Verifies shape/value count agreement and rejects non-finite values.
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

    /// Copies this view into an owned row-major matrix.
    pub fn into_owned(self) -> Result<F32Matrix> {
        F32Matrix::new(self.shape, self.values.to_vec())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Strided borrowed view over one logical matrix row.
pub struct RowView<'a> {
    values: &'a [f32],
    len: usize,
    offset: usize,
    stride: usize,
}

impl<'a> RowView<'a> {
    /// Returns the number of values in the row.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the row has no values.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterates over row values in logical column order.
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        (0..self.len).map(|index| self.values[self.offset + index * self.stride])
    }

    /// Collects the possibly strided row into a contiguous vector.
    pub fn as_slice(&self) -> Vec<f32> {
        self.iter().collect()
    }

    /// Copies the row into a validated dense vector.
    pub fn to_dense_vector(&self) -> Result<DenseVector> {
        DenseVector::new(self.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Strided borrowed view over one logical matrix column.
pub struct ColumnView<'a> {
    values: &'a [f32],
    len: usize,
    offset: usize,
    stride: usize,
}

impl<'a> ColumnView<'a> {
    /// Returns the number of values in the column.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the column has no values.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterates over column values in logical row order.
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        (0..self.len).map(|index| self.values[self.offset + index * self.stride])
    }

    /// Collects the possibly strided column into a contiguous vector.
    pub fn as_slice(&self) -> Vec<f32> {
        self.iter().collect()
    }

    /// Copies the column into a validated dense vector.
    pub fn to_dense_vector(&self) -> Result<DenseVector> {
        DenseVector::new(self.as_slice())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Finite 2D convolution kernel stored in row-major order.
pub struct Kernel2d {
    width: usize,
    height: usize,
    values: Vec<f32>,
}

impl Kernel2d {
    /// Creates a kernel with non-zero dimensions and matching finite values.
    pub fn new(width: usize, height: usize, values: impl Into<Vec<f32>>) -> Result<Self> {
        let kernel = Self {
            width,
            height,
            values: values.into(),
        };
        kernel.validate()?;
        Ok(kernel)
    }

    /// Returns the number of columns in the kernel.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the number of rows in the kernel.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Borrows row-major kernel values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Verifies dimensions, value count, and finite values.
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

    /// Creates a 3x3 identity kernel with a `1.0` center coefficient.
    pub fn identity_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Creates a standard 3x3 sharpen kernel.
    pub fn sharpen_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0],
        }
    }

    /// Creates a standard 3x3 edge-detection kernel.
    pub fn edge_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![-1.0, -1.0, -1.0, -1.0, 8.0, -1.0, -1.0, -1.0, -1.0],
        }
    }

    /// Creates an unnormalized 3x3 box blur kernel.
    pub fn blur_3x3() -> Self {
        Self {
            width: 3,
            height: 3,
            values: vec![1.0; 9],
        }
    }

    /// Copies a 3x3 kernel into a fixed-size row-major array.
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
/// Finite 1D convolution kernel.
pub struct Kernel1d {
    values: Vec<f32>,
}

impl Kernel1d {
    /// Creates a non-empty kernel and rejects non-finite values.
    pub fn new(values: impl Into<Vec<f32>>) -> Result<Self> {
        let kernel = Self {
            values: values.into(),
        };
        kernel.validate()?;
        Ok(kernel)
    }

    /// Borrows kernel coefficients in storage order.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Verifies that the kernel is non-empty and finite.
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
