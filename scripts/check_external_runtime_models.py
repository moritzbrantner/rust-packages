#!/usr/bin/env python3
"""Strict external runtime/model smoke-check orchestrator."""

from __future__ import annotations

import argparse
import datetime as dt
import glob
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from typing import Iterable, Literal


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REPORT = ROOT / "target/external-runtime-model-checks/report.json"
SECRET_KEYS = {"HF_TOKEN", "MODEL_HF_TOKEN", "HUGGING_FACE_HUB_TOKEN"}
SETUP_NEEDLES = (
    "missing ",
    "not found",
    "unavailable",
    "is required",
    "requires ",
    "No such file or directory",
    "setup",
    "HF_TOKEN",
    "CUDA",
    "CUBLAS_STATUS_NOT_INITIALIZED",
    "cublas",
    "libcudart",
    "error while loading shared libraries",
    "ORT_DYLIB_PATH",
    "ModuleNotFoundError",
    "GatedRepoError",
    "HfHubHTTPError",
    "UnsupportedColmapCameraModels",
    "UnsupportedCameraModel",
    "unsupported camera model",
    "403",
    "Forbidden",
)


@dataclass(frozen=True)
class RequiredCrate:
    area: str
    package: str
    short_name: str


@dataclass(frozen=True)
class Check:
    check_id: str
    crates: tuple[str, ...]
    runtime: str
    setup_command: str
    run_command: str | None
    kind: str = "runtime"
    required: bool = True
    tags: tuple[str, ...] = ()
    required_env: tuple[str, ...] = ()
    required_paths: tuple[str, ...] = ()
    required_commands: tuple[str, ...] = ()
    required_colmap_text_dirs: tuple[str, ...] = ()
    expected_tests: tuple[str, ...] = ()
    strict_skip_patterns: tuple[str, ...] = ("skipping ",)
    coverage_status_override: Literal["missing-coverage"] | None = None
    missing_coverage_reason: str | None = None
    timeout_seconds: int | None = None


REQUIRED_CRATES: tuple[RequiredCrate, ...] = (
    RequiredCrate("Runtime/model spine", "moritzbrantner-model-runtime", "model-runtime"),
    RequiredCrate("Runtime/model spine", "moritzbrantner-runtime-onnx", "runtime-onnx"),
    RequiredCrate("Runtime/model spine", "moritzbrantner-video-analysis", "root facade"),
    RequiredCrate(
        "Image ONNX/models",
        "moritzbrantner-image-analysis-classification",
        "image-analysis-classification",
    ),
    RequiredCrate(
        "Image ONNX/models",
        "moritzbrantner-image-analysis-captioning",
        "image-analysis-captioning",
    ),
    RequiredCrate(
        "Image ONNX/models",
        "moritzbrantner-image-analysis-detection",
        "image-analysis-detection",
    ),
    RequiredCrate(
        "Image ONNX/models",
        "moritzbrantner-image-analysis-embeddings",
        "image-analysis-embeddings",
    ),
    RequiredCrate("Image ONNX/models", "moritzbrantner-image-analysis-ocr", "image-analysis-ocr"),
    RequiredCrate(
        "Image ONNX/models",
        "moritzbrantner-image-analysis-comfyui",
        "image-analysis-comfyui",
    ),
    RequiredCrate("Text native/models", "moritzbrantner-text-model-runtime", "text-model-runtime"),
    RequiredCrate("Text native/models", "moritzbrantner-text-embeddings", "text-embeddings"),
    RequiredCrate("Text native/models", "moritzbrantner-text-linguistics", "text-linguistics"),
    RequiredCrate("Text native/models", "moritzbrantner-text-classification", "text-classification"),
    RequiredCrate(
        "Text native/models",
        "moritzbrantner-text-question-answering",
        "text-question-answering",
    ),
    RequiredCrate("Text native/models", "moritzbrantner-text-analysis", "text-analysis"),
    RequiredCrate("Text native/models", "moritzbrantner-text-transcripts", "text-transcripts"),
    RequiredCrate("Audio native/models", "moritzbrantner-audio-analysis-io", "audio-analysis-io"),
    RequiredCrate(
        "Audio native/models",
        "moritzbrantner-audio-analysis-speakers",
        "audio-analysis-speakers",
    ),
    RequiredCrate(
        "Audio native/models",
        "moritzbrantner-audio-analysis-transcription",
        "audio-analysis-transcription",
    ),
    RequiredCrate(
        "Audio native/models",
        "moritzbrantner-audio-analysis-separation",
        "audio-analysis-separation",
    ),
    RequiredCrate("Video native/models", "moritzbrantner-video-analysis-ffmpeg", "video-analysis-ffmpeg"),
    RequiredCrate("Video native/models", "moritzbrantner-video-analysis-split", "video-analysis-split"),
    RequiredCrate("Video native/models", "moritzbrantner-video-analysis-posture", "video-analysis-posture"),
    RequiredCrate(
        "Video native/models",
        "moritzbrantner-video-analysis-recognition",
        "video-analysis-recognition",
    ),
    RequiredCrate("Video native/models", "moritzbrantner-video-analysis-cli", "video-analysis-cli"),
    RequiredCrate(
        "Radiance / reconstruction",
        "moritzbrantner-video-analysis-radiance-fields",
        "video-analysis-radiance-fields",
    ),
    RequiredCrate(
        "Radiance / reconstruction",
        "moritzbrantner-video-analysis-radiance-io",
        "video-analysis-radiance-io",
    ),
    RequiredCrate(
        "Radiance / reconstruction",
        "moritzbrantner-video-analysis-radiance-pipeline",
        "video-analysis-radiance-pipeline",
    ),
    RequiredCrate("Radiance / reconstruction", "moritzbrantner-video-analysis-sfm", "video-analysis-sfm"),
    RequiredCrate("Radiance / reconstruction", "moritzbrantner-video-analysis-mvs", "video-analysis-mvs"),
    RequiredCrate(
        "Radiance / reconstruction",
        "moritzbrantner-video-analysis-reconstruction",
        "video-analysis-reconstruction",
    ),
    RequiredCrate(
        "Workflow integration",
        "moritzbrantner-video-analysis-use-cases",
        "prototypes/rust/video-analysis-use-cases",
    ),
)


def required_matrix() -> dict[str, RequiredCrate]:
    return {crate.package: crate for crate in REQUIRED_CRATES}


