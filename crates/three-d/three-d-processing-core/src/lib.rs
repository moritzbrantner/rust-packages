#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::ops::{Add, AddAssign, Div, Mul, Sub};

use serde::{Deserialize, Serialize};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
/// Data type for vector3.
pub struct Vector3 {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
    /// The z value.
    pub z: f32,
}

impl Vector3 {
    /// Constant for zero.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// Creates a new value.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Returns whether this value is finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Returns dot.
    pub fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(rhs.x, self.y.mul_add(rhs.y, self.z * rhs.z))
    }

    /// Returns cross.
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y.mul_add(rhs.z, -(self.z * rhs.y)),
            self.z.mul_add(rhs.x, -(self.x * rhs.z)),
            self.x.mul_add(rhs.y, -(self.y * rhs.x)),
        )
    }

    /// Returns length squared.
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Returns length.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns distance.
    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
    }

    /// Normalizes this value.
    pub fn normalize(self) -> Result<Self> {
        validate_finite_vector(self, "vector")?;
        let length = self.length();
        if length <= f32::EPSILON {
            return Err(invalid_argument("vector length must be greater than zero"));
        }
        Ok(self / length)
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vector3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vector3> for f32 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        rhs * self
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
/// Data type for point3.
pub struct Point3 {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
    /// The z value.
    pub z: f32,
}

impl Point3 {
    /// Creates a new value.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Returns whether this value is finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Returns distance.
    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
    }

    /// Returns midpoint.
    pub fn midpoint(self, rhs: Self) -> Self {
        self + ((rhs - self) * 0.5)
    }
}

impl Add<Vector3> for Point3 {
    type Output = Self;

