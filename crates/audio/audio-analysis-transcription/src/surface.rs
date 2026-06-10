//! Library-owned runtime surface for `audio-analysis-transcription`.

use runtime_core::{
    structured_surface_response, MobileCapability, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;

use crate::{
    candle_whisper_provider_plan, import_whisperx_json, transcribe, transcription_provider_plans,
    whisper_cpp_provider_plan, whisperx_provider_plan, AlignmentOptions, CandleWhisperOptions,
    DiarizationOptions, NativeDevicePreference, TranscriptionPipelineRequest,
    TranscriptionProviderSelection, TranscriptionSource, VadOptions, WhisperXCommandOptions,
    WhisperXDevice,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities {
            native: true,
            server: true,
            wasm: false,
            mobile: MobileCapability::Unsupported,
            requirements: vec![
                runtime_core::RuntimeRequirement {
                    name: "candle-whisper-model-bundle".to_string(),
                    description: Some(
                        "Required for native Candle Whisper ASR execution.".to_string(),
                    ),
                    required: false,
                },
                runtime_core::RuntimeRequirement {
                    name: "cuda".to_string(),
                    description: Some(
                        "Optional optimized device path when built with the cuda feature."
                            .to_string(),
                    ),
                    required: false,
                },
                runtime_core::RuntimeRequirement {
                    name: "whisperx".to_string(),
                    description: Some(
                        "Optional Python command for explicit external compatibility runs."
                            .to_string(),
                    ),
                    required: false,
                },
            ],
            max_recommended_input_bytes: None,
        },
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Rust-native audio and video transcription orchestration for video-analysis.",
                serde_json::json!({"includeOperations": true}),
                true,
            ),
            operation(
                "audio.transcription.transcribe",
                "Transcribe audio or video",
                "Runs transcription through native providers when built with explicit features and local bundles; WhisperX remains an external compatibility path.",
                serde_json::json!({
                    "source": {"path": "{\"segments\":[{\"start\":0.0,\"end\":1.0,\"text\":\"Hello from offline compatibility output.\"}]}"},
                    "provider": {
                        "kind": "externalWhisperX",
                        "command": "/usr/bin/printf",
                        "model": "mock-whisperx-json",
                        "device": "cpu"
                    },
                    "vad": {"enabled": true},
                    "alignment": {"enabled": false},
                    "diarization": {"enabled": false},
                    "output": {"formats": ["json", "srt", "webvtt"]}
                }),
                false,
            ),
            operation(
                "audio.transcription.importWhisperX",
                "Import WhisperX JSON",
                "Parses existing WhisperX JSON output through text-transcripts without running models.",
                serde_json::json!({"content": "{\"segments\":[{\"start\":0.0,\"end\":1.0,\"text\":\"Hello.\"}]}"}),
                true,
            ),
            operation(
                "audio.transcription.providers",
                "Inspect transcription providers",
                "Lists native and compatibility transcription providers and runtime constraints.",
                serde_json::json!({"includeExternal": true}),
                true,
            ),
            operation(
                "audio.transcription.plan",
                "Plan transcription runtime",
                "Explains the native transcription runtime without running models.",
                serde_json::json!({"provider": {"kind": "candleWhisper", "modelId": "openai/whisper-large-v3-turbo"}}),
                true,
            ),
            operation(
                "audio.transcription.modelPlan",
                "Plan ASR model",
                "Explains Candle Whisper and compatibility model requirements.",
                serde_json::json!({"provider": "candle-whisper"}),
                true,
            ),
            operation(
                "audio.transcription.vadPlan",
                "Plan VAD",
                "Explains deterministic energy VAD chunking defaults.",
                serde_json::json!({"vad": {"enabled": true}}),
                true,
            ),
            operation(
                "audio.transcription.alignmentPlan",
                "Plan alignment",
                "Explains deterministic CTC alignment and opt-in wav2vec2 bundle requirements.",
                serde_json::json!({"alignment": {"enabled": true, "modelId": "facebook/wav2vec2-base-960h"}}),
                true,
            ),
            operation(
                "audio.transcription.decodePlan",
                "Plan audio decode",
                "Explains whether a transcription source uses direct samples, native WAV loading, opt-in audio-io media decode, or external WhisperX compatibility.",
                serde_json::json!({"source": {"path": "clip.mp4"}, "provider": {"kind": "candleWhisper"}}),
                true,
            ),
            operation(
                "audio.transcription.diarizationPlan",
                "Plan diarization",
                "Explains current heuristic native diarization status and future model-backed provider options.",
                serde_json::json!({"diarization": {"enabled": true, "assignmentPolicy": "majority"}}),
                true,
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
    wasm_supported: bool,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "audio.transcription.transcribe" => transcribe_value(request.input)?,
        "audio.transcription.importWhisperX" => import_whisperx_value(parse_input(request.input)?)?,
        "audio.transcription.providers" => providers_value(request.input),
        "audio.transcription.plan" => plan_value(request.input),
        "audio.transcription.modelPlan" => model_plan_value(request.input),
        "audio.transcription.vadPlan" => vad_plan_value(request.input),
        "audio.transcription.alignmentPlan" => alignment_plan_value(request.input),
        "audio.transcription.decodePlan" => decode_plan_value(request.input),
        "audio.transcription.diarizationPlan" => diarization_plan_value(request.input),
        operation => {
            return Err(runtime_core::SurfaceError::unsupported_operation(
                operation,
                env!("CARGO_PKG_NAME"),
            )
            .to_error_string())
        }
    };
    Ok(response(operation, value))
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    let (title, message, summary) = match operation.as_str() {
        "describe" => (
            "Transcription package metadata",
            "Inspected native transcription operations and runtime support.",
            serde_json::json!({
                "operationCount": value.get("operationCount").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "audio.transcription.transcribe" => (
            "Audio transcription result",
            "Ran audio/video-to-text through the selected provider.",
            serde_json::json!({
                "provider": value.get("provider").cloned().unwrap_or(serde_json::Value::Null),
                "modelId": value.get("modelId").cloned().unwrap_or(serde_json::Value::Null),
                "segmentCount": value.pointer("/transcript/segments").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.transcription.importWhisperX" => (
            "WhisperX import result",
            "Parsed existing WhisperX JSON through text-transcripts.",
            serde_json::json!({
                "segmentCount": value.get("segments").and_then(serde_json::Value::as_array).map_or(0, Vec::len),
                "hasText": value.get("text").and_then(serde_json::Value::as_str).map(|text| !text.is_empty()).unwrap_or(false)
            }),
        ),
        "audio.transcription.providers" => (
            "Transcription providers",
            "Inspected native and compatibility transcription provider support.",
            serde_json::json!({
                "providerCount": value.get("providers").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.transcription.plan"
        | "audio.transcription.modelPlan"
        | "audio.transcription.vadPlan"
        | "audio.transcription.alignmentPlan"
        | "audio.transcription.decodePlan"
        | "audio.transcription.diarizationPlan" => (
            "Transcription runtime plan",
            "Planned transcription setup without execution.",
            serde_json::json!({
                "defaultProvider": value.get("defaultProvider").cloned().unwrap_or(serde_json::Value::Null),
                "normalizationOwner": value.get("normalizationOwner").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        _ => (
            "Transcription operation result",
            "Completed the transcription package operation.",
            serde_json::json!({}),
        ),
    };
    structured_surface_response(operation, title, message, summary, value)
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

fn transcribe_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request: TranscriptionPipelineRequest =
        serde_json::from_value(input).map_err(|error| error.to_string())?;
    let response = transcribe(request).map_err(|error| error.to_string())?;
    Ok(serde_json::json!(response))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportContentRequest {
    content: String,
}

fn import_whisperx_value(request: ImportContentRequest) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!(import_whisperx_json(
        request.content.as_bytes()
    )
    .map_err(|error| error.to_string())?))
}

fn providers_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "input": input,
        "providers": transcription_provider_plans().into_iter().map(|plan| {
            serde_json::json!({
                "id": plan.provider_id,
                "externalRuntime": plan.external_runtime,
                "wasmSupported": plan.wasm_supported,
                "primary": plan.primary,
                "setup": plan.setup,
                "diagnostics": plan.diagnostics,
            })
        }).collect::<Vec<_>>()
    })
}

fn plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "operation": "audio.transcription.transcribe",
        "defaultProvider": "candle-whisper",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "vadProvider": "energy-vad",
        "alignmentProvider": "ctc-forced-aligner",
        "diarizationProvider": "audio-analysis-speakers-native-baseline",
        "providers": transcription_provider_plans(),
        "input": input
    })
}

fn model_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "candle-whisper",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "asr": candle_whisper_provider_plan(),
        "compatibility": [whisper_cpp_provider_plan(), whisperx_provider_plan()],
        "models": [
            "openai/whisper-large-v3",
            "openai/whisper-large-v3-turbo",
            "facebook/wav2vec2-base-960h",
            "pyannote/speaker-diarization-3.1"
        ],
        "input": input
    })
}

fn vad_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "energy-vad",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "options": VadOptions::default(),
        "input": input
    })
}