CHECKS: tuple[Check, ...] = (
    Check(
        "setup-model-python-and-bundles",
        ("moritzbrantner-model-runtime",),
        "model Python deps, ONNX Runtime dylib, model bundles",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "bash scripts/check_model_external_tools.sh all bundles",
        kind="setup",
    ),
    Check(
        "setup-e2e-tools",
        (
            "moritzbrantner-audio-analysis-io",
            "moritzbrantner-audio-analysis-separation",
            "moritzbrantner-text-transcripts",
            "moritzbrantner-video-analysis-ffmpeg",
            "moritzbrantner-video-analysis-split",
            "moritzbrantner-video-analysis-cli",
            "moritzbrantner-video-analysis-use-cases",
        ),
        "FFmpeg, ffprobe, Demucs, whisper.cpp, yt-dlp, COLMAP, Nerfstudio",
        "bash scripts/setup_e2e_external_tools.sh all",
        "bash scripts/check_e2e_external_tools.sh",
        kind="setup",
    ),
    Check(
        "setup-radiance-training-tools",
        (
            "moritzbrantner-video-analysis-radiance-fields",
            "moritzbrantner-video-analysis-radiance-io",
            "moritzbrantner-video-analysis-radiance-pipeline",
            "moritzbrantner-video-analysis-sfm",
            "moritzbrantner-video-analysis-mvs",
            "moritzbrantner-video-analysis-reconstruction",
        ),
        "Nerfstudio training/export CLI with CUDA-capable torch",
        "NERFSTUDIO_REQUIRE_CUDA=1 bash scripts/setup_radiance_external_tools.sh training",
        "bash -lc 'command -v ns-process-data && command -v ns-train && command -v ns-export'",
        kind="setup",
        tags=("gpu",),
        required_commands=("ns-process-data", "ns-train", "ns-export"),
    ),
    Check(
        "cuda-host",
        (
            "moritzbrantner-audio-analysis-transcription",
            "moritzbrantner-video-analysis-radiance-pipeline",
        ),
        "CUDA-capable host",
        "Install NVIDIA driver/CUDA runtime visible to nvidia-smi and Candle/Nerfstudio",
        "bash -lc 'nvidia-smi && (nvcc --version || true)'",
        kind="setup",
        tags=("gpu",),
        required_commands=("nvidia-smi",),
    ),
    Check(
        "hf-token-pyannote",
        ("moritzbrantner-audio-analysis-transcription",),
        "Hugging Face token accepted for token-gated pyannote/WhisperX diarization",
        "Export HF_TOKEN with accepted pyannote gated-model access",
        '"$MODEL_PYTHON_VENV/bin/python" - <<\'PY\'\n'
        "import os\n"
        "from huggingface_hub import HfApi\n"
        "api = HfApi()\n"
        "api.whoami(token=os.environ['HF_TOKEN'])\n"
        "api.model_info('pyannote/speaker-diarization-community-1', token=os.environ['HF_TOKEN'])\n"
        "print('HF_TOKEN accepted for pyannote gated model access')\n"
        "PY",
        kind="setup",
        tags=("token-gated",),
        required_env=("HF_TOKEN",),
        required_paths=("MODEL_PYTHON_VENV",),
    ),
    Check(
        "onnx-native-suite",
        (
            "moritzbrantner-image-analysis-classification",
            "moritzbrantner-image-analysis-captioning",
            "moritzbrantner-image-analysis-detection",
            "moritzbrantner-image-analysis-embeddings",
            "moritzbrantner-image-analysis-ocr",
            "moritzbrantner-text-question-answering",
        ),
        "ONNX Runtime plus local model bundles",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "scripts/check_onnx_external_smoke.sh",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH",),
        expected_tests=(
            "vit_onnx_classifies_tiny_fixture_image",
            "vit_gpt2_onnx_returns_non_empty_caption",
            "detr_like_onnx_decodes_detection_shapes",
            "onnx_image_embedding_returns_finite_normalized_vector",
            "trocr_onnx_returns_non_empty_ocr_document",
            "roberta_onnx_bundle_and_runtime_helpers_regressions",
        ),
    ),
    Check(
        "runtime-onnx-ignored",
        ("moritzbrantner-runtime-onnx",),
        "ONNX Runtime session load/run",
        "bash scripts/setup_model_external_tools.sh onnx",
        "cargo test -p moenarch-runtime-onnx --features external-tests runtime_onnx_loads_and_runs_local_model -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH", "RUNTIME_ONNX_SMOKE_MODEL"),
        required_paths=("ORT_DYLIB_PATH", "RUNTIME_ONNX_SMOKE_MODEL"),
        expected_tests=("runtime_onnx_loads_and_runs_local_model",),
    ),
    Check(
        "facade-model-runtime-spine-onnx",
        ("moritzbrantner-video-analysis", "moritzbrantner-model-runtime"),
        "root facade ONNX model-runtime spine",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "cargo test --features external-tests --test model_runtime_spine_onnx -- --nocapture",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH",),
    ),
    Check(
        "text-model-runtime-external",
        ("moritzbrantner-text-model-runtime",),
        "text runtime tokenizers/Candle/ONNX model bundles",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "cargo test -p moenarch-text-model-runtime --features external-tests -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH",),
        expected_tests=("tokenizer_presets_have_honest_load_reports",),
    ),
    Check(
        "text-embeddings-external",
        ("moritzbrantner-text-embeddings",),
        "text embeddings Candle/ONNX model bundles",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "cargo test -p moenarch-text-embeddings --features external-tests -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH",),
    ),
    Check(
        "text-linguistics-external",
        ("moritzbrantner-text-linguistics",),
        "text linguistics Candle model bundles",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "cargo test -p moenarch-text-linguistics --features external-tests -- --ignored --nocapture",
    ),
    Check(
        "text-classification-distilbert-sst2",
        ("moritzbrantner-text-classification",),
        "local DistilBERT SST-2 Candle classification model bundle",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "cargo test -p moenarch-text-classification --features external-tests --test local_models_external distilbert_sst2 -- --ignored --nocapture",
        required_paths=("TEXT_CLASSIFICATION_DISTILBERT_BUNDLE",),
        expected_tests=(
            "distilbert_sst2_classification_smoke",
            "distilbert_sst2_sentiment_smoke",
        ),
    ),
    Check(
        "text-classification-bart-mnli-onnx",
        ("moritzbrantner-text-classification",),
        "local Xenova BART MNLI ONNX zero-shot model bundle",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "cargo test -p moenarch-text-classification --features external-tests --test local_models_external bart_mnli_zero_shot_smoke -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH", "TEXT_CLASSIFICATION_BART_MNLI_BUNDLE"),
        expected_tests=(
            "bart_mnli_zero_shot_smoke",
        ),
        timeout_seconds=120,
    ),
    Check(
        "text-question-answering-onnx",
        ("moritzbrantner-text-question-answering",),
        "local RoBERTa SQuAD2 ONNX bundle",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "cargo test -p moenarch-text-question-answering --features external-tests -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH",),
    ),
    Check(
        "text-analysis-local-models",
        ("moritzbrantner-text-analysis",),
        "text analysis local-model aggregate",
        "bash scripts/setup_model_external_tools.sh all bundles",
        "cargo test -p moenarch-text-analysis --features external-tests -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH",),
        required_paths=("ORT_DYLIB_PATH",),
    ),
    Check(
        "audio-io-ffmpeg-decode",
        ("moritzbrantner-audio-analysis-io",),
        "FFmpeg-backed audio decode",
        "bash scripts/setup_e2e_external_tools.sh ffmpeg",
        "FFMPEG_EXTERNAL_TESTS=1 cargo test -p moenarch-audio-analysis-io --test ffmpeg_decode -- --nocapture",
        required_commands=("ffmpeg", "ffprobe"),
    ),
    Check(
        "audio-separation-demucs",
        ("moritzbrantner-audio-analysis-separation",),
        "Demucs source separation command",
        "bash scripts/setup_e2e_external_tools.sh demucs",
        "RUN_REAL_DEMUCS_TESTS=1 cargo test -p moenarch-audio-analysis-separation --features external-tests real_demucs_smoke_test_when_requested -- --ignored --nocapture",
        required_commands=("demucs",),
        expected_tests=("real_demucs_smoke_test_when_requested",),
    ),
    Check(
        "video-ffmpeg-native",
        ("moritzbrantner-video-analysis-ffmpeg",),
        "FFmpeg command/runtime smoke",
        "bash scripts/setup_e2e_external_tools.sh ffmpeg",
        "cargo test -p moenarch-video-analysis-ffmpeg --features ffmpeg-tests --lib -- --nocapture",
        required_commands=("ffmpeg", "ffprobe"),
        expected_tests=(
            "decodes_generated_audio",
            "decodes_generated_two_scene_video",
            "decodes_generated_vertical_video_with_resize",
        ),
    ),
    Check(
        "video-split-ffmpeg",
        ("moritzbrantner-video-analysis-split",),
        "FFmpeg scene splitting",
        "bash scripts/setup_e2e_external_tools.sh ffmpeg",
        "cargo test -p moenarch-video-analysis-split --features external-tests --test ffmpeg_split -- --ignored --nocapture",
        required_commands=("ffmpeg", "ffprobe"),
    ),
    Check(
        "video-cli-external",
        ("moritzbrantner-video-analysis-cli",),
        "CLI detection workflow over generated video",
        "bash scripts/setup_e2e_external_tools.sh ffmpeg",
        "cargo test -p moenarch-video-analysis-cli --test cli_smoke vanalyze_detect_writes_scene_csv_for_generated_video -- --ignored --nocapture",
        required_commands=("ffmpeg", "ffprobe"),
    ),
    Check(
        "audio-transcription-candle-cuda",
        ("moritzbrantner-audio-analysis-transcription",),
        "Candle Whisper CUDA local bundle",
        "Prepare CUDA libs plus TRANSCRIPTION_MODEL_BUNDLE and TRANSCRIPTION_AUDIO_PATH",
        "RUN_NATIVE_TRANSCRIPTION_TESTS=1 cargo test -p moenarch-audio-analysis-transcription --features candle,cuda,model-bundles candle_whisper_cuda_smoke_when_requested -- --ignored --nocapture",
        tags=("gpu",),
        required_env=("TRANSCRIPTION_MODEL_BUNDLE", "TRANSCRIPTION_AUDIO_PATH"),
        required_paths=("TRANSCRIPTION_MODEL_BUNDLE", "TRANSCRIPTION_AUDIO_PATH"),
        required_commands=("nvidia-smi",),
    ),
    Check(
        "audio-transcription-wav2vec2-alignment",
        ("moritzbrantner-audio-analysis-transcription",),
        "Candle wav2vec2 CTC alignment local bundle",
        "bash scripts/setup_model_external_tools.sh bundles",
        "cargo test -p moenarch-audio-analysis-transcription --features candle,alignment,model-bundles ctc_alignment_wav2vec2_smoke_when_requested -- --ignored --nocapture",
        required_env=("ALIGNMENT_MODEL_DIR", "TRANSCRIPTION_AUDIO_PATH"),
        required_paths=("ALIGNMENT_MODEL_DIR", "TRANSCRIPTION_AUDIO_PATH"),
    ),
    Check(
        "audio-transcription-native-media-decode",
        ("moritzbrantner-audio-analysis-transcription", "moritzbrantner-audio-analysis-io"),
        "native media decode through audio-analysis-io",
        "bash scripts/setup_e2e_external_tools.sh ffmpeg",
        "RUN_NATIVE_MEDIA_DECODE_TESTS=1 cargo test -p moenarch-audio-analysis-transcription --features audio-io native_media_decode_when_requested -- --ignored --nocapture",
        required_env=("TRANSCRIPTION_MEDIA_PATH",),
        required_paths=("TRANSCRIPTION_MEDIA_PATH",),
        required_commands=("ffmpeg", "ffprobe"),
    ),
    Check(
        "audio-speakers-native-diarization",
        ("moritzbrantner-audio-analysis-speakers",),
        "native diarization baseline over local WAV",
        "Prepare DIARIZATION_AUDIO_PATH",
        "RUN_NATIVE_DIARIZATION_TESTS=1 cargo test -p moenarch-audio-analysis-speakers --features external-tests native_diarization_baseline_smoke_when_requested -- --ignored --nocapture",
        required_env=("DIARIZATION_AUDIO_PATH",),
        required_paths=("DIARIZATION_AUDIO_PATH",),
        expected_tests=("native_diarization_baseline_smoke_when_requested",),
    ),
    Check(
        "audio-speakers-onnx-embedding",
        ("moritzbrantner-audio-analysis-speakers",),
        "ONNX speaker embedding local model bundle",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "RUN_NATIVE_SPEAKER_MODEL_TESTS=1 cargo test -p moenarch-audio-analysis-speakers --features onnx,model-bundles onnx_speaker_embedding_smoke_when_requested -- --ignored --nocapture",
        required_env=(
            "ORT_DYLIB_PATH",
            "SPEAKER_EMBEDDING_MODEL_BUNDLE",
            "DIARIZATION_AUDIO_PATH",
        ),
        required_paths=("ORT_DYLIB_PATH", "SPEAKER_EMBEDDING_MODEL_BUNDLE", "DIARIZATION_AUDIO_PATH"),
        expected_tests=("onnx_speaker_embedding_smoke_when_requested",),
    ),
    Check(
        "audio-transcription-onnx-diarization",
        ("moritzbrantner-audio-analysis-transcription", "moritzbrantner-audio-analysis-speakers"),
        "native transcription ONNX diarization path",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "RUN_NATIVE_SPEAKER_MODEL_TESTS=1 cargo test -p moenarch-audio-analysis-transcription --features diarization,onnx,model-bundles native_onnx_diarization_smoke_when_requested -- --ignored --nocapture",
        required_env=(
            "ORT_DYLIB_PATH",
            "SPEAKER_EMBEDDING_MODEL_BUNDLE",
            "DIARIZATION_AUDIO_PATH",
        ),
        required_paths=("ORT_DYLIB_PATH", "SPEAKER_EMBEDDING_MODEL_BUNDLE", "DIARIZATION_AUDIO_PATH"),
    ),
    Check(
        "audio-transcription-whisperx-token-gated",
        ("moritzbrantner-audio-analysis-transcription",),
        "WhisperX pyannote diarization parity",
        "bash scripts/setup_audio_external_tools.sh ffmpeg demucs and install whisperx in .audio-tools/whisperx-venv",
        "RUN_WHISPERX_PARITY_TESTS=1 WHISPERX_DIARIZE=1 cargo test --test audio_transcription_native_contracts external_whisperx_parity_when_requested -- --ignored --nocapture",
        tags=("token-gated",),
        required_env=("HF_TOKEN", "WHISPERX_AUDIO_PATH"),
        required_paths=("WHISPERX_AUDIO_PATH",),
        required_commands=("ffmpeg",),
    ),
    Check(
        "text-transcripts-whisper-cpp",
        ("moritzbrantner-text-transcripts",),
        "native whisper.cpp CLI transcript parse/import",
        "bash scripts/setup_e2e_external_tools.sh whisper && bash scripts/setup_whisper_cpp_external_model.sh",
        "RUN_NATIVE_WHISPER_TESTS=1 cargo test -p moenarch-text-transcripts --features native,external-tests native_whisper_cpp_smoke_when_requested -- --ignored --nocapture",
        required_env=("NATIVE_WHISPER_AUDIO_PATH", "WHISPER_CPP_MODEL_STORE"),
        required_paths=("NATIVE_WHISPER_AUDIO_PATH", "WHISPER_CPP_MODEL_STORE"),
        required_commands=("whisper",),
    ),
    Check(
        "workflow-use-cases-core-external",
        ("moritzbrantner-video-analysis-use-cases",),
        "use-case workflows with external tools",
        "bash scripts/setup_e2e_external_tools.sh all",
        "cargo test -p moenarch-video-analysis-use-cases --test external_tools -- --ignored --nocapture --skip image_person_edit_workflow_runs_with_real_tools_when_configured",
        required_commands=("ffmpeg", "ffprobe", "demucs", "yt-dlp"),
        expected_tests=(
            "yt_dlp_can_resolve_default_smoke_test_video",
            "me_at_the_zoo_workflow_reports_one_scene",
            "me_at_the_zoo_workflow_counts_one_person_per_sampled_frame",
            "generated_video_red_cars_workflow_counts_red_cars",
            "audio_voice_analysis_workflow_runs_with_real_tools_when_configured",
        ),
    ),
    Check(
        "workflow-image-person-edit-real-tools",
        ("moritzbrantner-video-analysis-use-cases",),
        "image person edit workflow with configured detector/editor tools",
        "Configure IMAGE_PERSON_EDIT_INPUT, IMAGE_PERSON_EDIT_DETECTOR_COMMAND, and optional IMAGE_PERSON_EDIT_DETECTOR_ARGS/IMAGE_PERSON_EDIT_EDITOR_ARGS",
        "STRICT_EXTERNAL_RUNTIME_CHECKS=1 cargo test -p moenarch-video-analysis-use-cases --test external_tools image_person_edit_workflow_runs_with_real_tools_when_configured -- --ignored --nocapture",
        required_env=("IMAGE_PERSON_EDIT_INPUT", "IMAGE_PERSON_EDIT_DETECTOR_COMMAND"),
        required_paths=("IMAGE_PERSON_EDIT_INPUT",),
        expected_tests=("image_person_edit_workflow_runs_with_real_tools_when_configured",),
    ),
    Check(
        "image-comfyui-server",
        ("moritzbrantner-image-analysis-comfyui",),
        "ComfyUI HTTP server workflow submission and non-empty response",
        "Start ComfyUI and export COMFYUI_URL",
        "cargo test -p moenarch-image-analysis-comfyui --test external_comfyui_smoke comfyui_submits_generation_workflow_when_configured -- --ignored --nocapture",
        required_env=("COMFYUI_URL",),
        expected_tests=("comfyui_submits_generation_workflow_when_configured",),
    ),
    Check(
        "video-posture-onnx",
        ("moritzbrantner-video-analysis-posture",),
        "posture ONNX model load/run",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "cargo test -p moenarch-video-analysis-posture --features onnx --test external_onnx_smoke yolov8n_pose_onnx_estimator_loads_and_runs_fixture_frame -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH", "POSTURE_ONNX_MODEL_BUNDLE"),
        required_paths=("ORT_DYLIB_PATH", "POSTURE_ONNX_MODEL_BUNDLE"),
        expected_tests=("yolov8n_pose_onnx_estimator_loads_and_runs_fixture_frame",),
    ),
    Check(
        "video-recognition-onnx",
        ("moritzbrantner-video-analysis-recognition",),
        "recognition ONNX detector/model load/run",
        "bash scripts/setup_model_external_tools.sh onnx bundles",
        "cargo test -p moenarch-video-analysis-recognition --features onnx --test external_onnx_smoke video_recognition_onnx_detects_fixture_frame_when_configured -- --ignored --nocapture",
        required_env=("ORT_DYLIB_PATH", "RECOGNITION_ONNX_MODEL_BUNDLE"),
        required_paths=("ORT_DYLIB_PATH", "RECOGNITION_ONNX_MODEL_BUNDLE"),
        expected_tests=("video_recognition_onnx_detects_fixture_frame_when_configured",),
    ),
    Check(
        "radiance-fields-runtime",
        ("moritzbrantner-video-analysis-radiance-fields",),
        "radiance field runtime/model backend",
        "NERFSTUDIO_REQUIRE_CUDA=1 bash scripts/setup_radiance_external_tools.sh training",
        None,
        tags=("gpu",),
        missing_coverage_reason=(
            "No real external radiance-field model/training smoke is wired to this crate."
        ),
    ),
    Check(
        "radiance-io-external",
        ("moritzbrantner-video-analysis-radiance-io",),
        "COLMAP external project import",
        "bash scripts/setup_radiance_external_tools.sh colmap && scripts/setup_colmap_test_video.sh",
        "STRICT_EXTERNAL_RUNTIME_CHECKS=1 cargo test -p moenarch-video-analysis-radiance-io --test external_colmap_output radiance_io_reads_external_colmap_output -- --ignored --nocapture",
        required_env=("COLMAP_SPARSE_TEXT_DIR",),
        required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        expected_tests=("radiance_io_reads_external_colmap_output",),
    ),
    Check(
        "radiance-pipeline-external",
        ("moritzbrantner-video-analysis-radiance-pipeline",),
        "COLMAP pipeline load-run",
        "bash scripts/setup_radiance_external_tools.sh colmap && scripts/setup_colmap_test_video.sh",
        "STRICT_EXTERNAL_RUNTIME_CHECKS=1 cargo test -p moenarch-video-analysis-radiance-pipeline --test external_colmap_project radiance_pipeline_loads_external_colmap_project -- --ignored --nocapture",
        required_env=("COLMAP_SPARSE_TEXT_DIR",),
        required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        expected_tests=("radiance_pipeline_loads_external_colmap_project",),
    ),
    Check(
        "sfm-colmap-reconstruct-video",
        ("moritzbrantner-video-analysis-sfm",),
        "COLMAP video reconstruction command workflow",
        "bash scripts/setup_radiance_external_tools.sh colmap && scripts/setup_colmap_test_video.sh",
        "STRICT_EXTERNAL_RUNTIME_CHECKS=1 cargo test -p moenarch-video-analysis-sfm native_colmap_video_reconstruction_smoke_when_configured -- --ignored --nocapture",
        required_env=("COLMAP_TEST_VIDEO_PATH",),
        required_paths=("COLMAP_TEST_VIDEO_PATH",),
        required_commands=("ffmpeg", "colmap"),
        expected_tests=("native_colmap_video_reconstruction_smoke_when_configured",),
        timeout_seconds=300,
    ),
    Check(
        "mvs-external-runtime",
        ("moritzbrantner-video-analysis-mvs",),
        "MVS external reconstruction backend",
        "bash scripts/setup_radiance_external_tools.sh colmap",
        "STRICT_EXTERNAL_RUNTIME_CHECKS=1 cargo test -p moenarch-video-analysis-mvs --test external_colmap_dense_smoke colmap_dense_mvs_smoke_when_configured -- --ignored --nocapture",
        required_env=("COLMAP_MVS_IMAGE_DIR", "COLMAP_MVS_SPARSE_DIR"),
        required_paths=("COLMAP_MVS_IMAGE_DIR", "COLMAP_MVS_SPARSE_DIR"),
        required_commands=("colmap",),
        required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        expected_tests=("colmap_dense_mvs_smoke_when_configured",),
        timeout_seconds=600,
    ),
    Check(
        "reconstruction-external-runtime",
        ("moritzbrantner-video-analysis-reconstruction",),
        "reconstruction sparse COLMAP conversion",
        "bash scripts/setup_radiance_external_tools.sh colmap && scripts/setup_colmap_test_video.sh",
        "STRICT_EXTERNAL_RUNTIME_CHECKS=1 cargo test -p moenarch-video-analysis-radiance-io --test external_colmap_output reconstruction_accepts_external_sparse_colmap_output -- --ignored --nocapture",
        required_env=("COLMAP_SPARSE_TEXT_DIR",),
        required_colmap_text_dirs=("COLMAP_SPARSE_TEXT_DIR",),
        expected_tests=("reconstruction_accepts_external_sparse_colmap_output",),
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run strict external runtime/model smoke checks and write JSON/Markdown reports."
    )
    parser.add_argument("--strict", action="store_true", help="treat skipped ignored tests as failures")
    parser.add_argument("--gpu", action="store_true", help="include GPU/CUDA-required checks")
    parser.add_argument(
        "--token-gated",
        action="store_true",
        help="include token-gated Hugging Face/pyannote/WhisperX checks",
    )
    parser.add_argument("--setup", action="store_true", help="run setup commands before verification")
    parser.add_argument(
        "--report",
        default=str(DEFAULT_REPORT),
        help="JSON report path (default: target/external-runtime-model-checks/report.json)",
    )
    parser.add_argument(
        "--continue-on-failure",
        action="store_true",
        default=True,
        help=argparse.SUPPRESS,
    )
    return parser.parse_args()


