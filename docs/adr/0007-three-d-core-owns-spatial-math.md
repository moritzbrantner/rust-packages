# Three-D Core Owns Spatial Math

Status: accepted

`three-d-processing-core` owns workspace 3D vectors, points, rotations, transforms, pinhole camera geometry, and coordinate-convention conversions because those concepts carry 3D semantic conventions that generic linear algebra should not own. `math-linear` remains the dense matrix and decomposition crate; `nalgebra` may be used internally or as a reference while public 3D APIs stay owned by this workspace.