fn alignment_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "candle-whisper",
        "provider": "ctc-forced-aligner",
        "modelId": "facebook/wav2vec2-base-960h",
        "normalizationOwner": "moritzbrantner-text-transcripts",
        "requiresFeature": "alignment",
        "input": input
    })
}

fn decode_plan_value(input: serde_json::Value) -> serde_json::Value {
    let provider_kind = input
        .pointer("/provider/kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("candleWhisper");
    let source = input.get("source").unwrap_or(&input);
    let audio_io_enabled = cfg!(feature = "audio-io");
    let plan = if provider_kind == "externalWhisperX" || provider_kind == "whisperx" {
        serde_json::json!({
            "sourceKind": source_kind(source),
            "decodePath": "external-whisperx-compatibility",
            "opensFiles": false,
            "executesFfmpeg": false,
            "featureGated": false,
            "notes": "External WhisperX compatibility owns media/container decode for this provider."
        })
    } else if source.get("samples").is_some() {
        serde_json::json!({
            "sourceKind": "samples",
            "decodePath": "direct-samples",
            "opensFiles": false,
            "executesFfmpeg": false,
            "featureGated": false,
            "normalization": "normalize_samples_source"
        })
    } else if let Some(path) = source.get("path").and_then(serde_json::Value::as_str) {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if extension == "wav" {
            serde_json::json!({
                "sourceKind": "path",
                "pathExtension": extension,
                "decodePath": "native-wav-reader",
                "opensFiles": false,
                "executesFfmpeg": false,
                "featureGated": false,
                "normalization": "native mono mix and resample to 16 kHz"
            })
        } else {
            serde_json::json!({
                "sourceKind": "path",
                "pathExtension": extension,
                "decodePath": if audio_io_enabled { "audio-io-media-decode" } else { "unsupported-runtime-without-audio-io" },
                "opensFiles": false,
                "executesFfmpeg": false,
                "featureGated": true,
                "requiredFeature": "audio-io",
                "audioIoFeatureEnabled": audio_io_enabled,
                "normalization": if audio_io_enabled { "audio-io mono decode then normalize_samples_source to 16 kHz" } else { "not available" }
            })
        }
    } else {
        serde_json::json!({
            "sourceKind": source_kind(source),
            "decodePath": "unknown-source",
            "opensFiles": false,
            "executesFfmpeg": false,
            "featureGated": false,
            "notes": "Provide source.samples or source.path for a concrete decode plan."
        })
    };
    serde_json::json!({
        "defaultProvider": "candle-whisper",
        "normalizationOwner": "moritzbrantner-audio-analysis-transcription",
        "defaultNativeBoundary": "wav-or-direct-samples",
        "audioIoFeatureEnabled": audio_io_enabled,
        "plan": plan,
        "input": input
    })
}

fn diarization_plan_value(input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "defaultProvider": "native-speaker-diarization",
        "currentRuntime": "heuristic-native",
        "productionParity": false,
        "assignmentPolicies": ["majority", "nearestStart", "strictContained"],
        "speakerBounds": {
            "minSpeakers": "validated and reported only",
            "maxSpeakers": "validated and reported only",
            "enforcedAsClusteringConstraints": false
        },
        "futureProviders": [
            "onnx-speaker-embedding",
            "pyannote-style-speaker-embedding",
            "external-pyannote-compatibility"
        ],
        "input": input
    })
}