def shell_quote(value: str) -> str:
    return "'" + value.replace("'", "'\"'\"'") + "'"


def command_available(command: str, env: dict[str, str]) -> bool:
    path = env.get("PATH")
    return shutil.which(command, path=path) is not None


def make_abs(path: str) -> str:
    return str(Path(path).expanduser().resolve())


def derive_ort_dylib() -> str | None:
    patterns = [
        ROOT / ".external-test-tools/model-python-venv/lib/python*/site-packages/onnxruntime/capi/libonnxruntime.so*",
        ROOT / ".external-test-tools/model-python-venv/lib/python*/site-packages/onnxruntime/capi/libonnxruntime.dylib*",
        ROOT / ".audio-tools/whisperx-venv/lib/python*/site-packages/onnxruntime/capi/libonnxruntime.so*",
        ROOT / ".audio-tools/whisperx-venv/lib/python*/site-packages/onnxruntime/capi/libonnxruntime.dylib*",
    ]
    candidates: list[str] = []
    for pattern in patterns:
        candidates.extend(glob.glob(str(pattern)))
    files = [Path(candidate) for candidate in candidates if Path(candidate).is_file()]
    if not files:
        return None
    return str(sorted(files)[0].resolve())


def prepare_env() -> tuple[dict[str, str], dict[str, str]]:
    env = os.environ.copy()
    for local_bin in (ROOT / ".external-test-tools/bin", ROOT / ".audio-tools/bin"):
        if local_bin.is_dir():
            env["PATH"] = f"{local_bin}:{env.get('PATH', '')}"

    smoke_root = Path(env.get("SMOKE_ROOT", Path.home() / ".local/share/video-analysis-smoke"))
    env["SMOKE_ROOT"] = str(smoke_root.expanduser().resolve())
    smoke = Path(env["SMOKE_ROOT"])
    env["MODEL_PYTHON_VENV"] = make_abs(
        env.get("MODEL_PYTHON_VENV", str(ROOT / ".external-test-tools/model-python-venv"))
    )

    defaults = {
        "TRANSCRIPTION_MODEL_BUNDLE": smoke / "whisper-tiny",
        "TRANSCRIPTION_AUDIO_PATH": smoke / "audio/native-transcription-smoke.wav",
        "TRANSCRIPTION_MEDIA_PATH": ROOT / "tests/fixtures/me-at-the-zoo-jNQXAC9IVRw.webm",
        "DIARIZATION_AUDIO_PATH": smoke / "audio/native-transcription-smoke.wav",
        "ALIGNMENT_MODEL_DIR": smoke / "models/wav2vec2-base-960h/main",
        "SPEAKER_EMBEDDING_MODEL_BUNDLE": smoke / "models/wespeaker-voxceleb-resnet34-LM/main",
        "TEXT_CLASSIFICATION_DISTILBERT_BUNDLE": ROOT
        / ".model-runtime/distilbert-sst2/main/manifest.json",
        "TEXT_CLASSIFICATION_BART_MNLI_BUNDLE": ROOT
        / ".model-runtime/xenova-bart-large-mnli-onnx/main/manifest.json",
        "NATIVE_WHISPER_AUDIO_PATH": smoke / "audio/native-transcription-smoke.wav",
        "WHISPER_CPP_MODEL_STORE": ROOT / ".model-runtime/whisper-cpp",
        "WHISPERX_AUDIO_PATH": smoke / "audio/native-transcription-smoke.wav",
        "RUNTIME_ONNX_SMOKE_MODEL": ROOT
        / ".model-runtime/roberta-base-squad2-onnx/main/files/onnx/model_quantized.onnx",
        "RECOGNITION_ONNX_MODEL_BUNDLE": ROOT / ".model-runtime/xenova-detr-resnet-50-onnx/main",
        "POSTURE_ONNX_MODEL_BUNDLE": ROOT / ".model-runtime/xenova-yolov8n-pose-onnx/main",
        "COLMAP_TEST_VIDEO_PATH": ROOT
        / "prototypes/web/video-analysis-web/public/samples/video/test-video.mp4",
        "COLMAP_SPARSE_TEXT_DIR": ROOT / ".external-test-tools/colmap-runs/test-video/sparse_txt",
        "COLMAP_MVS_IMAGE_DIR": ROOT / ".external-test-tools/colmap-runs/test-video/frames",
        "COLMAP_MVS_SPARSE_DIR": ROOT / ".external-test-tools/colmap-runs/test-video/sparse/0",
        "COLMAP_MVS_WORKSPACE_DIR": ROOT / ".external-test-tools/colmap-runs/test-video/dense",
        "COLMAP_MVS_FUSED_PLY": ROOT / ".external-test-tools/colmap-runs/test-video/dense/fused.ply",
        "IMAGE_PERSON_EDIT_INPUT": ROOT / ".external-test-tools/image-person-edit/person-frame.jpg",
    }
    for key, default in defaults.items():
        env[key] = make_abs(env.get(key, str(default)))

    whisperx_local = ROOT / ".audio-tools/whisperx-venv/bin/whisperx"
    if "WHISPERX_COMMAND" not in env and whisperx_local.exists():
        env["WHISPERX_COMMAND"] = str(whisperx_local.resolve())
    env.setdefault("WHISPERX_MODEL", "tiny.en")
    env.setdefault("WHISPERX_LANGUAGE", "en")
    env.setdefault("WHISPERX_DEVICE", "cpu")
    env.setdefault("WHISPERX_COMPUTE_TYPE", "int8")
    env.setdefault("SPEAKER_EMBEDDING_MODEL_FILE", "speaker-embedding.onnx")
    env.setdefault("SPEAKER_EMBEDDING_DIMENSION", "256")
    env.setdefault("COLMAP_MVS_USE_GPU", "0")

    if "ORT_DYLIB_PATH" in env:
        env["ORT_DYLIB_PATH"] = make_abs(env["ORT_DYLIB_PATH"])
    else:
        derived = derive_ort_dylib()
        if derived:
            env["ORT_DYLIB_PATH"] = derived

    recorded_keys = (
        "SMOKE_ROOT",
        "TRANSCRIPTION_MODEL_BUNDLE",
        "TRANSCRIPTION_AUDIO_PATH",
        "TRANSCRIPTION_MEDIA_PATH",
        "DIARIZATION_AUDIO_PATH",
        "ALIGNMENT_MODEL_DIR",
        "SPEAKER_EMBEDDING_MODEL_BUNDLE",
        "TEXT_CLASSIFICATION_DISTILBERT_BUNDLE",
        "TEXT_CLASSIFICATION_BART_MNLI_BUNDLE",
        "SPEAKER_EMBEDDING_MODEL_FILE",
        "SPEAKER_EMBEDDING_DIMENSION",
        "MODEL_PYTHON_VENV",
        "ORT_DYLIB_PATH",
        "WHISPERX_COMMAND",
        "WHISPERX_AUDIO_PATH",
        "NATIVE_WHISPER_AUDIO_PATH",
        "WHISPER_CPP_MODEL_STORE",
        "WHISPERX_MODEL",
        "WHISPERX_LANGUAGE",
        "WHISPERX_DEVICE",
        "WHISPERX_COMPUTE_TYPE",
        "RUNTIME_ONNX_SMOKE_MODEL",
        "RECOGNITION_ONNX_MODEL_BUNDLE",
        "RECOGNITION_ONNX_IMAGE",
        "POSTURE_ONNX_MODEL_BUNDLE",
        "POSTURE_ONNX_IMAGE",
        "COLMAP_TEST_VIDEO_PATH",
        "COLMAP_SPARSE_TEXT_DIR",
        "COLMAP_MVS_IMAGE_DIR",
        "COLMAP_MVS_SPARSE_DIR",
        "COLMAP_MVS_WORKSPACE_DIR",
        "COLMAP_MVS_FUSED_PLY",
        "COLMAP_MVS_USE_GPU",
        "COMFYUI_URL",
        "COMFYUI_CHECKPOINT",
        "COMFYUI_WAIT_FOR_OUTPUT",
        "IMAGE_PERSON_EDIT_INPUT",
        "IMAGE_PERSON_EDIT_DETECTOR_COMMAND",
        "IMAGE_PERSON_EDIT_DETECTOR_ARGS",
        "IMAGE_PERSON_EDIT_EDITOR_COMMAND",
        "IMAGE_PERSON_EDIT_EDITOR_ARGS",
        "HF_TOKEN",
        "MODEL_HF_TOKEN",
    )
    safe_env = {}
    for key in recorded_keys:
        if key in env:
            safe_env[key] = "<redacted>" if key in SECRET_KEYS else env[key]
        else:
            safe_env[key] = None
    return env, safe_env


