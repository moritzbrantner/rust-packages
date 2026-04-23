#![doc = include_str!("../README.md")]

use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationFidelity {
    Exact,
    Preserved,
    Quantized,
    Estimated,
    Interpolated,
    Heuristic,
    Placeholder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InversionMethod {
    Preserved,
    Defaulted,
    Quantized,
    Inferred,
    Interpolated,
    Template,
    Omitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InversionNote {
    pub field: String,
    pub method: InversionMethod,
    pub message: String,
}

impl InversionNote {
    pub fn new(
        field: impl Into<String>,
        method: InversionMethod,
        message: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            method,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InversionTrace {
    pub source_type: String,
    pub target_type: String,
    pub fidelity: InformationFidelity,
    pub confidence: f32,
    pub assumptions: Vec<String>,
    pub notes: Vec<InversionNote>,
}

impl InversionTrace {
    pub fn new(
        source_type: impl Into<String>,
        target_type: impl Into<String>,
        fidelity: InformationFidelity,
    ) -> Self {
        Self {
            source_type: source_type.into(),
            target_type: target_type.into(),
            fidelity,
            confidence: default_confidence(fidelity),
            assumptions: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn confidence(mut self, confidence: f32) -> Result<Self> {
        validate_confidence(confidence)?;
        self.confidence = confidence;
        Ok(self)
    }

    pub fn assumption(mut self, assumption: impl Into<String>) -> Self {
        self.assumptions.push(assumption.into());
        self
    }

    pub fn note(
        mut self,
        field: impl Into<String>,
        method: InversionMethod,
        message: impl Into<String>,
    ) -> Self {
        self.notes.push(InversionNote::new(field, method, message));
        self
    }

    pub fn merge_notes(&mut self, other: &InversionTrace) {
        self.assumptions.extend(other.assumptions.iter().cloned());
        self.notes.extend(other.notes.iter().cloned());
        self.confidence = self.confidence.min(other.confidence);
        self.fidelity = weaker_fidelity(self.fidelity, other.fidelity);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Generated<T> {
    pub value: T,
    pub trace: InversionTrace,
}

impl<T> Generated<T> {
    pub fn new(value: T, trace: InversionTrace) -> Self {
        Self { value, trace }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Generated<U> {
        Generated {
            value: f(self.value),
            trace: self.trace,
        }
    }
}

pub fn validate_confidence(confidence: f32) -> Result<()> {
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err(DetectError::InvalidArgument(
            "inversion confidence must be finite and between 0 and 1".to_string(),
        ));
    }
    Ok(())
}

pub fn weaker_fidelity(
    left: InformationFidelity,
    right: InformationFidelity,
) -> InformationFidelity {
    if fidelity_rank(left) >= fidelity_rank(right) {
        left
    } else {
        right
    }
}

fn fidelity_rank(fidelity: InformationFidelity) -> u8 {
    match fidelity {
        InformationFidelity::Exact => 0,
        InformationFidelity::Preserved => 1,
        InformationFidelity::Quantized => 2,
        InformationFidelity::Estimated => 3,
        InformationFidelity::Interpolated => 4,
        InformationFidelity::Heuristic => 5,
        InformationFidelity::Placeholder => 6,
    }
}

fn default_confidence(fidelity: InformationFidelity) -> f32 {
    match fidelity {
        InformationFidelity::Exact => 1.0,
        InformationFidelity::Preserved => 0.95,
        InformationFidelity::Quantized => 0.8,
        InformationFidelity::Estimated => 0.65,
        InformationFidelity::Interpolated => 0.55,
        InformationFidelity::Heuristic => 0.35,
        InformationFidelity::Placeholder => 0.15,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weaker_fidelity_keeps_more_lossy_value() {
        assert_eq!(
            weaker_fidelity(
                InformationFidelity::Preserved,
                InformationFidelity::Heuristic
            ),
            InformationFidelity::Heuristic
        );
    }

    #[test]
    fn rejects_invalid_confidence() {
        assert!(validate_confidence(1.1).is_err());
        assert!(validate_confidence(f32::NAN).is_err());
    }
}
