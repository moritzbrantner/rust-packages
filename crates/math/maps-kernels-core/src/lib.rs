#![doc = include_str!("../README.md")]

use numbers_core::NumberRange;
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

/// Resamples an open line represented as flat `[x0, y0, x1, y1, ...]` coordinates.
pub fn resample_line_flat(coordinates: &[f64], coordinate_count: usize) -> Result<Vec<f64>> {
    validate_flat_coordinates(coordinates, 2, "line coordinates")?;
    validate_coordinate_count(coordinate_count, 2)?;

    let source_count = coordinates.len() / 2;

    if source_count == coordinate_count {
        return Ok(coordinates.to_vec());
    }

    let distances = cumulative_distances(coordinates, false)?;
    let total_distance = *distances.last().unwrap_or(&0.0);

    if total_distance == 0.0 {
        return Ok(repeat_position(coordinates, coordinate_count));
    }

    let mut samples = Vec::with_capacity(coordinate_count * 2);
    let mut segment_index = 0;

    for index in 0..coordinate_count {
        if index == coordinate_count - 1 {
            samples.extend_from_slice(&coordinates[(source_count - 1) * 2..source_count * 2]);
            continue;
        }

        let target_distance = total_distance * index as f64 / (coordinate_count - 1) as f64;
        let sample = interpolate_along_path_from_segment(
            coordinates,
            &distances,
            target_distance,
            false,
            segment_index,
        )?;

        samples.extend_from_slice(&sample.position);
        segment_index = sample.segment_index;
    }

    Ok(samples)
}

/// Resamples an open ring represented as flat `[x0, y0, x1, y1, ...]` coordinates.
pub fn resample_ring_flat(open_ring: &[f64], coordinate_count: usize) -> Result<Vec<f64>> {
    validate_flat_coordinates(open_ring, 3, "ring coordinates")?;
    validate_coordinate_count(coordinate_count, 3)?;

    let distances = cumulative_distances(open_ring, true)?;
    let total_distance = *distances.last().unwrap_or(&0.0);

    if total_distance == 0.0 {
        return Ok(repeat_position(open_ring, coordinate_count));
    }

    let mut samples = Vec::with_capacity(coordinate_count * 2);
    let mut segment_index = 0;

    for index in 0..coordinate_count {
        let target_distance = total_distance * index as f64 / coordinate_count as f64;
        let sample = interpolate_along_path_from_segment(
            open_ring,
            &distances,
            target_distance,
            true,
            segment_index,
        )?;

        samples.extend_from_slice(&sample.position);
        segment_index = sample.segment_index;
    }

    Ok(samples)
}

fn validate_flat_coordinates(coordinates: &[f64], min_points: usize, label: &str) -> Result<()> {
    if coordinates.len() < min_points * 2 {
        return Err(invalid_argument(format!(
            "{label} must contain at least {min_points} positions"
        )));
    }

    if !coordinates.len().is_multiple_of(2) {
        return Err(invalid_argument(format!("{label} length must be even")));
    }

    if coordinates.iter().any(|value| !value.is_finite()) {
        return Err(invalid_argument(format!("{label} must be finite")));
    }

    Ok(())
}

fn validate_coordinate_count(coordinate_count: usize, minimum: usize) -> Result<()> {
    if coordinate_count < minimum {
        return Err(invalid_argument(format!(
            "coordinate count must be at least {minimum}"
        )));
    }

    Ok(())
}

fn cumulative_distances(coordinates: &[f64], closed: bool) -> Result<Vec<f64>> {
    let point_count = coordinates.len() / 2;
    let segment_count = if closed { point_count } else { point_count - 1 };
    let mut distances = Vec::with_capacity(segment_count + 1);

    distances.push(0.0);

    for index in 0..segment_count {
        let start = position_at(coordinates, index);
        let end = position_at(coordinates, (index + 1) % point_count);
        let previous_distance = *distances.last().unwrap_or(&0.0);

        distances.push(previous_distance + distance(start, end));
    }

    Ok(distances)
}