def included(check: Check, args: argparse.Namespace) -> bool:
    tags = set(check.tags)
    if "gpu" in tags and not args.gpu:
        return False
    if "token-gated" in tags and not args.token_gated:
        return False
    return True


def command_for_check(check: Check, args: argparse.Namespace) -> str | None:
    if args.setup and check.kind == "setup":
        return check.setup_command
    return check.run_command


def precondition_blockers(check: Check, env: dict[str, str]) -> list[str]:
    blockers: list[str] = []
    for key in check.required_env:
        if not env.get(key):
            blockers.append(f"missing required env {key}")
    for key in check.required_paths:
        value = env.get(key)
        if not value:
            continue
        if not Path(value).is_absolute():
            blockers.append(f"{key} must be absolute, got {value}")
        elif not Path(value).exists():
            blockers.append(f"{key} path does not exist: {value}")
    for command in check.required_commands:
        if not command_available(command, env):
            blockers.append(f"missing command on PATH: {command}")
    for key in check.required_colmap_text_dirs:
        value = env.get(key)
        if not value:
            blockers.append(f"missing required COLMAP sparse text dir env {key}")
            continue
        path = Path(value)
        if not path.is_absolute():
            blockers.append(f"{key} must be absolute, got {value}")
            continue
        if not path.is_dir():
            blockers.append(f"{key} directory does not exist: {value}")
            continue
        for name in ("cameras.txt", "images.txt", "points3D.txt"):
            file_path = path / name
            if not file_path.is_file():
                blockers.append(f"{key} missing COLMAP text file: {file_path}")
            elif file_path.stat().st_size == 0:
                blockers.append(f"{key} COLMAP text file is empty: {file_path}")
    return blockers


