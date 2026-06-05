#![doc = include_str!("../README.md")]

pub mod surface;
use vector_analysis_core::DenseVector;
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing sparse similarity metric.
pub enum SparseSimilarityMetric {
    /// The dot variant.
    Dot,
    /// The cosine variant.
    Cosine,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for sparse vector.
pub struct SparseVector {
    dimensions: usize,
    indices: Vec<usize>,
    values: Vec<f32>,
}

impl SparseVector {
    /// Creates a new value.
    pub fn new(dimensions: usize, indices: Vec<usize>, values: Vec<f32>) -> Result<Self> {
        let vector = Self {
            dimensions,
            indices,
            values,
        };
        vector.validate()?;
        Ok(vector)
    }

    /// Returns dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Returns indices.
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }

    /// Returns values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Returns nnz.
    pub fn nnz(&self) -> usize {
        self.indices.len()
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.dimensions == 0 {
            return Err(invalid_argument(
                "sparse vector dimensions must be greater than zero",
            ));
        }
        if self.indices.len() != self.values.len() {
            return Err(invalid_argument(
                "sparse vector indices and values must have the same length",
            ));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("sparse vector values must be finite"));
        }
        if self.indices.iter().any(|index| *index >= self.dimensions) {
            return Err(invalid_argument("sparse vector index is out of bounds"));
        }
        Ok(())
    }

    /// Returns canonicalized.
    pub fn canonicalized(&self) -> Result<Self> {
        self.validate()?;
        let mut pairs = self
            .indices
            .iter()
            .copied()
            .zip(self.values.iter().copied())
            .collect::<Vec<_>>();
        pairs.sort_by_key(|(index, _)| *index);
        let mut indices = Vec::new();
        let mut values = Vec::new();
        for (index, value) in pairs {
            if let Some(last) = indices.last().copied() {
                if last == index {
                    if let Some(last_value) = values.last_mut() {
                        *last_value += value;
                    }
                    continue;
                }
            }
            if value != 0.0 {
                indices.push(index);
                values.push(value);
            }
        }
        Self::new(self.dimensions, indices, values)
    }

    /// Returns dot.
    pub fn dot(&self, other: &Self) -> Result<f32> {
        let left = self.canonicalized()?;
        let right = other.canonicalized()?;
        if left.dimensions != right.dimensions {
            return Err(invalid_argument("sparse vector dimensions must match"));
        }
        let mut i = 0;
        let mut j = 0;
        let mut acc = 0.0;
        while i < left.indices.len() && j < right.indices.len() {
            match left.indices[i].cmp(&right.indices[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    acc += left.values[i] * right.values[j];
                    i += 1;
                    j += 1;
                }
            }
        }
        Ok(acc)
    }

    /// Returns cosine similarity.
    pub fn cosine_similarity(&self, other: &Self) -> Result<f32> {
        let left_norm = self
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        let right_norm = other
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
            return Err(invalid_argument(
                "cosine similarity requires non-zero sparse vectors",
            ));
        }
        Ok(self.dot(other)? / (left_norm * right_norm))
    }

    /// Returns the L1 norm.
    pub fn l1_norm(&self) -> Result<f32> {
        self.validate()?;
        Ok(self.values.iter().map(|value| value.abs()).sum())
    }

    /// Returns the L2 norm.
    pub fn l2_norm(&self) -> Result<f32> {
        self.validate()?;
        Ok(self
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt())
    }

    /// Scales all sparse values by a finite factor.
    pub fn scale(&self, factor: f32) -> Result<Self> {
        self.validate()?;
        if !factor.is_finite() {
            return Err(invalid_argument(
                "sparse vector scale factor must be finite",
            ));
        }
        Self::new(
            self.dimensions,
            self.indices.clone(),
            self.values.iter().map(|value| value * factor).collect(),
        )?
        .canonicalized()
    }

    /// Adds two sparse vectors with matching dimensions.
    pub fn add(&self, other: &Self) -> Result<Self> {
        let left = self.canonicalized()?;
        let right = other.canonicalized()?;
        if left.dimensions != right.dimensions {
            return Err(invalid_argument("sparse vector dimensions must match"));
        }
        let mut indices = Vec::new();
        let mut values = Vec::new();
        let mut left_index = 0;
        let mut right_index = 0;
        while left_index < left.indices.len() || right_index < right.indices.len() {
            match (
                left.indices.get(left_index).copied(),
                right.indices.get(right_index).copied(),
            ) {
                (Some(left_col), Some(right_col)) if left_col == right_col => {
                    let value = left.values[left_index] + right.values[right_index];
                    if value != 0.0 {
                        indices.push(left_col);
                        values.push(value);
                    }
                    left_index += 1;
                    right_index += 1;
                }
                (Some(left_col), Some(right_col)) if left_col < right_col => {
                    indices.push(left_col);
                    values.push(left.values[left_index]);
                    left_index += 1;
                }
                (Some(_), Some(right_col)) => {
                    indices.push(right_col);
                    values.push(right.values[right_index]);
                    right_index += 1;
                }
                (Some(left_col), None) => {
                    indices.push(left_col);
                    values.push(left.values[left_index]);
                    left_index += 1;
                }
                (None, Some(right_col)) => {
                    indices.push(right_col);
                    values.push(right.values[right_index]);
                    right_index += 1;
                }
                (None, None) => break,
            }
        }
        Self::new(left.dimensions, indices, values)
    }

    /// Returns the top `k` entries sorted by descending absolute value, then index.
    pub fn top_k_by_abs(&self, k: usize) -> Result<Vec<(usize, f32)>> {
        let canonical = self.canonicalized()?;
        let mut pairs = canonical
            .indices
            .into_iter()
            .zip(canonical.values)
            .collect::<Vec<_>>();
        pairs.sort_by(|left, right| {
            right
                .1
                .abs()
                .partial_cmp(&left.1.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        pairs.truncate(k);
        Ok(pairs)
    }

    /// Returns normalize l2.
    pub fn normalize_l2(&self) -> Result<Self> {
        let norm = self
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        if norm <= f32::EPSILON {
            return Err(invalid_argument(
                "sparse vector norm must be greater than zero",
            ));
        }
        Self::new(
            self.dimensions,
            self.indices.clone(),
            self.values.iter().map(|value| value / norm).collect(),
        )
    }

    /// Converts this value to dense.
    pub fn to_dense(&self) -> Vec<f32> {
        let mut dense = vec![0.0; self.dimensions];
        for (&index, &value) in self.indices.iter().zip(&self.values) {
            dense[index] = value;
        }
        dense
    }

    /// Builds this value from dense.
    pub fn from_dense(values: &[f32]) -> Result<Self> {
        if values.is_empty() {
            return Err(invalid_argument("dense vector must not be empty"));
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("dense vector values must be finite"));
        }
        let mut indices = Vec::new();
        let mut sparse_values = Vec::new();
        for (index, value) in values.iter().copied().enumerate() {
            if value != 0.0 {
                indices.push(index);
                sparse_values.push(value);
            }
        }
        Self::new(values.len(), indices, sparse_values)
    }
}

impl TryFrom<&DenseVector> for SparseVector {
    type Error = DetectError;

    fn try_from(value: &DenseVector) -> Result<Self> {
        Self::from_dense(value.as_slice())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for coo matrix.
pub struct CooMatrix {
    rows: usize,
    cols: usize,
    entries: Vec<(usize, usize, f32)>,
}

impl CooMatrix {
    /// Creates a new value.
    pub fn new(rows: usize, cols: usize, entries: Vec<(usize, usize, f32)>) -> Result<Self> {
        let matrix = Self {
            rows,
            cols,
            entries,
        };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Returns rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns cols.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns entries.
    pub fn entries(&self) -> &[(usize, usize, f32)] {
        &self.entries
    }

    /// Returns nnz.
    pub fn nnz(&self) -> usize {
        self.entries.len()
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(invalid_argument(
                "COO matrix rows and cols must be greater than zero",
            ));
        }
        for &(row, col, value) in &self.entries {
            if row >= self.rows || col >= self.cols {
                return Err(invalid_argument("COO entry index is out of bounds"));
            }
            if !value.is_finite() {
                return Err(invalid_argument("COO entry values must be finite"));
            }
        }
        Ok(())
    }

    /// Returns canonicalized.
    pub fn canonicalized(&self) -> Result<Self> {
        self.validate()?;
        let mut entries = self.entries.clone();
        entries.sort_by_key(|(row, col, _)| (*row, *col));
        let mut output = Vec::new();
        for (row, col, value) in entries {
            if let Some((last_row, last_col, last_value)) = output.last_mut() {
                if *last_row == row && *last_col == col {
                    *last_value += value;
                    continue;
                }
            }
            if value != 0.0 {
                output.push((row, col, value));
            }
        }
        Self::new(self.rows, self.cols, output)
    }

    /// Converts this value to csr.
    pub fn to_csr(&self) -> Result<CsrMatrix> {
        CsrMatrix::from_coo(self)
    }

    /// Returns a transposed COO matrix.
    pub fn transpose(&self) -> Result<Self> {
        self.validate()?;
        Self::new(
            self.cols,
            self.rows,
            self.entries
                .iter()
                .map(|(row, col, value)| (*col, *row, *value))
                .collect(),
        )
        .and_then(|matrix| matrix.canonicalized())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for csr matrix.
pub struct CsrMatrix {
    rows: usize,
    cols: usize,
    row_offsets: Vec<usize>,
    column_indices: Vec<usize>,
    values: Vec<f32>,
}

impl CsrMatrix {
    /// Creates a new value.
    pub fn new(
        rows: usize,
        cols: usize,
        row_offsets: Vec<usize>,
        column_indices: Vec<usize>,
        values: Vec<f32>,
    ) -> Result<Self> {
        let matrix = Self {
            rows,
            cols,
            row_offsets,
            column_indices,
            values,
        };
        matrix.validate()?;
        Ok(matrix)
    }

    /// Builds this value from coo.
    pub fn from_coo(coo: &CooMatrix) -> Result<Self> {
        let canonical = coo.canonicalized()?;
        let mut row_offsets = vec![0usize; canonical.rows + 1];
        let mut column_indices = Vec::with_capacity(canonical.entries.len());
        let mut values = Vec::with_capacity(canonical.entries.len());
        let mut current_row = 0usize;
        for (row, col, value) in canonical.entries {
            while current_row < row {
                row_offsets[current_row + 1] = column_indices.len();
                current_row += 1;
            }
            column_indices.push(col);
            values.push(value);
        }
        while current_row < canonical.rows {
            row_offsets[current_row + 1] = column_indices.len();
            current_row += 1;
        }
        Self::new(
            canonical.rows,
            canonical.cols,
            row_offsets,
            column_indices,
            values,
        )
    }

    /// Returns rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns cols.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns row.
    pub fn row(&self, index: usize) -> Result<SparseRow<'_>> {
        if index >= self.rows {
            return Err(invalid_argument("CSR row index is out of bounds"));
        }
        let start = self.row_offsets[index];
        let end = self.row_offsets[index + 1];
        Ok(SparseRow {
            cols: self.cols,
            indices: &self.column_indices[start..end],
            values: &self.values[start..end],
        })
    }

    /// Returns rows iter.
    pub fn rows_iter(&self) -> impl Iterator<Item = SparseRow<'_>> {
        (0..self.rows).map(|index| self.row(index).expect("indices are validated"))
    }

    /// Returns the non-zero count for each row.
    pub fn row_nnz(&self) -> Vec<usize> {
        self.row_offsets
            .windows(2)
            .map(|window| window[1] - window[0])
            .collect()
    }

    /// Multiplies this CSR matrix by a dense finite vector.
    pub fn mul_dense_vector(&self, vector: &[f32]) -> Result<Vec<f32>> {
        self.validate()?;
        if vector.len() != self.cols {
            return Err(invalid_argument(
                "sparse matrix/vector dimensions are incompatible",
            ));
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("dense vector values must be finite"));
        }
        let mut output = vec![0.0; self.rows];
        for (row_index, row) in self.rows_iter().enumerate() {
            output[row_index] = row
                .indices()
                .iter()
                .zip(row.values())
                .map(|(col, value)| vector[*col] * value)
                .sum();
        }
        Ok(output)
    }

    /// Converts this CSR matrix to COO entries.
    pub fn to_coo(&self) -> Result<CooMatrix> {
        self.validate()?;
        let mut entries = Vec::with_capacity(self.values.len());
        for row in 0..self.rows {
            for index in self.row_offsets[row]..self.row_offsets[row + 1] {
                entries.push((row, self.column_indices[index], self.values[index]));
            }
        }
        CooMatrix::new(self.rows, self.cols, entries)
    }

    /// Returns the matrix transpose.
    pub fn transpose(&self) -> Result<Self> {
        self.to_coo()?.transpose()?.to_csr()
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(invalid_argument(
                "CSR matrix rows and cols must be greater than zero",
            ));
        }
        if self.row_offsets.len() != self.rows + 1 {
            return Err(invalid_argument(
                "CSR row_offsets length must equal rows + 1",
            ));
        }
        if self.column_indices.len() != self.values.len() {
            return Err(invalid_argument(
                "CSR column_indices and values must have the same length",
            ));
        }
        if self.row_offsets.first().copied().unwrap_or_default() != 0 {
            return Err(invalid_argument("CSR row_offsets must start at zero"));
        }
        if *self.row_offsets.last().unwrap_or(&0) != self.values.len() {
            return Err(invalid_argument("CSR row_offsets must end at nnz"));
        }
        for window in self.row_offsets.windows(2) {
            if window[0] > window[1] {
                return Err(invalid_argument("CSR row_offsets must be non-decreasing"));
            }
        }
        if self.column_indices.iter().any(|index| *index >= self.cols) {
            return Err(invalid_argument("CSR column index is out of bounds"));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("CSR values must be finite"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for sparse row.
pub struct SparseRow<'a> {
    cols: usize,
    indices: &'a [usize],
    values: &'a [f32],
}

impl<'a> SparseRow<'a> {
    /// Returns cols.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns indices.
    pub fn indices(&self) -> &'a [usize] {
        self.indices
    }

    /// Returns values.
    pub fn values(&self) -> &'a [f32] {
        self.values
    }

    /// Converts this value to sparse vector.
    pub fn to_sparse_vector(&self) -> Result<SparseVector> {
        SparseVector::new(self.cols, self.indices.to_vec(), self.values.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_vector_canonicalization_and_similarity_work() {
        let vector = SparseVector::new(4, vec![3, 1, 3], vec![2.0, 1.0, 1.0])
            .unwrap()
            .canonicalized()
            .unwrap();
        assert_eq!(vector.indices(), &[1, 3]);
        assert_eq!(vector.values(), &[1.0, 3.0]);
        assert_eq!(vector.dot(&vector).unwrap(), 10.0);
        assert!((vector.cosine_similarity(&vector).unwrap() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn csr_and_coo_invariants_hold() {
        let coo = CooMatrix::new(2, 3, vec![(1, 2, 2.0), (0, 0, 1.0), (1, 2, 1.0)]).unwrap();
        let csr = coo.to_csr().unwrap();
        assert_eq!(csr.row(0).unwrap().indices(), &[0]);
        assert_eq!(csr.row(1).unwrap().values(), &[3.0]);
    }

    #[test]
    fn dense_sparse_round_trip_preserves_values() {
        let dense = [0.0, 1.0, 0.0, 2.0];
        let sparse = SparseVector::from_dense(&dense).unwrap();
        assert_eq!(sparse.to_dense(), dense);
    }

    #[test]
    fn vector_ops_and_sparse_matrix_transpose_work() {
        let left = SparseVector::new(4, vec![0, 2], vec![1.0, -3.0]).unwrap();
        let right = SparseVector::new(4, vec![2, 3], vec![1.0, 2.0]).unwrap();
        let added = left.add(&right).unwrap();
        assert_eq!(added.indices(), &[0, 2, 3]);
        assert_eq!(added.values(), &[1.0, -2.0, 2.0]);
        assert_eq!(left.top_k_by_abs(1).unwrap(), vec![(2, -3.0)]);

        let matrix = CooMatrix::new(2, 3, vec![(0, 1, 2.0), (1, 2, 3.0)])
            .unwrap()
            .to_csr()
            .unwrap();
        assert_eq!(matrix.row_nnz(), vec![1, 1]);
        assert_eq!(
            matrix.mul_dense_vector(&[1.0, 2.0, 3.0]).unwrap(),
            vec![4.0, 9.0]
        );
        let transposed = matrix.transpose().unwrap();
        assert_eq!(transposed.rows(), 3);
        assert_eq!(transposed.cols(), 2);
        assert_eq!(
            transposed.transpose().unwrap().to_coo().unwrap().entries(),
            matrix.to_coo().unwrap().entries()
        );
    }
}
