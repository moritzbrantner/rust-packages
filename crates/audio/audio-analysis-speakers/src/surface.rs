//! Library-owned runtime surface for `audio-analysis-speakers`.

use audio_analysis_recognition::SpectralEmbeddingConfig;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    assign_speakers_to_transcript, SpeakerAudio, SpeakerDiarizationResponse,
    SpeakerEmbeddingExtractor, SpeakerId, SpeakerIdentificationOptions, SpeakerLabel,
    SpeakerLibrary, SpectralSpeakerEmbedder, TranscriptionContract,
};

const MAX_SAMPLES: usize = 192_000;
const DEFAULT_PREVIEW_VALUES: usize = 32;

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Speaker embeddings, enrollment, identification, VAD, and diarization APIs for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "audio.speakers.embed",
                "Embed speaker",
                "Computes a deterministic spectral speaker embedding from normalized samples.",
                serde_json::json!({"samples": [0.0, 1.0, 0.0, -1.0], "sampleRate": 48000, "channels": 1}),
            ),
            operation(
                "audio.speakers.identify",
                "Identify speaker",
                "Builds a transient enrolled-speaker library and identifies a query embedding.",
                serde_json::json!({"querySamples": [0.0, 1.0, 0.0, -1.0], "sampleRate": 48000, "speakers": [{"id": "speaker-1", "label": "Speaker 1", "samples": [0.0, 1.0, 0.0, -1.0]}]}),
            ),
            operation(
                "audio.speakers.assignTranscript",
                "Assign transcript speakers",
                "Applies diarization segments to an existing transcription contract.",
                serde_json::json!({"transcript": {"segments": [{"index": 0, "text": "hello", "startSeconds": 0.0, "endSeconds": 1.0, "isFinal": true}]}, "diarization": {"accepted": true, "operation": "diarize", "modelId": "single-speaker-heuristic", "runtime": "deterministicFallback", "segments": [{"speaker": "speaker_0", "startSeconds": 0.0, "endSeconds": 1.0, "score": 1.0}]}}),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "audio.speakers.embed" => embed_value(request.input)?,
        "audio.speakers.identify" => identify_value(request.input)?,
        "audio.speakers.assignTranscript" => assign_transcript_value(request.input)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface.operations.iter().map(|operation| operation.id.as_str()).collect::<Vec<_>>(),
        "input": input
    })
}

fn embed_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let samples = sample_array(&input, "samples")?;
    let sample_rate = sample_rate(&input)?;
    let channels = channels(&input)?;
    let mut embedder = speaker_embedder(&input)?;
    let audio = SpeakerAudio::interleaved(&samples, sample_rate, channels)
        .map_err(|error| error.to_string())?;
    let embedding = embedder
        .embed_speaker(&audio)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "sampleRate": sample_rate,
        "channels": channels,
        "sampleCount": samples.len(),
        "durationSeconds": audio.duration_seconds(),
        "model": {
            "name": embedding.model().name,
            "version": embedding.model().version,
            "dimensions": embedding.dimensions()
        },
        "valuesPreview": embedding.values().iter().copied().take(DEFAULT_PREVIEW_VALUES).collect::<Vec<_>>()
    }))
}