def run_command(
    command: str,
    env: dict[str, str],
    log_path: Path,
    timeout_seconds: int | None,
) -> subprocess.CompletedProcess[str]:
    log_path.parent.mkdir(parents=True, exist_ok=True)
    header = [
        f"$ {command}",
        f"cwd: {ROOT}",
        f"started_at: {dt.datetime.now(dt.timezone.utc).isoformat()}",
        "",
    ]
    with log_path.open("w", encoding="utf-8") as handle:
        handle.write("\n".join(header))
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        shell=True,
        executable="/bin/bash",
        capture_output=True,
        timeout=timeout_seconds,
    )
    with log_path.open("a", encoding="utf-8") as handle:
        if completed.stdout:
            handle.write("\n[stdout]\n")
            handle.write(completed.stdout)
        if completed.stderr:
            handle.write("\n[stderr]\n")
            handle.write(completed.stderr)
        handle.write(f"\nexit_code: {completed.returncode}\n")
    return completed


def classify_failure(output: str) -> str:
    lowered = output.lower()
    if any(needle.lower() in lowered for needle in SETUP_NEEDLES):
        return "blocked-setup"
    return "fail"


def strict_skip_evidence(output: str, patterns: Iterable[str]) -> list[str]:
    matched: list[str] = []
    lowered_patterns = tuple(pattern.lower() for pattern in patterns)
    for line in output.splitlines():
        lowered = line.lower()
        if any(pattern in lowered for pattern in lowered_patterns):
            matched.append(line)
    return matched


