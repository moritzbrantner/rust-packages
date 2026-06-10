# Analytical Math SVD and f64 Matrix Defaults

## Status

Accepted.

## Context

The Analytical Math Crates need stronger dense matrix tooling for statistics,
finance, package surfaces, and app workflows. Existing `F32Matrix` callers must
remain stable, but SVD, PCA, pseudoinverse, and matrix-statistics workflows need
f64 precision by default.

## Decision

Add `F64Matrix` and `F64MatrixView` alongside `F32Matrix`.

Use a pure Rust real-valued SVD implementation as the default path for
`math-linear` SVD, pseudoinverse, and numerical-rank operations.

Keep `faer` and `nalgebra` behind hidden feature-gated reference/benchmark
paths. They are not public runtime choices and are not selected by package
surface requests.

Evolve matrix-statistics package contracts to default to f64 for normalization,
covariance, PCA, OLS, and OLS diagnostics. Keep explicit `precision: "f32"` as
a compatibility request path.

Do not add a separate numerical backend layer.

## Consequences

`F32Matrix` remains available for existing library consumers.

SVD-class package operations use one operation ID each with a precision field,
default to f64, cap package-surface dimensions, and return compact factor output
unless callers opt into thin factors.

PCA now uses centered-data SVD in package workflows, and OLS can use
pseudoinverse for rank-deficient designs. OLS diagnostics stay strict and
require identifiable full-rank designs.