fn source_kind(source: &serde_json::Value) -> &'static str {
    if source.get("samples").is_some() {
        "samples"
    } else if source.get("path").is_some() {
        "path"
    } else {
        "unknown"
    }
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    runtime_core::parse_surface_input(None, input)
}

/// Builds a default native Candle Whisper request for local callers.
pub fn default_native_request(path: impl Into<std::path::PathBuf>) -> TranscriptionPipelineRequest {
    TranscriptionPipelineRequest {
        source: TranscriptionSource::Path { path: path.into() },
        provider: TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions::default()),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    }
}

/// Builds a default WhisperX compatibility request for local callers.
pub fn default_whisperx_request(
    path: impl Into<std::path::PathBuf>,
) -> TranscriptionPipelineRequest {
    TranscriptionPipelineRequest {
        source: TranscriptionSource::Path { path: path.into() },
        provider: TranscriptionProviderSelection::ExternalWhisperX(WhisperXCommandOptions {
            device: WhisperXDevice::Cpu,
            compute_type: Some("int8".to_string()),
            ..WhisperXCommandOptions::default()
        }),
        vad: VadOptions::default(),
        alignment: AlignmentOptions::default(),
        diarization: DiarizationOptions::default(),
        output: Default::default(),
    }
}

/// Builds a CUDA-preferring native request.
pub fn cuda_native_request(path: impl Into<std::path::PathBuf>) -> TranscriptionPipelineRequest {
    let mut request = default_native_request(path);
    request.provider = TranscriptionProviderSelection::CandleWhisper(CandleWhisperOptions {
        device: NativeDevicePreference::Cuda,
        ..CandleWhisperOptions::default()
    });
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_transcription_operations() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"audio.transcription.transcribe"));
        assert!(ids.contains(&"audio.transcription.importWhisperX"));
        assert!(ids.contains(&"audio.transcription.providers"));
        assert!(ids.contains(&"audio.transcription.plan"));
        assert!(ids.contains(&"audio.transcription.modelPlan"));
        assert!(ids.contains(&"audio.transcription.vadPlan"));
        assert!(ids.contains(&"audio.transcription.alignmentPlan"));
        assert!(ids.contains(&"audio.transcription.decodePlan"));
        assert!(ids.contains(&"audio.transcription.diarizationPlan"));
    }

    #[test]
    fn import_whisperx_operation_returns_transcript() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.transcription.importWhisperX"),
            input: serde_json::json!({
                "content": "{\"segments\":[{\"start\":0.0,\"end\":1.0,\"text\":\"hello\"}]}"
            }),
        })
        .expect("import");
        assert_eq!(
            response.value["operation"],
            "audio.transcription.importWhisperX"
        );
        assert_eq!(
            response.value["result"]["segments"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn plan_reports_candle_as_primary_native_runtime() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.transcription.plan"),
            input: serde_json::json!({}),
        })
        .expect("plan");
        assert_eq!(
            response.value["result"]["defaultProvider"],
            "candle-whisper"
        );
        assert_eq!(
            response.value["result"]["normalizationOwner"],
            "moritzbrantner-text-transcripts"
        );
    }

    #[test]
    fn decode_plan_reports_non_wav_audio_io_boundary_without_opening_files() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.transcription.decodePlan"),
            input: serde_json::json!({"source": {"path": "clip.mp4"}}),
        })
        .expect("decode plan");
        assert_eq!(
            response.value["result"]["plan"]["decodePath"],
            if cfg!(feature = "audio-io") {
                "audio-io-media-decode"
            } else {
                "unsupported-runtime-without-audio-io"
            }
        );
        assert_eq!(response.value["result"]["plan"]["opensFiles"], false);
        assert_eq!(response.value["result"]["plan"]["executesFfmpeg"], false);
    }

    #[test]
    fn diarization_plan_reports_heuristic_status_without_model_parity_claim() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("audio.transcription.diarizationPlan"),
            input: serde_json::json!({}),
        })
        .expect("diarization plan");
        assert_eq!(
            response.value["result"]["currentRuntime"],
            "heuristic-native"
        );
        assert_eq!(response.value["result"]["productionParity"], false);
        assert_eq!(
            response.value["result"]["speakerBounds"]["enforcedAsClusteringConstraints"],
            false
        );
    }
}