PASSED_TEST_RE = re.compile(r"^test (?P<name>\S+) \.\.\. ok$")


def passed_test_names(output: str) -> set[str]:
    names: set[str] = set()
    for line in output.splitlines():
        match = PASSED_TEST_RE.match(line.strip())
        if match:
            names.add(match.group("name"))
    return names


def expected_test_evidence(output: str, expected_tests: Iterable[str]) -> tuple[list[str], list[str]]:
    passed = passed_test_names(output)
    evidence: list[str] = []
    missing: list[str] = []
    for expected in expected_tests:
        if expected in passed or any(name.endswith(f"::{expected}") for name in passed):
            evidence.append(expected)
        else:
            missing.append(expected)
    return evidence, missing


def classify_completed_check(
    check: Check,
    completed: subprocess.CompletedProcess[str],
    strict: bool,
) -> tuple[str, list[str], list[str], list[str], list[str]]:
    combined = f"{completed.stdout}\n{completed.stderr}"
    evidence, missing_evidence = expected_test_evidence(combined, check.expected_tests)
    skip_evidence = strict_skip_evidence(combined, check.strict_skip_patterns) if strict else []
    if completed.returncode == 0:
        if strict and skip_evidence:
            return "fail", ["strict mode detected skipped smoke output"], evidence, missing_evidence, skip_evidence
        if missing_evidence:
            return (
                "fail",
                ["command succeeded but expected test evidence was missing"],
                evidence,
                missing_evidence,
                skip_evidence,
            )
        return "pass", [], evidence, missing_evidence, skip_evidence

    status = classify_failure(combined)
    return status, [f"command exited with {completed.returncode}"], evidence, missing_evidence, skip_evidence