struct PathSample {
    position: [f64; 2],
    segment_index: usize,
}

fn interpolate_along_path_from_segment(
    coordinates: &[f64],
    distances: &[f64],
    target_distance: f64,
    closed: bool,
    start_segment_index: usize,
) -> Result<PathSample> {
    let point_count = coordinates.len() / 2;
    let segment_count = if closed { point_count } else { point_count - 1 };
    let target_distance =
        NumberRange::new(0.0, *distances.last().unwrap_or(&0.0))?.clamp(target_distance)?;

    for index in start_segment_index..segment_count {
        let segment_start_distance = distances[index];
        let segment_end_distance = distances[index + 1];

        if target_distance > segment_end_distance {
            continue;
        }

        let start = position_at(coordinates, index);
        let end = position_at(coordinates, (index + 1) % point_count);
        let segment_length = segment_end_distance - segment_start_distance;
        let progress = if segment_length == 0.0 {
            0.0
        } else {
            (target_distance - segment_start_distance) / segment_length
        };

        return Ok(PathSample {
            position: interpolate_position(start, end, progress),
            segment_index: index,
        });
    }

    Ok(PathSample {
        position: position_at(coordinates, point_count - 1),
        segment_index: segment_count.saturating_sub(1),
    })
}

fn repeat_position(coordinates: &[f64], coordinate_count: usize) -> Vec<f64> {
    let position = &coordinates[0..2];
    let mut samples = Vec::with_capacity(coordinate_count * 2);

    for _ in 0..coordinate_count {
        samples.extend_from_slice(position);
    }

    samples
}

fn position_at(coordinates: &[f64], index: usize) -> [f64; 2] {
    let offset = index * 2;

    [coordinates[offset], coordinates[offset + 1]]
}

fn interpolate_position(start: [f64; 2], end: [f64; 2], progress: f64) -> [f64; 2] {
    [
        start[0] + (end[0] - start[0]) * progress,
        start[1] + (end[1] - start[1]) * progress,
    ]
}

fn distance(start: [f64; 2], end: [f64; 2]) -> f64 {
    (end[0] - start[0]).hypot(end[1] - start[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_one_segment_line() {
        let samples = resample_line_flat(&[0.0, 0.0, 10.0, 0.0], 3).unwrap();

        assert_eq!(samples, vec![0.0, 0.0, 5.0, 0.0, 10.0, 0.0]);
    }

    #[test]
    fn resamples_ring_without_closing_it() {
        let samples = resample_ring_flat(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 4).unwrap();

        assert_eq!(samples, vec![0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0]);
    }

    #[test]
    fn repeats_zero_distance_line() {
        let samples = resample_line_flat(&[3.0, 4.0, 3.0, 4.0], 4).unwrap();

        assert_eq!(samples, vec![3.0, 4.0, 3.0, 4.0, 3.0, 4.0, 3.0, 4.0]);
    }

    #[test]
    fn repeats_zero_distance_ring() {
        let samples = resample_ring_flat(&[1.0, 2.0, 1.0, 2.0, 1.0, 2.0], 3).unwrap();

        assert_eq!(samples, vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0]);
    }

    #[test]
    fn rejects_odd_length_coordinates() {
        assert!(resample_line_flat(&[0.0, 0.0, 1.0], 2).is_err());
    }

    #[test]
    fn rejects_non_finite_coordinates() {
        assert!(resample_ring_flat(&[0.0, 0.0, f64::NAN, 0.0, 1.0, 1.0], 3).is_err());
    }

    #[test]
    fn rejects_invalid_coordinate_count() {
        assert!(resample_line_flat(&[0.0, 0.0, 1.0, 1.0], 1).is_err());
        assert!(resample_ring_flat(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0], 2).is_err());
    }
}