    fn add(self, rhs: Vector3) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub<Vector3> for Point3 {
    type Output = Self;

    fn sub(self, rhs: Vector3) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Sub<Point3> for Point3 {
    type Output = Vector3;

    fn sub(self, rhs: Point3) -> Self::Output {
        Vector3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for bounds3.
pub struct Bounds3 {
    /// The min value.
    pub min: Point3,
    /// The max value.
    pub max: Point3,
}

impl Bounds3 {
    /// Creates a new value.
    pub fn new(min: Point3, max: Point3) -> Result<Self> {
        let bounds = Self { min, max };
        bounds.validate()?;
        Ok(bounds)
    }

    /// Builds this value from points.
    pub fn from_points(points: &[Point3]) -> Result<Option<Self>> {
        validate_points(points)?;
        let Some(first) = points.first().copied() else {
            return Ok(None);
        };
        let mut min = first;
        let mut max = first;
        for point in points.iter().copied().skip(1) {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
        Ok(Some(Self { min, max }))
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        validate_points(&[self.min, self.max])?;
        if self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z {
            return Err(invalid_argument("bounds min must not exceed max"));
        }
        Ok(())
    }

    /// Returns size.
    pub fn size(self) -> Vector3 {
        self.max - self.min
    }

    /// Returns center.
    pub fn center(self) -> Point3 {
        self.min + (self.size() * 0.5)
    }

    /// Returns whether this value contains the point.
    pub fn contains_point(self, point: Point3) -> Result<bool> {
        self.validate()?;
        validate_points(&[point])?;
        Ok(point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z)
    }

    /// Returns whether this value intersects another bounds.
    pub fn intersects(self, other: Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
            && self.min.z < other.max.z
            && other.min.z < self.max.z)
    }

    /// Returns intersection.
    pub fn intersection(self, other: Self) -> Result<Option<Self>> {
        if !self.intersects(other)? {
            return Ok(None);
        }
        Self::new(
            Point3::new(
                self.min.x.max(other.min.x),
                self.min.y.max(other.min.y),
                self.min.z.max(other.min.z),
            ),
            Point3::new(
                self.max.x.min(other.max.x),
                self.max.y.min(other.max.y),
                self.max.z.min(other.max.z),
            ),
        )
        .map(Some)
    }

    /// Returns union.
    pub fn union(self, other: Self) -> Result<Self> {
        self.validate()?;
        other.validate()?;
        Self::new(
            Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }

    /// Returns volume.
    pub fn volume(self) -> Result<f32> {
        self.validate()?;
        let size = self.size();
        Ok(size.x * size.y * size.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Data type for a broad-phase collision pair.
pub struct CollisionPair {
    /// Index of the first item.
    pub left_index: usize,
    /// Index of the second item.
    pub right_index: usize,
}

impl CollisionPair {
    /// Creates a new ordered same-set pair.
    pub fn ordered(left_index: usize, right_index: usize) -> Self {
        if left_index <= right_index {
            Self {
                left_index,
                right_index,
            }
        } else {
            Self {
                left_index: right_index,
                right_index: left_index,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Strategy used for broad-phase 3D collision detection.
pub enum BroadPhase3Strategy {
    /// Selects an implementation from the input shape.
    Auto,
    /// Checks every pair.
    BruteForce,
    /// Uses a spatial hash grid.
    SpatialHashGrid,
    /// Uses sweep and prune along the x axis.
    SweepAndPrune,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Cell size selection for 3D spatial hashing.
pub enum SpatialCellSize3 {
    /// Uses median item extent.
    Auto,
    /// Uses a fixed cubic cell size.
    Fixed {
        /// Cell edge length.
        size: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Options for 3D broad-phase collision detection.
pub struct BroadPhase3Config {
    /// Strategy to use.
    pub strategy: BroadPhase3Strategy,
    /// Maximum item count handled with brute force in auto mode.
    pub brute_force_threshold: usize,
    /// Maximum cells a single item may span before auto mode uses sweep and prune.
    pub max_cells_per_item: usize,
    /// Spatial hash grid cell size.
    pub cell_size: SpatialCellSize3,
}

impl Default for BroadPhase3Config {
    fn default() -> Self {
        Self {
            strategy: BroadPhase3Strategy::Auto,
            brute_force_threshold: 128,
            max_cells_per_item: 1024,
            cell_size: SpatialCellSize3::Auto,
        }
    }
}

impl BroadPhase3Config {
    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.max_cells_per_item == 0 {
            return Err(invalid_argument(
                "max_cells_per_item must be greater than zero",
            ));
        }
        if let SpatialCellSize3::Fixed { size } = self.cell_size {
            if !size.is_finite() || size <= 0.0 {
                return Err(invalid_argument(
                    "fixed spatial cell size must be finite and greater than zero",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Runtime statistics for broad-phase collision detection.
pub struct BroadPhaseStats {
    /// Number of objects indexed.
    pub object_count: usize,
    /// Number of occupied cells.
    pub cell_count: usize,
    /// Number of object-cell entries.
    pub cell_entry_count: usize,
    /// Number of candidate pairs emitted.
    pub candidate_pair_count: usize,
    /// Strategy selected after auto resolution.
    pub selected_strategy: BroadPhase3Strategy,
}

impl Default for BroadPhaseStats {
    fn default() -> Self {
        Self {
            object_count: 0,
            cell_count: 0,
            cell_entry_count: 0,
            candidate_pair_count: 0,
            selected_strategy: BroadPhase3Strategy::BruteForce,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GridCell3 {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GridEntry3 {
    cell: GridCell3,
    set: u8,
    index: usize,
}

#[derive(Debug, Clone)]
/// Reusable spatial hash grid for 3D broad-phase collision detection.
pub struct SpatialHashGrid3 {
    config: BroadPhase3Config,
    cell_size: f32,
    left_bounds: Vec<Bounds3>,
    right_bounds: Vec<Bounds3>,
    entries: Vec<GridEntry3>,
    pairs: Vec<CollisionPair>,
    stats: BroadPhaseStats,
}

impl SpatialHashGrid3 {
    /// Creates a new value.
    pub fn new(config: BroadPhase3Config) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            cell_size: f32::EPSILON,
            left_bounds: Vec::new(),
            right_bounds: Vec::new(),
            entries: Vec::new(),
            pairs: Vec::new(),
            stats: BroadPhaseStats {
                selected_strategy: config.strategy,
                ..BroadPhaseStats::default()
            },
        })
    }

    /// Rebuilds this grid for a single set of bounds.
    pub fn rebuild(&mut self, bounds: &[Bounds3]) -> Result<()> {
        validate_bounds3(bounds)?;
        self.cell_size = resolve_cell_size_3d(bounds, self.config.cell_size)?;
        self.left_bounds.clear();
        self.left_bounds.extend_from_slice(bounds);
        self.right_bounds.clear();
        self.entries.clear();
        self.pairs.clear();
        self.push_entries(bounds, 0)?;
        self.stats = BroadPhaseStats {
            object_count: bounds.len(),
            cell_count: 0,
            cell_entry_count: self.entries.len(),
            candidate_pair_count: 0,
            selected_strategy: BroadPhase3Strategy::SpatialHashGrid,
        };
        Ok(())
    }

    /// Returns candidate pairs for the most recently rebuilt set.
    pub fn candidate_pairs(&mut self) -> Result<&[CollisionPair]> {
        self.entries.sort_unstable();
        self.pairs.clear();
        let mut cell_count = 0;
        let mut start = 0;
        while start < self.entries.len() {
            let cell = self.entries[start].cell;
            let mut end = start + 1;
            while end < self.entries.len() && self.entries[end].cell == cell {
                end += 1;
            }
            cell_count += 1;
            for left in start..end {
                for right in (left + 1)..end {
                    let left_index = self.entries[left].index;
                    let right_index = self.entries[right].index;
                    if self.left_bounds[left_index].intersects(self.left_bounds[right_index])? {
                        self.pairs
                            .push(CollisionPair::ordered(left_index, right_index));
                    }
                }
            }
            start = end;
        }
        finish_pairs_3d(&mut self.pairs);
        self.stats.cell_count = cell_count;
        self.stats.candidate_pair_count = self.pairs.len();
        Ok(&self.pairs)
    }

    /// Returns candidate pairs between two independent sets.
    pub fn candidate_pairs_between(
        &mut self,
        left: &[Bounds3],
        right: &[Bounds3],
    ) -> Result<&[CollisionPair]> {
        validate_bounds3(left)?;
        validate_bounds3(right)?;
        self.cell_size = resolve_cell_size_3d_for_sets(left, right, self.config.cell_size)?;
        self.left_bounds.clear();
        self.left_bounds.extend_from_slice(left);
        self.right_bounds.clear();
        self.right_bounds.extend_from_slice(right);
        self.entries.clear();
        self.pairs.clear();
        self.push_entries(left, 0)?;
        self.push_entries(right, 1)?;
        self.entries.sort_unstable();

        let mut cell_count = 0;
        let mut start = 0;
        while start < self.entries.len() {
            let cell = self.entries[start].cell;
            let mut end = start + 1;
            while end < self.entries.len() && self.entries[end].cell == cell {
                end += 1;
            }
            cell_count += 1;
            for left_entry in start..end {
                if self.entries[left_entry].set != 0 {
                    continue;
                }
                for right_entry in start..end {
                    if self.entries[right_entry].set == 1
                        && self.left_bounds[self.entries[left_entry].index]
                            .intersects(self.right_bounds[self.entries[right_entry].index])?
                    {
                        self.pairs.push(CollisionPair {
                            left_index: self.entries[left_entry].index,
                            right_index: self.entries[right_entry].index,
                        });
                    }
                }
            }
            start = end;
        }
        finish_pairs_3d(&mut self.pairs);
        self.stats = BroadPhaseStats {
            object_count: left.len() + right.len(),
            cell_count,
            cell_entry_count: self.entries.len(),
            candidate_pair_count: self.pairs.len(),
            selected_strategy: BroadPhase3Strategy::SpatialHashGrid,
        };
        Ok(&self.pairs)
    }

    /// Returns stats.
    pub fn stats(&self) -> BroadPhaseStats {
        self.stats
    }

    fn push_entries(&mut self, bounds: &[Bounds3], set: u8) -> Result<()> {
        for (index, bounds) in bounds.iter().copied().enumerate() {
            let (min, max) = bounds_cell_range_3d(bounds, self.cell_size)?;
            for z in min.z..=max.z {
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        self.entries.push(GridEntry3 {
                            cell: GridCell3 { x, y, z },
                            set,
                            index,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

/// Returns broad-phase candidate pairs for 3D bounds.
pub fn broad_phase_pairs_3d(
    bounds: &[Bounds3],
    config: BroadPhase3Config,
) -> Result<Vec<CollisionPair>> {
    config.validate()?;
    validate_bounds3(bounds)?;
    match select_strategy_3d(bounds, config)? {
        BroadPhase3Strategy::Auto => unreachable!("auto strategy must resolve before execution"),
        BroadPhase3Strategy::BruteForce => brute_force_pairs_3d(bounds),
        BroadPhase3Strategy::SweepAndPrune => sweep_and_prune_pairs_3d(bounds),
        BroadPhase3Strategy::SpatialHashGrid => {
            let mut grid = SpatialHashGrid3::new(BroadPhase3Config {
                strategy: BroadPhase3Strategy::SpatialHashGrid,
                ..config
            })?;
            grid.rebuild(bounds)?;
            grid.candidate_pairs().map(|pairs| pairs.to_vec())
        }
    }
}

fn select_strategy_3d(
    bounds: &[Bounds3],
    config: BroadPhase3Config,
) -> Result<BroadPhase3Strategy> {
    Ok(match config.strategy {
        BroadPhase3Strategy::Auto if bounds.len() <= config.brute_force_threshold => {
            BroadPhase3Strategy::BruteForce
        }
        BroadPhase3Strategy::Auto => {
            let cell_size = resolve_cell_size_3d(bounds, config.cell_size)?;
            if bounds.iter().copied().any(|bounds| {
                bounds_cell_count_3d(bounds, cell_size)
                    .map(|count| count > config.max_cells_per_item)
                    .unwrap_or(true)
            }) {
                BroadPhase3Strategy::SweepAndPrune
            } else {
                BroadPhase3Strategy::SpatialHashGrid
            }
        }
        strategy => strategy,
    })
}

fn brute_force_pairs_3d(bounds: &[Bounds3]) -> Result<Vec<CollisionPair>> {
    let mut pairs = Vec::new();
    for left_index in 0..bounds.len() {
        for right_index in (left_index + 1)..bounds.len() {
            if bounds[left_index].intersects(bounds[right_index])? {
                pairs.push(CollisionPair {
                    left_index,
                    right_index,
                });
            }
        }
    }
    Ok(pairs)
}

fn sweep_and_prune_pairs_3d(bounds: &[Bounds3]) -> Result<Vec<CollisionPair>> {
    let mut ordered = bounds
        .iter()
        .copied()
        .enumerate()
        .map(|(index, bounds)| {
            bounds.validate()?;
            Ok((index, bounds.min.x, bounds.max.x, bounds))
        })
        .collect::<Result<Vec<_>>>()?;
    ordered.sort_unstable_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut pairs = Vec::new();
    let mut active = Vec::<(usize, f32, Bounds3)>::new();
    for (index, min_x, max_x, bounds) in ordered {
        active.retain(|(_, active_max_x, _)| *active_max_x > min_x);
        for (active_index, _, active_bounds) in &active {
            if active_bounds.intersects(bounds)? {
                pairs.push(CollisionPair::ordered(*active_index, index));
            }
        }
        active.push((index, max_x, bounds));
    }
    finish_pairs_3d(&mut pairs);
    Ok(pairs)
}

fn resolve_cell_size_3d(bounds: &[Bounds3], cell_size: SpatialCellSize3) -> Result<f32> {
    match cell_size {
        SpatialCellSize3::Fixed { size } => {
            if !size.is_finite() || size <= 0.0 {
                return Err(invalid_argument(
                    "fixed spatial cell size must be finite and greater than zero",
                ));
            }
            Ok(size)
        }
        SpatialCellSize3::Auto => {
            if bounds.is_empty() {
                return Ok(f32::EPSILON);
            }
            let mut extents = bounds
                .iter()
                .copied()
                .map(|bounds| {
                    let size = bounds.size();
                    size.x.max(size.y).max(size.z).max(f32::EPSILON)
                })
                .collect::<Vec<_>>();
            extents.sort_unstable_by(f32::total_cmp);
            Ok(extents[extents.len() / 2])
        }
    }
}

fn resolve_cell_size_3d_for_sets(
    left: &[Bounds3],
    right: &[Bounds3],
    cell_size: SpatialCellSize3,
) -> Result<f32> {
    match cell_size {
        SpatialCellSize3::Fixed { .. } => resolve_cell_size_3d(left, cell_size),
        SpatialCellSize3::Auto => {
            let mut extents = left
                .iter()
                .chain(right.iter())
                .copied()
                .map(|bounds| {
                    let size = bounds.size();
                    size.x.max(size.y).max(size.z).max(f32::EPSILON)
                })
                .collect::<Vec<_>>();
            if extents.is_empty() {
                return Ok(f32::EPSILON);
            }
            extents.sort_unstable_by(f32::total_cmp);
            Ok(extents[extents.len() / 2])
        }
    }
}

fn bounds_cell_range_3d(bounds: Bounds3, cell_size: f32) -> Result<(GridCell3, GridCell3)> {
    bounds.validate()?;
    let max = Point3::new(
        (bounds.max.x - f32::EPSILON).max(bounds.min.x),
        (bounds.max.y - f32::EPSILON).max(bounds.min.y),
        (bounds.max.z - f32::EPSILON).max(bounds.min.z),
    );
    Ok((
        GridCell3 {
            x: (bounds.min.x / cell_size).floor() as i64,
            y: (bounds.min.y / cell_size).floor() as i64,
            z: (bounds.min.z / cell_size).floor() as i64,
        },
        GridCell3 {
            x: (max.x / cell_size).floor() as i64,
            y: (max.y / cell_size).floor() as i64,
            z: (max.z / cell_size).floor() as i64,
        },
    ))
}

fn bounds_cell_count_3d(bounds: Bounds3, cell_size: f32) -> Result<usize> {
    let (min, max) = bounds_cell_range_3d(bounds, cell_size)?;
    let x = (max.x - min.x + 1).max(0) as usize;
    let y = (max.y - min.y + 1).max(0) as usize;
    let z = (max.z - min.z + 1).max(0) as usize;
    Ok(x.saturating_mul(y).saturating_mul(z))
}

fn validate_bounds3(bounds: &[Bounds3]) -> Result<()> {
    for bounds in bounds {
        bounds.validate()?;
    }
    Ok(())
}

fn finish_pairs_3d(pairs: &mut Vec<CollisionPair>) {
    pairs.sort_unstable();
    pairs.dedup();
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for transform3.
pub struct Transform3 {
    /// The translation value.
    pub translation: Vector3,
    /// The scale value.
    pub scale: f32,
}

impl Transform3 {
    /// Constant for identity.
    pub const IDENTITY: Self = Self {
        translation: Vector3::ZERO,
        scale: 1.0,
    };

    /// Creates a new value.
    pub fn new(translation: Vector3, scale: f32) -> Result<Self> {
        if !translation.is_finite() || !scale.is_finite() || scale == 0.0 {
            return Err(invalid_argument(
                "transform translation must be finite and scale must be finite and non-zero",
            ));
        }
        Ok(Self { translation, scale })
    }

    /// Creates a translation transform.
    pub fn translation(translation: Vector3) -> Self {
        Self {
            translation,
            scale: 1.0,
        }
    }

    /// Creates a uniform scaling transform.
    pub fn scaling(scale: f32) -> Result<Self> {
        Self::new(Vector3::ZERO, scale)
    }

    /// Returns apply point.
    pub fn apply_point(self, point: Point3) -> Point3 {
        Point3::new(
            point.x * self.scale + self.translation.x,
            point.y * self.scale + self.translation.y,
            point.z * self.scale + self.translation.z,
        )
    }

    /// Returns apply vector.
    pub fn apply_vector(self, vector: Vector3) -> Vector3 {
        vector * self.scale
    }

    /// Returns inverse.
    pub fn inverse(self) -> Result<Self> {
        if !self.translation.is_finite() || !self.scale.is_finite() || self.scale == 0.0 {
            return Err(invalid_argument(
                "transform translation must be finite and scale must be finite and non-zero",
            ));
        }
        Self::new(self.translation * (-1.0 / self.scale), 1.0 / self.scale)
    }

    /// Returns compose.
    pub fn compose(self, next: Self) -> Result<Self> {
        Self::new(
            next.apply_vector(self.translation) + next.translation,
            self.scale * next.scale,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for quaternion.
pub struct Quaternion {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
    /// The z value.
    pub z: f32,
    /// The w value.
    pub w: f32,
}

impl Quaternion {
    /// Constant for identity.
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Creates a new value.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Returns whether this value is finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    /// Builds this value from axis angle.
    pub fn from_axis_angle(axis: Vector3, angle_radians: f32) -> Result<Self> {
        validate_finite_vector(axis, "axis")?;
        if !angle_radians.is_finite() {
            return Err(invalid_argument("angle must be finite"));
        }
        let axis = axis.normalize()?;
        let half = angle_radians * 0.5;
        let sin = half.sin();
        Ok(Self::new(
            axis.x * sin,
            axis.y * sin,
            axis.z * sin,
            half.cos(),
        ))
    }

    /// Returns dot.
    pub fn dot(self, rhs: Self) -> f32 {
        self.x.mul_add(
            rhs.x,
            self.y.mul_add(rhs.y, self.z.mul_add(rhs.z, self.w * rhs.w)),
        )
    }

    /// Returns norm.
    pub fn norm(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Normalizes this value.
    pub fn normalize(self) -> Result<Self> {
        if !self.is_finite() {
            return Err(invalid_argument("quaternion components must be finite"));
        }
        let norm = self.norm();
        if norm <= f32::EPSILON {
            return Err(invalid_argument(
                "quaternion norm must be greater than zero",
            ));
        }
        Ok(Self::new(
            self.x / norm,
            self.y / norm,
            self.z / norm,
            self.w / norm,
        ))
    }

    /// Returns conjugate.
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Returns rotate vector.
    pub fn rotate_vector(self, vector: Vector3) -> Result<Vector3> {
        let q = self.normalize()?;
        validate_finite_vector(vector, "vector")?;
        let u = Vector3::new(q.x, q.y, q.z);
        let uv = u.cross(vector);
        let uuv = u.cross(uv);
        Ok(vector + ((2.0 * q.w) * uv) + (2.0 * uuv))
    }

    /// Returns mul quaternion.
    pub fn mul_quaternion(self, rhs: Self) -> Result<Self> {
        let lhs = self.normalize()?;
        let rhs = rhs.normalize()?;
        Ok(Self::new(
            lhs.w
                .mul_add(rhs.x, lhs.x.mul_add(rhs.w, lhs.y * rhs.z - lhs.z * rhs.y)),
            lhs.w
                .mul_add(rhs.y, -lhs.x * rhs.z + lhs.y.mul_add(rhs.w, lhs.z * rhs.x)),
            lhs.w
                .mul_add(rhs.z, lhs.x * rhs.y - lhs.y * rhs.x + lhs.z * rhs.w),
            lhs.w
                .mul_add(rhs.w, -(lhs.x * rhs.x + lhs.y * rhs.y + lhs.z * rhs.z)),
        ))
    }

    /// Returns normalized linear interpolation.
    pub fn nlerp(self, rhs: Self, t: f32) -> Result<Self> {
        if !t.is_finite() {
            return Err(invalid_argument("interpolation factor must be finite"));
        }
        let lhs = self.normalize()?;
        let mut rhs = rhs.normalize()?;
        if lhs.dot(rhs) < 0.0 {
            rhs = Self::new(-rhs.x, -rhs.y, -rhs.z, -rhs.w);
        }
        Self::new(
            lhs.x + (rhs.x - lhs.x) * t,
            lhs.y + (rhs.y - lhs.y) * t,
            lhs.z + (rhs.z - lhs.z) * t,
            lhs.w + (rhs.w - lhs.w) * t,
        )
        .normalize()
    }

    /// Returns spherical linear interpolation.
    pub fn slerp(self, rhs: Self, t: f32) -> Result<Self> {
        if !t.is_finite() {
            return Err(invalid_argument("interpolation factor must be finite"));
        }
        let lhs = self.normalize()?;
        let mut rhs = rhs.normalize()?;
        let mut dot = lhs.dot(rhs);
        if dot < 0.0 {
            rhs = Self::new(-rhs.x, -rhs.y, -rhs.z, -rhs.w);
            dot = -dot;
        }
        if dot > 0.9995 {
            return lhs.nlerp(rhs, t);
        }
        let theta_0 = dot.clamp(-1.0, 1.0).acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();
        let s0 = theta.cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;
        Self::new(
            lhs.x * s0 + rhs.x * s1,
            lhs.y * s0 + rhs.y * s1,
            lhs.z * s0 + rhs.z * s1,
            lhs.w * s0 + rhs.w * s1,
        )
        .normalize()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for rigid transform3.
pub struct RigidTransform3 {
    /// The rotation value.
    pub rotation: Quaternion,
    /// The translation value.
    pub translation: Vector3,
}

impl RigidTransform3 {
    /// Constant for identity.
    pub const IDENTITY: Self = Self {
        rotation: Quaternion::IDENTITY,
        translation: Vector3::ZERO,
    };

    /// Creates a new value.
    pub fn new(rotation: Quaternion, translation: Vector3) -> Result<Self> {
        let rotation = rotation.normalize()?;
        validate_finite_vector(translation, "translation")?;
        Ok(Self {
            rotation,
            translation,
        })
    }

    /// Returns apply point.
    pub fn apply_point(self, point: Point3) -> Result<Point3> {
        let rotated = self
            .rotation
            .rotate_vector(point - Point3::new(0.0, 0.0, 0.0))?;
        Ok(Point3::new(
            rotated.x + self.translation.x,
            rotated.y + self.translation.y,
            rotated.z + self.translation.z,
        ))
    }

    /// Returns apply vector.
    pub fn apply_vector(self, vector: Vector3) -> Result<Vector3> {
        self.rotation.rotate_vector(vector)
    }

    /// Returns inverse.
    pub fn inverse(self) -> Result<Self> {
        let rotation = self.rotation.normalize()?.conjugate();
        let translation = rotation.rotate_vector(self.translation * -1.0)?;
        Self::new(rotation, translation)
    }

    /// Returns compose.
    pub fn compose(self, rhs: Self) -> Result<Self> {
        let rotation = self.rotation.mul_quaternion(rhs.rotation)?;
        let translation = self.apply_vector(rhs.translation)? + self.translation;
        Self::new(rotation, translation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for line segment3.
pub struct LineSegment3 {
    /// The start value.
    pub start: Point3,
    /// The end value.
    pub end: Point3,
}

impl LineSegment3 {
    /// Creates a new value.
    pub fn new(start: Point3, end: Point3) -> Result<Self> {
        validate_points(&[start, end])?;
        if start == end {
            return Err(invalid_argument(
                "line segment start and end must not be identical",
            ));
        }
        Ok(Self { start, end })
    }

    /// Returns length.
    pub fn length(self) -> f32 {
        self.start.distance(self.end)
    }

    /// Returns midpoint.
    pub fn midpoint(self) -> Point3 {
        self.start.midpoint(self.end)
    }

    /// Returns direction.
    pub fn direction(self) -> Result<Vector3> {
        (self.end - self.start).normalize()
    }

    /// Returns closest point on this segment to a point.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_segment(self, point)
    }

    /// Returns distance from this segment to a point.
    pub fn distance_to_point(self, point: Point3) -> Result<f32> {
        Ok(self.closest_point(point)?.distance(point))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for ray3.
pub struct Ray3 {
    /// The origin value.
    pub origin: Point3,
    /// Unit direction.
    pub direction: Vector3,
}

impl Ray3 {
    /// Creates a new value.
    pub fn new(origin: Point3, direction: Vector3) -> Result<Self> {
        validate_points(&[origin])?;
        Ok(Self {
            origin,
            direction: direction.normalize()?,
        })
    }

    /// Returns point at distance.
    pub fn at(self, distance: f32) -> Result<Point3> {
        if !distance.is_finite() {
            return Err(invalid_argument("ray distance must be finite"));
        }
        Ok(self.origin + self.direction * distance)
    }

    /// Returns closest point on this ray to a point.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_ray(self, point)
    }

    /// Returns distance from this ray to a point.
    pub fn distance_to_point(self, point: Point3) -> Result<f32> {
        Ok(self.closest_point(point)?.distance(point))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for plane3.
pub struct Plane3 {
    /// Unit normal.
    pub normal: Vector3,
    /// Signed distance from origin.
    pub d: f32,
}

impl Plane3 {
    /// Creates a new value from a point and normal.
    pub fn from_point_normal(point: Point3, normal: Vector3) -> Result<Self> {
        validate_points(&[point])?;
        let normal = normal.normalize()?;
        let d = -normal.dot(Vector3::new(point.x, point.y, point.z));
        Ok(Self { normal, d })
    }

    /// Returns signed distance to point.
    pub fn signed_distance(self, point: Point3) -> Result<f32> {
        validate_points(&[point])?;
        Ok(self.normal.dot(Vector3::new(point.x, point.y, point.z)) + self.d)
    }

    /// Returns the orthogonal projection of a point onto this plane.
    pub fn project_point(self, point: Point3) -> Result<Point3> {
        project_point_to_plane(point, self)
    }

    /// Returns the forward ray-plane intersection point, if any.
    pub fn intersect_ray(self, ray: Ray3) -> Result<Option<Point3>> {
        intersect_ray_plane(ray, self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Data type for sphere3.
pub struct Sphere3 {
    /// The center value.
    pub center: Point3,
    /// The radius value.
    pub radius: f32,
}

impl Sphere3 {
    /// Creates a new value.
    pub fn new(center: Point3, radius: f32) -> Result<Self> {
        validate_points(&[center])?;
        if !radius.is_finite() || radius <= 0.0 {
            return Err(invalid_argument(
                "sphere radius must be finite and greater than zero",
            ));
        }
        Ok(Self { center, radius })
    }

    /// Returns contains point.
    pub fn contains_point(self, point: Point3) -> Result<bool> {
        validate_points(&[point])?;
        Ok(self.center.distance(point) <= self.radius)
    }

    /// Returns surface area.
    pub fn surface_area(self) -> f32 {
        sphere_surface_area(self)
    }

    /// Returns volume.
    pub fn volume(self) -> f32 {
        sphere_volume(self)
    }

    /// Returns signed distance to the sphere surface.
    pub fn signed_distance(self, point: Point3) -> Result<f32> {
        validate_points(&[point])?;
        Ok(self.center.distance(point) - self.radius)
    }

    /// Returns closest point on the sphere surface.
    pub fn closest_point(self, point: Point3) -> Result<Point3> {
        closest_point_on_sphere(self, point)
    }

    /// Returns forward ray-sphere intersection points in distance order.
    pub fn intersect_ray(self, ray: Ray3) -> Result<Vec<Point3>> {
        intersect_ray_sphere(ray, self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for point cloud.
pub struct PointCloud {
    points: Vec<Point3>,
}

impl PointCloud {
    /// Creates a new value.
    pub fn new(points: impl Into<Vec<Point3>>) -> Result<Self> {
        let points = points.into();
        validate_points(&points)?;
        Ok(Self { points })
    }

    /// Returns points.
    pub fn points(&self) -> &[Point3] {
        &self.points
    }

    /// Returns bounds.
    pub fn bounds(&self) -> Result<Option<Bounds3>> {
        Bounds3::from_points(&self.points)
    }

    /// Returns centroid.
    pub fn centroid(&self) -> Result<Option<Point3>> {
        centroid(&self.points)
    }

    /// Returns transformed.
    pub fn transformed(&self, transform: Transform3) -> Result<Self> {
        PointCloud::new(
            self.points
                .iter()
                .copied()
                .map(|point| transform.apply_point(point))
                .collect::<Vec<_>>(),
        )
    }

    /// Returns transformed rigid.
    pub fn transformed_rigid(&self, transform: RigidTransform3) -> Result<Self> {
        PointCloud::new(transform_rigid(&self.points, transform)?)
    }

    /// Returns voxel downsample.
    pub fn voxel_downsample(&self, voxel_size: f32) -> Result<Self> {
        PointCloud::new(voxel_downsample(&self.points, voxel_size)?)
    }

    /// Returns center and scale.
    pub fn center_and_scale(&self, target_extent: f32) -> Result<Option<Self>> {
        center_and_scale(&self.points, target_extent)
            .map(|value| value.map(|points| Self { points }))
    }

    /// Returns nearest point.
    pub fn nearest_point(&self, query: Point3) -> Result<Option<Point3>> {
        nearest_point(&self.points, query)
    }
}

/// Returns centroid.
pub fn centroid(points: &[Point3]) -> Result<Option<Point3>> {
    validate_points(points)?;
    if points.is_empty() {
        return Ok(None);
    }
    let mut sum = Vector3::ZERO;
    for point in points {
        sum += Vector3::new(point.x, point.y, point.z);
    }
    let count = points.len() as f32;
    Ok(Some(Point3::new(
        sum.x / count,
        sum.y / count,
        sum.z / count,
    )))
}

/// Returns point distance.
pub fn point_distance(a: Point3, b: Point3) -> Result<f32> {
    validate_points(&[a, b])?;
    Ok(a.distance(b))
}

/// Returns nearest point.
pub fn nearest_point(points: &[Point3], query: Point3) -> Result<Option<Point3>> {
    validate_points(points)?;
    validate_points(&[query])?;
    Ok(points.iter().copied().min_by(|a, b| {
        a.distance(query)
            .partial_cmp(&b.distance(query))
            .unwrap_or(std::cmp::Ordering::Equal)
    }))
}

/// Returns closest point on segment to a point.
pub fn closest_point_on_segment(segment: LineSegment3, point: Point3) -> Result<Point3> {
    validate_points(&[segment.start, segment.end, point])?;
    let segment_vector = segment.end - segment.start;
    let length_squared = segment_vector.length_squared();
    if length_squared <= f32::EPSILON {
        return Err(invalid_argument(
            "line segment start and end must not be identical",
        ));
    }
    let t = ((point - segment.start).dot(segment_vector) / length_squared).clamp(0.0, 1.0);
    Ok(segment.start + segment_vector * t)
}

/// Returns closest point on ray to a point.
pub fn closest_point_on_ray(ray: Ray3, point: Point3) -> Result<Point3> {
    validate_points(&[ray.origin, point])?;
    validate_finite_vector(ray.direction, "ray direction")?;
    let direction = ray.direction.normalize()?;
    let distance = (point - ray.origin).dot(direction).max(0.0);
    Ok(ray.origin + direction * distance)
}

/// Returns point projected to plane.
pub fn project_point_to_plane(point: Point3, plane: Plane3) -> Result<Point3> {
    validate_points(&[point])?;
    validate_finite_vector(plane.normal, "plane normal")?;
    let normal = plane.normal.normalize()?;
    if !plane.d.is_finite() {
        return Err(invalid_argument("plane distance must be finite"));
    }
    let distance = normal.dot(Vector3::new(point.x, point.y, point.z)) + plane.d;
    Ok(point - normal * distance)
}

/// Returns forward ray-plane intersection point, if any.
pub fn intersect_ray_plane(ray: Ray3, plane: Plane3) -> Result<Option<Point3>> {
    validate_points(&[ray.origin])?;
    validate_finite_vector(ray.direction, "ray direction")?;
    validate_finite_vector(plane.normal, "plane normal")?;
    if !plane.d.is_finite() {
        return Err(invalid_argument("plane distance must be finite"));
    }
    let direction = ray.direction.normalize()?;
    let normal = plane.normal.normalize()?;
    let denominator = normal.dot(direction);
    if denominator.abs() <= f32::EPSILON {
        return Ok(None);
    }
    let numerator = -(normal.dot(Vector3::new(ray.origin.x, ray.origin.y, ray.origin.z)) + plane.d);
    let distance = numerator / denominator;
    if distance < 0.0 {
        return Ok(None);
    }
    Ok(Some(ray.origin + direction * distance))
}

/// Returns sphere surface area.
pub fn sphere_surface_area(sphere: Sphere3) -> f32 {
    4.0 * std::f32::consts::PI * sphere.radius * sphere.radius
}

/// Returns sphere volume.
pub fn sphere_volume(sphere: Sphere3) -> f32 {
    (4.0 / 3.0) * std::f32::consts::PI * sphere.radius * sphere.radius * sphere.radius
}

/// Returns closest point on sphere surface.
pub fn closest_point_on_sphere(sphere: Sphere3, point: Point3) -> Result<Point3> {
    validate_points(&[sphere.center, point])?;
    if !sphere.radius.is_finite() || sphere.radius <= 0.0 {
        return Err(invalid_argument(
            "sphere radius must be finite and greater than zero",
        ));
    }
    let offset = point - sphere.center;
    let direction = if offset.length() <= f32::EPSILON {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        offset.normalize()?
    };
    Ok(sphere.center + direction * sphere.radius)
}

/// Returns forward ray-sphere intersection points in distance order.
pub fn intersect_ray_sphere(ray: Ray3, sphere: Sphere3) -> Result<Vec<Point3>> {
    validate_points(&[ray.origin, sphere.center])?;
    validate_finite_vector(ray.direction, "ray direction")?;
    if !sphere.radius.is_finite() || sphere.radius <= 0.0 {
        return Err(invalid_argument(
            "sphere radius must be finite and greater than zero",
        ));
    }
    let direction = ray.direction.normalize()?;
    let origin_to_center = ray.origin - sphere.center;
    let b = 2.0 * origin_to_center.dot(direction);
    let c = origin_to_center.length_squared() - sphere.radius * sphere.radius;
    let discriminant = b.mul_add(b, -4.0 * c);
    if discriminant < 0.0 {
        return Ok(Vec::new());
    }
    let sqrt_discriminant = discriminant.sqrt();
    let mut distances = [
        (-b - sqrt_discriminant) * 0.5,
        (-b + sqrt_discriminant) * 0.5,
    ];
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut intersections = Vec::new();
    for distance in distances {
        if distance < 0.0 {
            continue;
        }
        if intersections.last().is_some_and(|point: &Point3| {
            point.distance(ray.origin + direction * distance) <= f32::EPSILON
        }) {
            continue;
        }
        intersections.push(ray.origin + direction * distance);
    }
    Ok(intersections)
}

/// Returns transform rigid.
pub fn transform_rigid(points: &[Point3], transform: RigidTransform3) -> Result<Vec<Point3>> {
    validate_points(points)?;
    points
        .iter()
        .copied()
        .map(|point| transform.apply_point(point))
        .collect()
}

/// Returns voxel downsample.
pub fn voxel_downsample(points: &[Point3], voxel_size: f32) -> Result<Vec<Point3>> {
    validate_points(points)?;
    if !voxel_size.is_finite() || voxel_size <= 0.0 {
        return Err(invalid_argument(
            "voxel size must be finite and greater than zero",
        ));
    }
    let mut buckets: BTreeMap<(i32, i32, i32), (Vector3, usize)> = BTreeMap::new();
    for point in points {
        let key = (
            (point.x / voxel_size).floor() as i32,
            (point.y / voxel_size).floor() as i32,
            (point.z / voxel_size).floor() as i32,
        );
        let entry = buckets.entry(key).or_insert((Vector3::ZERO, 0));
        entry.0 += Vector3::new(point.x, point.y, point.z);
        entry.1 += 1;
    }
    Ok(buckets
        .into_values()
        .map(|(sum, count)| {
            let denom = count as f32;
            Point3::new(sum.x / denom, sum.y / denom, sum.z / denom)
        })
        .collect())
}

/// Returns center and scale.
pub fn center_and_scale(points: &[Point3], target_extent: f32) -> Result<Option<Vec<Point3>>> {
    validate_points(points)?;
    if points.is_empty() {
        return Ok(None);
    }
    if !target_extent.is_finite() || target_extent <= 0.0 {
        return Err(invalid_argument(
            "target extent must be finite and greater than zero",
        ));
    }
    let Some(bounds) = Bounds3::from_points(points)? else {
        return Ok(None);
    };
    let center = bounds.center();
    let extent = bounds.size();
    let max_extent = extent.x.max(extent.y).max(extent.z);
    let scale = if max_extent <= f32::EPSILON {
        1.0
    } else {
        target_extent / max_extent
    };
    Ok(Some(
        points
            .iter()
            .map(|point| {
                let relative = *point - center;
                Point3::new(relative.x * scale, relative.y * scale, relative.z * scale)
            })
            .collect(),
    ))
}

fn validate_points(points: &[Point3]) -> Result<()> {
    if points.iter().any(|point| !point.is_finite()) {
        return Err(invalid_argument("points must be finite"));
    }
    Ok(())
}

fn validate_finite_vector(vector: Vector3, name: &str) -> Result<()> {
    if vector.is_finite() {
        Ok(())
    } else {
        Err(invalid_argument(format!(
            "{name} components must be finite"
        )))
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    #[test]
    fn computes_point_cloud_bounds_and_centroid() {
        let cloud =
            PointCloud::new([Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 4.0, 6.0)]).unwrap();
        assert_eq!(cloud.centroid().unwrap(), Some(Point3::new(1.0, 2.0, 3.0)));
        assert_eq!(
            cloud.bounds().unwrap().unwrap().size(),
            Vector3::new(2.0, 4.0, 6.0)
        );
    }

    #[test]
    fn quaternion_normalization_and_rigid_inverse_are_stable() {
        let rotation = Quaternion::from_axis_angle(Vector3::new(0.0, 0.0, 2.0), FRAC_PI_2)
            .unwrap()
            .normalize()
            .unwrap();
        let transform = RigidTransform3::new(rotation, Vector3::new(1.0, 2.0, 3.0)).unwrap();
        let point = Point3::new(1.0, 0.0, 0.0);
        let transformed = transform.apply_point(point).unwrap();
        let recovered = transform
            .inverse()
            .unwrap()
            .apply_point(transformed)
            .unwrap();
        assert!((recovered.x - point.x).abs() < 0.001);
        assert!((recovered.y - point.y).abs() < 0.001);
        assert!((recovered.z - point.z).abs() < 0.001);
    }

    #[test]
    fn voxel_downsampling_is_deterministic() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.1, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        let first = voxel_downsample(&points, 0.5).unwrap();
        let second = voxel_downsample(&points, 0.5).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn center_and_scale_normalizes_extent() {
        let points = vec![Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 5.0, 1.0)];
        let normalized = center_and_scale(&points, 2.0).unwrap().unwrap();
        let bounds = Bounds3::from_points(&normalized).unwrap().unwrap();
        let extent = bounds.size();
        assert!((extent.y - 2.0).abs() < 0.001);
        assert_eq!(bounds.center(), Point3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn transform_helpers_round_trip_points() {
        let transform = Transform3::translation(Vector3::new(2.0, 0.0, 0.0))
            .compose(Transform3::scaling(3.0).unwrap())
            .unwrap();
        let point = Point3::new(1.0, 2.0, 3.0);
        let transformed = transform.apply_point(point);
        let recovered = transform.inverse().unwrap().apply_point(transformed);
        assert!((recovered.x - point.x).abs() < 0.001);
        assert!((recovered.y - point.y).abs() < 0.001);
        assert!((recovered.z - point.z).abs() < 0.001);
    }

    #[test]
    fn quaternion_slerp_and_spatial_primitives_are_stable() {
        let identity = Quaternion::IDENTITY;
        let half_turn =
            Quaternion::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), std::f32::consts::PI).unwrap();
        let midpoint = identity.slerp(half_turn, 0.5).unwrap();
        assert!((midpoint.norm() - 1.0).abs() < 0.001);

        let ray = Ray3::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 2.0, 0.0)).unwrap();
        assert_eq!(ray.at(2.0).unwrap(), Point3::new(0.0, 2.0, 0.0));

        let plane =
            Plane3::from_point_normal(Point3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 1.0, 0.0))
                .unwrap();
        assert!((plane.signed_distance(Point3::new(0.0, 3.0, 0.0)).unwrap() - 2.0).abs() < 0.001);

        let sphere = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 1.0).unwrap();
        assert!(sphere.contains_point(Point3::new(0.5, 0.0, 0.0)).unwrap());
    }

    #[test]
    fn point_cloud_reports_nearest_point() {
        let cloud =
            PointCloud::new([Point3::new(-1.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)]).unwrap();
        assert_eq!(
            cloud.nearest_point(Point3::new(1.5, 0.0, 0.0)).unwrap(),
            Some(Point3::new(2.0, 0.0, 0.0))
        );
    }

    #[test]
    fn closest_point_helpers_clamp_to_segment_and_ray() {
        let segment =
            LineSegment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)).unwrap();
        assert_eq!(
            segment.closest_point(Point3::new(1.0, 2.0, 0.0)).unwrap(),
            Point3::new(1.0, 0.0, 0.0)
        );
        assert_eq!(
            segment.closest_point(Point3::new(4.0, 0.0, 0.0)).unwrap(),
            Point3::new(2.0, 0.0, 0.0)
        );

        let ray = Ray3::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)).unwrap();
        assert_eq!(
            ray.closest_point(Point3::new(-1.0, 2.0, 0.0)).unwrap(),
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn plane_projection_and_ray_intersection_are_stable() {
        let plane =
            Plane3::from_point_normal(Point3::new(0.0, 2.0, 0.0), Vector3::new(0.0, 1.0, 0.0))
                .unwrap();
        assert_eq!(
            plane.project_point(Point3::new(1.0, 5.0, 1.0)).unwrap(),
            Point3::new(1.0, 2.0, 1.0)
        );

        let ray = Ray3::new(Point3::new(0.0, 5.0, 0.0), Vector3::new(0.0, -1.0, 0.0)).unwrap();
        assert_eq!(
            plane.intersect_ray(ray).unwrap(),
            Some(Point3::new(0.0, 2.0, 0.0))
        );
    }

    #[test]
    fn sphere_algorithms_report_surface_volume_and_intersections() {
        let sphere = Sphere3::new(Point3::new(0.0, 0.0, 0.0), 2.0).unwrap();
        assert!((sphere.surface_area() - (16.0 * std::f32::consts::PI)).abs() < 0.001);
        assert!((sphere.volume() - ((32.0 / 3.0) * std::f32::consts::PI)).abs() < 0.001);
        assert!((sphere.signed_distance(Point3::new(3.0, 0.0, 0.0)).unwrap() - 1.0).abs() < 0.001);
        assert_eq!(
            sphere.closest_point(Point3::new(3.0, 0.0, 0.0)).unwrap(),
            Point3::new(2.0, 0.0, 0.0)
        );

        let ray = Ray3::new(Point3::new(-3.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)).unwrap();
        let intersections = sphere.intersect_ray(ray).unwrap();
        assert_eq!(
            intersections,
            vec![Point3::new(-2.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0)]
        );
    }

    #[test]
    fn bounds3_validates_and_reports_basic_operations() {
        assert!(Bounds3::new(Point3::new(f32::NAN, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)).is_err());
        assert!(Bounds3::new(Point3::new(2.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)).is_err());

        let a = Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(2.0, 2.0, 2.0)).unwrap();
        let b = Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)).unwrap();
        let touching =
            Bounds3::new(Point3::new(2.0, 0.0, 0.0), Point3::new(4.0, 1.0, 1.0)).unwrap();

        assert!(a.contains_point(Point3::new(0.0, 0.0, 0.0)).unwrap());
        assert!(a.intersects(b).unwrap());
        assert!(!a.intersects(touching).unwrap());
        assert_eq!(
            a.intersection(b).unwrap(),
            Some(Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0)).unwrap())
        );
        assert_eq!(
            a.union(b).unwrap(),
            Bounds3::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(3.0, 3.0, 3.0)).unwrap()
        );
        assert_eq!(a.volume().unwrap(), 27.0);
    }

    #[test]
    fn broad_phase_3d_strategies_match_for_mixed_bounds() {
        let bounds = [
            Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)).unwrap(),
            Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)).unwrap(),
            Bounds3::new(Point3::new(10.0, 0.0, 0.0), Point3::new(12.0, 2.0, 2.0)).unwrap(),
            Bounds3::new(Point3::new(10.5, 0.0, 0.0), Point3::new(11.0, 1.0, 1.0)).unwrap(),
        ];
        let brute = broad_phase_pairs_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::BruteForce,
                ..BroadPhase3Config::default()
            },
        )
        .unwrap();
        let sweep = broad_phase_pairs_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::SweepAndPrune,
                ..BroadPhase3Config::default()
            },
        )
        .unwrap();
        let grid = broad_phase_pairs_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::SpatialHashGrid,
                cell_size: SpatialCellSize3::Fixed { size: 1.0 },
                ..BroadPhase3Config::default()
            },
        )
        .unwrap();

        assert_eq!(brute, sweep);
        assert_eq!(brute, grid);
        assert_eq!(
            brute,
            vec![
                CollisionPair {
                    left_index: 0,
                    right_index: 1
                },
                CollisionPair {
                    left_index: 2,
                    right_index: 3
                }
            ]
        );
    }

    #[test]
    fn broad_phase_3d_auto_uses_sweep_for_large_cell_spans() {
        let bounds = [
            Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(100.0, 100.0, 100.0)).unwrap(),
            Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0)).unwrap(),
        ];
        let strategy = select_strategy_3d(
            &bounds,
            BroadPhase3Config {
                strategy: BroadPhase3Strategy::Auto,
                brute_force_threshold: 0,
                max_cells_per_item: 4,
                cell_size: SpatialCellSize3::Fixed { size: 1.0 },
            },
        )
        .unwrap();

        assert_eq!(strategy, BroadPhase3Strategy::SweepAndPrune);
    }

    #[test]
    fn spatial_hash_grid_3d_reports_cross_set_pairs() {
        let left = [
            Bounds3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)).unwrap(),
            Bounds3::new(Point3::new(10.0, 0.0, 0.0), Point3::new(12.0, 2.0, 2.0)).unwrap(),
        ];
        let right = [
            Bounds3::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)).unwrap(),
            Bounds3::new(Point3::new(30.0, 0.0, 0.0), Point3::new(31.0, 1.0, 1.0)).unwrap(),
        ];
        let mut grid = SpatialHashGrid3::new(BroadPhase3Config {
            strategy: BroadPhase3Strategy::SpatialHashGrid,
            cell_size: SpatialCellSize3::Fixed { size: 1.0 },
            ..BroadPhase3Config::default()
        })
        .unwrap();
        let pairs = grid.candidate_pairs_between(&left, &right).unwrap();

        assert_eq!(
            pairs,
            &[CollisionPair {
                left_index: 0,
                right_index: 0
            }]
        );
        assert_eq!(grid.stats().object_count, 4);
        assert_eq!(grid.stats().candidate_pair_count, 1);
    }
}