def slug(value: str) -> str:
    return "".join(ch if ch.isalnum() else "-" for ch in value).strip("-").lower()


def load_feature_inventory(env: dict[str, str], out_dir: Path) -> dict[str, object]:
    log_path = out_dir / "logs/cargo-metadata.log"
    try:
        completed = run_command(
            "cargo metadata --no-deps --format-version 1",
            env,
            log_path,
            timeout_seconds=None,
        )
    except Exception as error:  # pragma: no cover - defensive report path
        return {"status": "fail", "error": str(error), "log_path": str(log_path)}
    if completed.returncode != 0:
        return {
            "status": "fail",
            "exit_code": completed.returncode,
            "log_path": str(log_path),
        }
    try:
        metadata = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        return {"status": "fail", "error": str(error), "log_path": str(log_path)}

    feature_needles = {
        "external-tests",
        "local-models",
        "local-onnx",
        "onnx",
        "candle",
        "cuda",
        "native",
        "model-bundles",
    }
    packages = []
    for package in metadata.get("packages", []):
        features = package.get("features", {})
        matched = sorted(
            feature
            for feature in features
            if feature in feature_needles or feature.startswith("ffmpeg-")
        )
        if matched:
            packages.append(
                {
                    "name": package.get("name"),
                    "manifest_path": package.get("manifest_path"),
                    "matched_features": matched,
                }
            )
    return {
        "status": "pass",
        "log_path": str(log_path),
        "matched_feature_packages": packages,
    }


def excluded_by_option(package: str, included_results: list[dict[str, object]]) -> list[str]:
    if included_results:
        return []
    reasons: list[str] = []
    for check in CHECKS:
        if package not in check.crates:
            continue
        tags = set(check.tags)
        if "gpu" in tags:
            reasons.append(f"{check.check_id} excluded because --gpu was not requested")
        if "token-gated" in tags:
            reasons.append(f"{check.check_id} excluded because --token-gated was not requested")
    return reasons


