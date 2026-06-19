//! Library-owned runtime surface for `audio-generation-tts`.

use model_runtime::{ModelFileRequest, ModelPreset, ModelSpec};
use runtime_core::{
    set_surface_operation_curation, structured_surface_response, OperationId, PackageSurface,
    RuntimeCapabilities, SurfaceOperation, SurfaceOperationCuration, SurfaceRequest,
    SurfaceResponse,
};

use crate::{
    synthesize, PcmAudio, ReferenceVoicePrompt, SpeechSynthesisRequest, SpeechSynthesisStatus,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust().with_requirement(
            "native-tts-provider",
            "Native providers and model bundles are not implemented in this slice.",
            false,
        ),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Generic and speaker-conditioned TTS contracts, validation, and setup diagnostics.",
                serde_json::json!({"includeOperations": true}),
                SurfaceOperationCuration::debug(900),
            ),
            operation(
                "audio.tts.synthesize",
                "Synthesize speech",
                "Validates a TTS request and returns explicit setup diagnostics until native providers are implemented.",
                example_synthesis_request(),
                SurfaceOperationCuration::workflow(10).primary(),
            ),
            operation(
                "audio.tts.plan",
                "Preview synthesis plan",
                "Previews provider, runtime, and output requirements without synthesizing audio.",
                example_synthesis_request(),
                SurfaceOperationCuration::debug(910),
            ),
            operation(
                "audio.tts.models",
                "Inspect TTS models",
                "Inspects the current side-effect-free TTS model inventory.",
                serde_json::json!({}),
                SurfaceOperationCuration::debug(920),
            ),
            operation(
                "audio.tts.referencePromptPlan",
                "Inspect reference prompt plan",
                "Inspects Reference Voice Prompt readiness for speaker-conditioned TTS.",
                serde_json::json!({"referenceVoicePrompt": example_reference_prompt()}),
                SurfaceOperationCuration::debug(930),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
    curation: SurfaceOperationCuration,
) -> SurfaceOperation {
    let mut operation = runtime_core::surface_operation(id, name, description, example_request);
    set_surface_operation_curation(&mut operation, curation);
    operation
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "audio.tts.synthesize" => synthesize_value(request.input)?,
        "audio.tts.plan" => plan_value(request.input)?,
        "audio.tts.models" => models_value(),
        "audio.tts.referencePromptPlan" => reference_prompt_plan_value(request.input)?,
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
    let (title, message, summary) = match operation.as_str() {
        "describe" => (
            "TTS package metadata",
            "Inspected the generic and speaker-conditioned TTS operations exposed by this package.",
            serde_json::json!({
                "operationCount": value.get("operationCount").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "audio.tts.synthesize" => (
            "TTS synthesis setup result",
            "Validated the synthesis request and returned explicit setup diagnostics; no native audio was generated.",
            serde_json::json!({
                "status": value.get("status").cloned().unwrap_or(serde_json::Value::Null),
                "audioGenerated": value.get("audioGenerated").cloned().unwrap_or(serde_json::Value::Null),
                "diagnosticCount": value.get("diagnostics").and_then(serde_json::Value::as_array).map_or(0, Vec::len)
            }),
        ),
        "audio.tts.plan" => (
            "TTS synthesis plan",
            "Previewed provider, runtime, and output requirements without synthesizing audio.",
            serde_json::json!({
                "willSynthesize": value.get("willSynthesize").cloned().unwrap_or(serde_json::Value::Null),
                "status": value.get("status").cloned().unwrap_or(serde_json::Value::Null),
                "speakerConditioned": value.get("speakerConditioned").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "audio.tts.models" => (
            "TTS model inventory",
            "Inspected current TTS model inventory state without selecting or downloading a model.",
            serde_json::json!({
                "modelCount": value.get("models").and_then(serde_json::Value::as_array).map_or(0, Vec::len),
                "defaultModelSelected": value.get("defaultModelSelected").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        "audio.tts.referencePromptPlan" => (
            "Reference prompt plan",
            "Inspected Reference Voice Prompt readiness for speaker-conditioned TTS.",
            serde_json::json!({
                "provided": value.get("provided").cloned().unwrap_or(serde_json::Value::Null),
                "transcriptProvided": value.get("transcriptProvided").cloned().unwrap_or(serde_json::Value::Null),
                "action": value.get("action").cloned().unwrap_or(serde_json::Value::Null)
            }),
        ),
        _ => (
            "TTS operation result",
            "Completed the TTS package surface operation.",
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

fn synthesize_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = request_from_value(input)?;
    let output = synthesize(&request)?;
    Ok(serde_json::json!({
        "status": status_string(&output.status),
        "provider": output.provider,
        "audioGenerated": output.audio.is_some(),
        "audio": output.audio,
        "diagnostics": output.diagnostics,
        "plan": plan_for_request(&request, &output.status),
    }))
}

fn plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let request = request_from_value(input)?;
    request.validate()?;
    Ok(plan_for_request(
        &request,
        if request.provider.native {
            &SpeechSynthesisStatus::SetupRequired
        } else {
            &SpeechSynthesisStatus::UnsupportedRuntime
        },
    ))
}

fn models_value() -> serde_json::Value {
    let models = tts_model_presets()
        .into_iter()
        .map(tts_model_json)
        .collect::<Vec<_>>();
    serde_json::json!({
        "defaultModelSelected": false,
        "models": models,
        "nativeProvidersImplemented": false,
        "message": "No TTS model preset is selected by default. F5/E2/Vocos metadata is available for explicit opt-in planning.",
        "requirements": [
            {
                "id": "native-tts-provider",
                "requiredFor": ["audio.tts.synthesize"],
                "available": false
            }
        ]
    })
}

fn tts_model_presets() -> Vec<ModelPreset> {
    ModelPreset::ALL
        .iter()
        .copied()
        .filter(|preset| {
            matches!(
                preset,
                ModelPreset::F5TtsV1Base
                    | ModelPreset::F5TtsBase
                    | ModelPreset::E2TtsBase
                    | ModelPreset::VocosMel24Khz
            )
        })
        .collect()
}

fn tts_model_json(preset: ModelPreset) -> serde_json::Value {
    let spec = preset.spec();
    serde_json::json!({
        "id": preset.as_str(),
        "name": spec.name.as_str(),
        "displayName": spec.metadata.get("displayName"),
        "task": spec.task.as_protocol_str(),
        "repoId": spec.repo_id_value(),
        "revision": spec.revision_value(),
        "requiredFiles": required_files(&spec),
        "requestedFiles": file_requests_json(&spec.files),
        "license": license_json(&spec),
        "explicitOptIn": spec.metadata.get("explicitOptIn").is_some_and(|value| value == "true"),
        "metadata": spec.metadata,
        "runtime": {
            "downloadsModels": false,
            "runsInference": false,
            "sideEffects": []
        }
    })
}

fn required_files(spec: &ModelSpec) -> Vec<&str> {
    spec.files
        .iter()
        .filter_map(|request| match request {
            ModelFileRequest::Required(path) => Some(path.as_str()),
            ModelFileRequest::Optional(_) | ModelFileRequest::FirstAvailable(_) => None,
        })
        .collect()
}

fn file_requests_json(files: &[ModelFileRequest]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|request| match request {
            ModelFileRequest::Required(path) => {
                serde_json::json!({"kind": "required", "path": path})
            }
            ModelFileRequest::Optional(path) => {
                serde_json::json!({"kind": "optional", "path": path})
            }
            ModelFileRequest::FirstAvailable(paths) => {
                serde_json::json!({"kind": "firstAvailable", "paths": paths})
            }
        })
        .collect()
}

fn license_json(spec: &ModelSpec) -> serde_json::Value {
    serde_json::json!({
        "id": spec.metadata.get("license"),
        "name": spec.metadata.get("licenseName"),
        "url": spec.metadata.get("licenseUrl"),
        "scope": spec.metadata.get("licenseScope"),
    })
}

fn reference_prompt_plan_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let prompt = input
        .get("referenceVoicePrompt")
        .cloned()
        .or_else(|| input.get("referencePrompt").cloned());
    match prompt {
        Some(prompt) => {
            let prompt: ReferenceVoicePrompt = serde_json::from_value(prompt)
                .map_err(|error| format!("invalid request: referenceVoicePrompt {error}"))?;
            prompt.validate()?;
            Ok(reference_prompt_plan(Some(&prompt)))
        }
        None => Ok(reference_prompt_plan(None)),
    }
}

fn request_from_value(input: serde_json::Value) -> Result<SpeechSynthesisRequest, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn plan_for_request(
    request: &SpeechSynthesisRequest,
    status: &SpeechSynthesisStatus,
) -> serde_json::Value {
    serde_json::json!({
        "status": status_string(status),
        "willSynthesize": false,
        "speakerConditioned": request.is_speaker_conditioned(),
        "provider": request.provider,
        "requestedOutput": {
            "sampleRateHz": request.options.sample_rate_hz,
            "channels": request.options.channels,
            "format": "pcm-f32-interleaved"
        },
        "runtime": {
            "nativeProvidersImplemented": false,
            "downloadsModels": false,
            "runsInference": false,
            "sideEffects": []
        },
        "referencePrompt": reference_prompt_plan(request.reference_voice_prompt.as_ref()),
        "requirements": [
            {
                "id": "native-tts-provider",
                "available": false,
                "message": "Native providers are added by later slices."
            }
        ]
    })
}

fn reference_prompt_plan(prompt: Option<&ReferenceVoicePrompt>) -> serde_json::Value {
    match prompt {
        Some(prompt) => {
            let sample_count = prompt.audio.samples.len();
            serde_json::json!({
                "provided": true,
                "transcriptProvided": prompt.has_transcript(),
                "sampleRateHz": prompt.audio.sample_rate_hz,
                "channels": prompt.audio.channels,
                "sampleCount": sample_count,
                "action": if prompt.has_transcript() {
                    "readyForProviderValidation"
                } else {
                    "needsTranscriptOrAsrFallback"
                },
                "message": if prompt.has_transcript() {
                    "Reference Voice Prompt includes audio and transcript."
                } else {
                    "Reference Voice Prompt includes audio but no transcript; later slices add ASR fallback planning."
                }
            })
        }
        None => serde_json::json!({
            "provided": false,
            "transcriptProvided": false,
            "action": "notRequiredForGenericTts",
            "message": "No Reference Voice Prompt was supplied."
        }),
    }
}

fn status_string(status: &SpeechSynthesisStatus) -> &'static str {
    match status {
        SpeechSynthesisStatus::Ready => "ready",
        SpeechSynthesisStatus::SetupRequired => "setupRequired",
        SpeechSynthesisStatus::UnsupportedRuntime => "unsupportedRuntime",
    }
}

fn example_synthesis_request() -> serde_json::Value {
    serde_json::json!({
        "text": "Hello from the TTS package surface.",
        "provider": {
            "providerId": "generic",
            "native": false
        },
        "options": {
            "sampleRateHz": 24000,
            "channels": 1,
            "seed": 42,
            "speed": 1.0,
            "removeSilence": false
        }
    })
}

fn example_reference_prompt() -> serde_json::Value {
    serde_json::json!({
        "audio": example_pcm_audio(),
        "transcript": "Reference voice prompt text.",
        "metadata": {
            "source": "inline-example"
        }
    })
}

fn example_pcm_audio() -> PcmAudio {
    PcmAudio {
        sample_rate_hz: 24_000,
        channels: 1,
        samples: vec![0.0, 0.01, -0.01, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runtime_core::SurfaceOperationRole;

    #[test]
    fn package_surface_exposes_tts_operations_with_roles() {
        let surface = package_surface();
        let ids = surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            [
                "describe",
                "audio.tts.synthesize",
                "audio.tts.plan",
                "audio.tts.models",
                "audio.tts.referencePromptPlan"
            ]
        );
        let synthesize = surface
            .operations
            .iter()
            .find(|operation| operation.id.as_str() == "audio.tts.synthesize")
            .expect("synthesize");
        assert_eq!(synthesize.curation.role, SurfaceOperationRole::Workflow);
        assert!(synthesize.curation.primary);
        let plan = surface
            .operations
            .iter()
            .find(|operation| operation.id.as_str() == "audio.tts.plan")
            .expect("plan");
        assert_eq!(plan.curation.role, SurfaceOperationRole::Debug);
    }

    #[test]
    fn synthesize_surface_returns_setup_diagnostics() {
        let response = run_surface_operation(SurfaceRequest {
            operation: "audio.tts.synthesize".into(),
            input: example_synthesis_request(),
        })
        .expect("synthesize response");
        assert_eq!(response.value["result"]["status"], "unsupportedRuntime");
        assert_eq!(response.value["result"]["audioGenerated"], false);
        assert!(response.value["result"]["diagnostics"].is_array());
    }

    #[test]
    fn reference_prompt_plan_reports_missing_transcript() {
        let response = run_surface_operation(SurfaceRequest {
            operation: "audio.tts.referencePromptPlan".into(),
            input: serde_json::json!({
                "referenceVoicePrompt": {
                    "audio": example_pcm_audio()
                }
            }),
        })
        .expect("reference prompt plan");
        assert_eq!(
            response.value["result"]["action"],
            "needsTranscriptOrAsrFallback"
        );
    }

    #[test]
    fn models_surface_lists_explicit_tts_presets_with_license_metadata() {
        let response = run_surface_operation(SurfaceRequest {
            operation: "audio.tts.models".into(),
            input: serde_json::json!({}),
        })
        .expect("models response");

        assert_eq!(response.value["result"]["defaultModelSelected"], false);
        let models = response.value["result"]["models"]
            .as_array()
            .expect("models array");
        let ids = models
            .iter()
            .map(|model| model["id"].as_str().expect("model id"))
            .collect::<Vec<_>>();
        assert!(ids.contains(&"f5-tts-v1-base"));
        assert!(ids.contains(&"f5-tts-base"));
        assert!(ids.contains(&"e2-tts-base"));
        assert!(ids.contains(&"vocos-mel-24khz"));

        let f5 = models
            .iter()
            .find(|model| model["id"] == "f5-tts-v1-base")
            .expect("f5 v1 preset");
        assert_eq!(f5["repoId"], "SWivid/F5-TTS");
        assert_eq!(f5["task"], "speaker_conditioned_tts");
        assert_eq!(f5["license"]["id"], "cc-by-nc-4.0");
        assert_eq!(f5["explicitOptIn"], true);
        assert!(f5["requiredFiles"]
            .as_array()
            .expect("required files")
            .contains(&serde_json::json!(
                "F5TTS_v1_Base/model_1250000.safetensors"
            )));

        let e2 = models
            .iter()
            .find(|model| model["id"] == "e2-tts-base")
            .expect("e2 preset");
        assert_eq!(e2["repoId"], "SWivid/E2-TTS");
        assert_eq!(e2["license"]["id"], "cc-by-nc-4.0");

        let vocos = models
            .iter()
            .find(|model| model["id"] == "vocos-mel-24khz")
            .expect("vocos preset");
        assert_eq!(vocos["repoId"], "charactr/vocos-mel-24khz");
        assert_eq!(vocos["license"]["id"], "mit");
    }
}