fn identify_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let query = sample_array(&input, "querySamples")?;
    let sample_rate = sample_rate(&input)?;
    let channels = channels(&input)?;
    let mut embedder = speaker_embedder(&input)?;
    let query_audio = SpeakerAudio::interleaved(&query, sample_rate, channels)
        .map_err(|error| error.to_string())?;
    let query_embedding = embedder
        .embed_speaker(&query_audio)
        .map_err(|error| error.to_string())?;
    let mut library = SpeakerLibrary::new();
    let speakers = input
        .get("speakers")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "speakers must be an array".to_string())?;
    for speaker in speakers {
        let id = speaker
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "speaker id must be a string".to_string())?;
        let label = speaker
            .get("label")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(id);
        let samples = sample_array(speaker, "samples")?;
        let audio = SpeakerAudio::interleaved(&samples, sample_rate, channels)
            .map_err(|error| error.to_string())?;
        library
            .enroll(
                SpeakerId::new(id).map_err(|error| error.to_string())?,
                SpeakerLabel::new(label).map_err(|error| error.to_string())?,
                &audio,
                &mut embedder,
            )
            .map_err(|error| error.to_string())?;
    }
    let result = library
        .identify(&query_embedding, &SpeakerIdentificationOptions::default())
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "sampleRate": sample_rate,
        "speakerCount": library.len(),
        "unknown": result.unknown,
        "margin": result.margin,
        "bestMatch": result.best_match.as_ref().map(|matched| serde_json::json!({
            "speakerId": matched.speaker_id.as_str(),
            "label": matched.label.as_str(),
            "score": matched.score,
            "confidence": format!("{:?}", matched.confidence)
        })),
        "matches": result.ranked_matches.iter().map(|matched| serde_json::json!({
            "speakerId": matched.speaker_id.as_str(),
            "label": matched.label.as_str(),
            "score": matched.score,
            "margin": matched.margin,
            "confidence": format!("{:?}", matched.confidence)
        })).collect::<Vec<_>>()
    }))
}

fn assign_transcript_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let transcript: TranscriptionContract = serde_json::from_value(
        input
            .get("transcript")
            .cloned()
            .ok_or_else(|| "transcript is required".to_string())?,
    )
    .map_err(|error| format!("invalid transcript: {error}"))?;
    let diarization: SpeakerDiarizationResponse = serde_json::from_value(
        input
            .get("diarization")
            .cloned()
            .ok_or_else(|| "diarization is required".to_string())?,
    )
    .map_err(|error| format!("invalid diarization: {error}"))?;
    let assigned = assign_speakers_to_transcript(&transcript, &diarization)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "segmentCount": assigned.segments.len(),
        "transcript": assigned
    }))
}

fn speaker_embedder(input: &serde_json::Value) -> Result<SpectralSpeakerEmbedder, String> {
    let fft_size = positive_usize(input, "fftSize", 512)?;
    let hop_size = positive_usize(input, "hopSize", fft_size / 2)?;
    let bands = positive_usize(input, "bands", 8)?;
    SpectralSpeakerEmbedder::new(
        SpectralEmbeddingConfig::new(fft_size, hop_size, bands)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn sample_array(input: &serde_json::Value, field: &str) -> Result<Vec<f32>, String> {
    let values = input
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?;
    if values.len() > MAX_SAMPLES {
        return Err(format!(
            "{field} must not contain more than {MAX_SAMPLES} samples"
        ));
    }
    values
        .iter()
        .map(|value| {
            let sample = value
                .as_f64()
                .ok_or_else(|| format!("{field} must contain only numbers"))?
                as f32;
            if sample.is_finite() {
                Ok(sample)
            } else {
                Err(format!("{field} must contain only finite numbers"))
            }
        })
        .collect()
}

fn sample_rate(input: &serde_json::Value) -> Result<u32, String> {
    let value = input
        .get("sampleRate")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(48_000);
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "sampleRate must be a positive u32".to_string())
}

fn channels(input: &serde_json::Value) -> Result<u16, String> {
    let value = input
        .get("channels")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1);
    u16::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "channels must be a positive u16".to_string())
}

fn positive_usize(
    input: &serde_json::Value,
    field: &str,
    default_value: usize,
) -> Result<usize, String> {
    let value = input
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(default_value as u64);
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{field} must be positive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_speaker_operations() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"audio.speakers.embed"));
        assert!(ids.contains(&"audio.speakers.assignTranscript"));
    }

    #[test]
    fn embed_operation_returns_dimensions() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.speakers.embed"),
            input: serde_json::json!({"samples": [0.0, 1.0, 0.0, -1.0], "sampleRate": 4, "fftSize": 4, "hopSize": 2, "bands": 2}),
        })
        .expect("embed");
        assert!(response.value["model"]["dimensions"].as_u64().unwrap() > 0);
    }

    #[test]
    fn invalid_samples_return_error() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.speakers.embed"),
            input: serde_json::json!({"samples": "bad"}),
        })
        .unwrap_err();
        assert!(error.contains("samples"));
    }
}
