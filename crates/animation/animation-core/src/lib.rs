#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use three_d_processing_core::{Quaternion, Transform3, Vector3};
use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
/// Data type for animation time in seconds.
pub struct TimeSeconds(pub f32);

impl TimeSeconds {
    /// Creates a new value.
    pub fn new(seconds: f32) -> Result<Self> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(invalid_argument(
                "animation time must be finite and non-negative",
            ));
        }
        Ok(Self(seconds))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Variants describing interpolation.
pub enum Interpolation {
    /// Hold the previous keyframe value.
    Step,
    /// Interpolate linearly.
    Linear,
    /// Smooth interpolation placeholder for formats that distinguish cubic data.
    Cubic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for keyframe.
pub struct Keyframe<T> {
    /// Time in seconds.
    pub time: TimeSeconds,
    /// Sample value.
    pub value: T,
}

impl<T> Keyframe<T> {
    /// Creates a new value.
    pub fn new(time_seconds: f32, value: T) -> Result<Self> {
        Ok(Self {
            time: TimeSeconds::new(time_seconds)?,
            value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for track.
pub struct Track<T> {
    /// Human-readable track target or channel name.
    pub name: String,
    /// Ordered keyframes.
    pub keyframes: Vec<Keyframe<T>>,
    /// Interpolation mode.
    pub interpolation: Interpolation,
}

impl<T> Track<T> {
    /// Creates a new value.
    pub fn new(
        name: impl Into<String>,
        keyframes: impl Into<Vec<Keyframe<T>>>,
        interpolation: Interpolation,
    ) -> Result<Self> {
        let track = Self {
            name: name.into(),
            keyframes: keyframes.into(),
            interpolation,
        };
        track.validate()?;
        Ok(track)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(invalid_argument("track name must not be empty"));
        }
        let mut previous = None;
        for keyframe in &self.keyframes {
            if let Some(previous) = previous {
                if keyframe.time.0 <= previous {
                    return Err(invalid_argument(
                        "track keyframes must be strictly ordered by time",
                    ));
                }
            }
            previous = Some(keyframe.time.0);
        }
        Ok(())
    }

    /// Returns duration.
    pub fn duration(&self) -> Option<TimeSeconds> {
        self.keyframes.last().map(|keyframe| keyframe.time)
    }

    fn sample_pair(&self, time_seconds: f32) -> Result<Option<SamplePair<'_, T>>> {
        self.validate()?;
        if !time_seconds.is_finite() || time_seconds < 0.0 {
            return Err(invalid_argument(
                "sample time must be finite and non-negative",
            ));
        }
        let Some(first) = self.keyframes.first() else {
            return Ok(None);
        };
        if time_seconds <= first.time.0 {
            return Ok(Some(SamplePair::Single(first)));
        }
        let Some(last) = self.keyframes.last() else {
            return Ok(None);
        };
        if time_seconds >= last.time.0 {
            return Ok(Some(SamplePair::Single(last)));
        }
        for window in self.keyframes.windows(2) {
            let left = &window[0];
            let right = &window[1];
            if time_seconds >= left.time.0 && time_seconds <= right.time.0 {
                let span = right.time.0 - left.time.0;
                let t = if span <= f32::EPSILON {
                    0.0
                } else {
                    (time_seconds - left.time.0) / span
                };
                return Ok(Some(SamplePair::Pair { left, right, t }));
            }
        }
        Ok(None)
    }
}

enum SamplePair<'a, T> {
    Single(&'a Keyframe<T>),
    Pair {
        left: &'a Keyframe<T>,
        right: &'a Keyframe<T>,
        t: f32,
    },
}

impl Track<f32> {
    /// Samples this f32 track.
    pub fn sample_f32(&self, time_seconds: f32) -> Result<Option<f32>> {
        Ok(match self.sample_pair(time_seconds)? {
            Some(SamplePair::Single(keyframe)) => Some(keyframe.value),
            Some(SamplePair::Pair { left, right, t }) => match self.interpolation {
                Interpolation::Step => Some(left.value),
                Interpolation::Linear | Interpolation::Cubic => {
                    Some(left.value + (right.value - left.value) * t)
                }
            },
            None => None,
        })
    }
}

impl Track<Vector3> {
    /// Samples this vector track.
    pub fn sample_vector3(&self, time_seconds: f32) -> Result<Option<Vector3>> {
        Ok(match self.sample_pair(time_seconds)? {
            Some(SamplePair::Single(keyframe)) => Some(keyframe.value),
            Some(SamplePair::Pair { left, right, t }) => match self.interpolation {
                Interpolation::Step => Some(left.value),
                Interpolation::Linear | Interpolation::Cubic => {
                    Some(left.value + (right.value - left.value) * t)
                }
            },
            None => None,
        })
    }
}

impl Track<Quaternion> {
    /// Samples this quaternion track.
    pub fn sample_quaternion(&self, time_seconds: f32) -> Result<Option<Quaternion>> {
        Ok(match self.sample_pair(time_seconds)? {
            Some(SamplePair::Single(keyframe)) => Some(keyframe.value.normalize()?),
            Some(SamplePair::Pair { left, right, t }) => match self.interpolation {
                Interpolation::Step => Some(left.value.normalize()?),
                Interpolation::Linear | Interpolation::Cubic => {
                    Some(left.value.slerp(right.value, t)?)
                }
            },
            None => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for transform track.
pub struct TransformTrack {
    /// Translation track.
    pub translation: Option<Track<Vector3>>,
    /// Rotation track.
    pub rotation: Option<Track<Quaternion>>,
    /// Uniform scale track.
    pub scale: Option<Track<f32>>,
}

impl TransformTrack {
    /// Creates a new value.
    pub fn new(
        translation: Option<Track<Vector3>>,
        rotation: Option<Track<Quaternion>>,
        scale: Option<Track<f32>>,
    ) -> Result<Self> {
        let track = Self {
            translation,
            rotation,
            scale,
        };
        track.validate()?;
        Ok(track)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if let Some(track) = &self.translation {
            track.validate()?;
        }
        if let Some(track) = &self.rotation {
            track.validate()?;
        }
        if let Some(track) = &self.scale {
            track.validate()?;
        }
        Ok(())
    }

    /// Samples this value as a uniform-scale transform.
    pub fn sample_transform3(&self, time_seconds: f32) -> Result<Transform3> {
        self.validate()?;
        let translation = self
            .translation
            .as_ref()
            .map(|track| track.sample_vector3(time_seconds))
            .transpose()?
            .flatten()
            .unwrap_or(Vector3::ZERO);
        let scale = self
            .scale
            .as_ref()
            .map(|track| track.sample_f32(time_seconds))
            .transpose()?
            .flatten()
            .unwrap_or(1.0);
        Transform3::new(translation, scale)
    }

    /// Samples this value as rotation and uniform-scale transform parts.
    pub fn sample_rotation_and_transform3(
        &self,
        time_seconds: f32,
    ) -> Result<(Quaternion, Transform3)> {
        let rotation = self
            .rotation
            .as_ref()
            .map(|track| track.sample_quaternion(time_seconds))
            .transpose()?
            .flatten()
            .unwrap_or(Quaternion::IDENTITY);
        Ok((rotation, self.sample_transform3(time_seconds)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for joint.
pub struct Joint {
    /// Human-readable joint name.
    pub name: String,
    /// Optional parent joint index.
    pub parent: Option<usize>,
}

impl Joint {
    /// Creates a new value.
    pub fn new(name: impl Into<String>, parent: Option<usize>) -> Result<Self> {
        let joint = Self {
            name: name.into(),
            parent,
        };
        joint.validate(usize::MAX)?;
        Ok(joint)
    }

    fn validate(&self, own_index: usize) -> Result<()> {
        if self.name.is_empty() {
            return Err(invalid_argument("joint name must not be empty"));
        }
        if self.parent == Some(own_index) {
            return Err(invalid_argument("joint cannot parent itself"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Data type for skeleton.
pub struct Skeleton {
    /// Ordered joints.
    pub joints: Vec<Joint>,
}

impl Skeleton {
    /// Creates a new value.
    pub fn new(joints: impl Into<Vec<Joint>>) -> Result<Self> {
        let skeleton = Self {
            joints: joints.into(),
        };
        skeleton.validate()?;
        Ok(skeleton)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        for (index, joint) in self.joints.iter().enumerate() {
            joint.validate(index)?;
            if let Some(parent) = joint.parent {
                if parent >= self.joints.len() {
                    return Err(invalid_argument("joint parent index is out of bounds"));
                }
                if parent >= index {
                    return Err(invalid_argument(
                        "joint parents must appear before their children",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Returns joint index by name.
    pub fn joint_index(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|joint| joint.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Data type for animation clip.
pub struct AnimationClip {
    /// Human-readable clip name.
    pub name: String,
    /// Transform tracks keyed by target name.
    pub transform_tracks: Vec<(String, TransformTrack)>,
}

impl AnimationClip {
    /// Creates a new value.
    pub fn new(
        name: impl Into<String>,
        transform_tracks: impl Into<Vec<(String, TransformTrack)>>,
    ) -> Result<Self> {
        let clip = Self {
            name: name.into(),
            transform_tracks: transform_tracks.into(),
        };
        clip.validate()?;
        Ok(clip)
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(invalid_argument("animation clip name must not be empty"));
        }
        for (target, track) in &self.transform_tracks {
            if target.is_empty() {
                return Err(invalid_argument("animation target name must not be empty"));
            }
            track.validate()?;
        }
        Ok(())
    }

    /// Returns duration.
    pub fn duration(&self) -> Option<TimeSeconds> {
        self.transform_tracks
            .iter()
            .flat_map(|(_, track)| {
                [
                    track.translation.as_ref().and_then(Track::duration),
                    track.rotation.as_ref().and_then(Track::duration),
                    track.scale.as_ref().and_then(Track::duration),
                ]
            })
            .flatten()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    }
}

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_and_vector_tracks_sample_linearly() {
        let opacity = Track::new(
            "opacity",
            [
                Keyframe::new(0.0, 0.0).unwrap(),
                Keyframe::new(1.0, 1.0).unwrap(),
            ],
            Interpolation::Linear,
        )
        .unwrap();
        assert_eq!(opacity.sample_f32(0.5).unwrap(), Some(0.5));

        let position = Track::new(
            "translation",
            [
                Keyframe::new(0.0, Vector3::ZERO).unwrap(),
                Keyframe::new(1.0, Vector3::new(2.0, 0.0, 0.0)).unwrap(),
            ],
            Interpolation::Linear,
        )
        .unwrap();
        assert_eq!(
            position.sample_vector3(0.25).unwrap(),
            Some(Vector3::new(0.5, 0.0, 0.0))
        );
    }

    #[test]
    fn transform_tracks_sample_defaults_and_clip_duration() {
        let translation = Track::new(
            "translation",
            [
                Keyframe::new(0.0, Vector3::ZERO).unwrap(),
                Keyframe::new(2.0, Vector3::new(2.0, 0.0, 0.0)).unwrap(),
            ],
            Interpolation::Linear,
        )
        .unwrap();
        let transform_track = TransformTrack::new(Some(translation), None, None).unwrap();
        let transform = transform_track.sample_transform3(1.0).unwrap();
        assert_eq!(transform.translation, Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(transform.scale, 1.0);

        let clip = AnimationClip::new("move", [("root".to_string(), transform_track)]).unwrap();
        assert_eq!(clip.duration(), Some(TimeSeconds(2.0)));
    }

    #[test]
    fn skeleton_requires_parent_before_child() {
        let skeleton = Skeleton::new([
            Joint::new("root", None).unwrap(),
            Joint::new("hand", Some(0)).unwrap(),
        ])
        .unwrap();
        assert_eq!(skeleton.joint_index("hand"), Some(1));
        assert!(Skeleton::new([Joint::new("bad", Some(1)).unwrap()]).is_err());
    }
}
