use video_analysis_core::Result;

use crate::{invalid_argument, Bounds3, Point3};

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

pub(crate) fn select_strategy_3d(
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