def aggregate_crates(check_results: list[dict[str, object]]) -> list[dict[str, object]]:
    matrix = required_matrix()
    by_crate: dict[str, list[dict[str, object]]] = {package: [] for package in matrix}
    for result in check_results:
        for package in result["crates"]:
            if package in by_crate:
                by_crate[package].append(result)

    ranks = {"excluded": 0, "pass": 1, "missing-coverage": 2, "blocked-setup": 3, "fail": 4}
    summary = []
    for package, crate in matrix.items():
        results = by_crate[package]
        if not results:
            excluded_reasons = excluded_by_option(package, results)
            if excluded_reasons:
                status = "excluded"
                blockers = excluded_reasons
            else:
                status = "missing-coverage"
                blockers = ["no check row covered this required crate"]
            log_paths: list[str] = []
        else:
            status = max((str(item["status"]) for item in results), key=lambda item: ranks[item])
            blockers = [
                blocker
                for item in results
                for blocker in item.get("blockers", [])
                if isinstance(blocker, str)
            ]
            log_paths = [
                str(item["log_path"])
                for item in results
                if item.get("log_path")
            ]
        summary.append(
            {
                "area": crate.area,
                "package": package,
                "short_name": crate.short_name,
                "status": status,
                "blockers": blockers,
                "log_paths": log_paths,
            }
        )
    return summary


def markdown_report(report: dict[str, object]) -> str:
    lines = [
        "# External Runtime And Model Checks",
        "",
        f"- Generated: `{report['generated_at']}`",
        f"- Strict: `{report['options']['strict']}`",
        f"- GPU checks: `{report['options']['gpu']}`",
        f"- Token-gated checks: `{report['options']['token_gated']}`",
        f"- Setup mode: `{report['options']['setup']}`",
        f"- Overall status: `{report['status']}`",
        "",
        "## Crate Status",
        "",
        "| Area | Crate | Status | Blockers |",
        "| --- | --- | --- | --- |",
    ]
    for crate in report["crates"]:
        blockers = "<br>".join(crate["blockers"]) if crate["blockers"] else ""
        lines.append(
            f"| {crate['area']} | `{crate['package']}` | `{crate['status']}` | {blockers} |"
        )

    lines.extend(
        [
            "",
            "## Checks",
            "",
            "| Check | Crates | Runtime/model | Status | Log |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for check in report["checks"]:
        crates = "<br>".join(f"`{crate}`" for crate in check["crates"])
        log_path = check.get("log_path") or ""
        if log_path:
            log_path = f"`{log_path}`"
        lines.append(
            f"| `{check['id']}` | {crates} | {check['runtime']} | `{check['status']}` | {log_path} |"
        )

    lines.extend(["", "## Recorded Environment", "", "| Key | Value |", "| --- | --- |"])
    for key, value in report["environment"].items():
        lines.append(f"| `{key}` | `{value}` |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    report_path = Path(args.report).expanduser()
    if not report_path.is_absolute():
        report_path = (ROOT / report_path).resolve()
    out_dir = report_path.parent
    logs_dir = out_dir / "logs"
    out_dir.mkdir(parents=True, exist_ok=True)
    logs_dir.mkdir(parents=True, exist_ok=True)

    env, safe_env = prepare_env()
    feature_inventory = load_feature_inventory(env, out_dir)

    results: list[dict[str, object]] = []
    for check in CHECKS:
        if not included(check, args):
            continue

        log_path = logs_dir / f"{slug(check.check_id)}.log"
        if check.missing_coverage_reason:
            log_path.write_text(check.missing_coverage_reason + "\n", encoding="utf-8")
            results.append(
                {
                    "id": check.check_id,
                    "crates": list(check.crates),
                    "runtime": check.runtime,
                    "setup_command": check.setup_command,
                    "run_command": check.run_command,
                    "status": "missing-coverage",
                    "blockers": [check.missing_coverage_reason],
                    "evidence": [],
                    "missing_evidence": list(check.expected_tests),
                    "skip_evidence": [],
                    "log_path": str(log_path),
                }
            )
            continue

        command = command_for_check(check, args)
        blockers = precondition_blockers(check, env)
        if blockers:
            log_path.write_text("\n".join(blockers) + "\n", encoding="utf-8")
            results.append(
                {
                    "id": check.check_id,
                    "crates": list(check.crates),
                    "runtime": check.runtime,
                    "setup_command": check.setup_command,
                    "run_command": check.run_command,
                    "status": "blocked-setup",
                    "blockers": blockers,
                    "evidence": [],
                    "missing_evidence": list(check.expected_tests),
                    "skip_evidence": [],
                    "log_path": str(log_path),
                }
            )
            continue
        if command is None:
            log_path.write_text("no run command configured\n", encoding="utf-8")
            results.append(
                {
                    "id": check.check_id,
                    "crates": list(check.crates),
                    "runtime": check.runtime,
                    "setup_command": check.setup_command,
                    "run_command": None,
                    "status": "missing-coverage",
                    "blockers": ["no run command configured"],
                    "evidence": [],
                    "missing_evidence": list(check.expected_tests),
                    "skip_evidence": [],
                    "log_path": str(log_path),
                }
            )
            continue

        try:
            completed = run_command(command, env, log_path, check.timeout_seconds)
            status, blockers, evidence, missing_evidence, skip_evidence = classify_completed_check(
                check,
                completed,
                args.strict,
            )
        except subprocess.TimeoutExpired:
            status = "fail"
            blockers = [f"command timed out after {check.timeout_seconds}s"]
            evidence = []
            missing_evidence = list(check.expected_tests)
            skip_evidence = []
            with log_path.open("a", encoding="utf-8") as handle:
                handle.write(f"\ncommand timed out after {check.timeout_seconds}s\n")

        results.append(
            {
                "id": check.check_id,
                "crates": list(check.crates),
                "runtime": check.runtime,
                "setup_command": check.setup_command,
                "run_command": command,
                "status": status,
                "blockers": blockers,
                "evidence": evidence,
                "missing_evidence": missing_evidence,
                "skip_evidence": skip_evidence,
                "log_path": str(log_path),
            }
        )

    crate_summary = aggregate_crates(results)
    overall = "pass"
    for crate in crate_summary:
        if crate["status"] not in ("pass", "excluded"):
            overall = "fail"
            break

    report = {
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "workspace_root": str(ROOT),
        "status": overall,
        "options": {
            "strict": args.strict,
            "gpu": args.gpu,
            "token_gated": args.token_gated,
            "setup": args.setup,
        },
        "environment": safe_env,
        "feature_inventory": feature_inventory,
        "crates": crate_summary,
        "checks": results,
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    md_path = out_dir / "report.md"
    md_path.write_text(markdown_report(report), encoding="utf-8")

    print(f"wrote JSON report: {report_path}")
    print(f"wrote Markdown report: {md_path}")
    for crate in crate_summary:
        print(f"{crate['status']}: {crate['package']}")

    return 0 if overall == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
