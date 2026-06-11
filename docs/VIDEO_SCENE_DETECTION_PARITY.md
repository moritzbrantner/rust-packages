# Video Scene Detection Parity

This workspace treats PySceneDetect as the behavior baseline for scene boundary
detectors and surrounding workflows. The Rust APIs stay idiomatic and crate
owned; parity means comparable detector behavior, scene lists, stats, outputs,
and split plans.

## Current Baseline

Current parity target: PySceneDetect `0.6.7.1`.

The vendored behavior reference is pinned in
`references/pyscenedetect/UPSTREAM.md` to tag `v0.6.7.1-release`, commit
`f8e1914f9057a2d9692738ca0fece14fc4b2ecad`. Keep that reference unchanged
unless a task explicitly asks to update or add an upstream reference. Scene
dataset evaluator reports include `pyscenedetectBaseline: "0.6.7.1"` so local
benchmark JSON can be compared without guessing which upstream line was used.

PySceneDetect `0.7` was released on 2026-05-03 and is tracked as a future
parity lane, not a silent replacement for the current target. The latest
documentation lists `save-fcp` and `save-qp`, and the migration guide describes
the timestamp/VFR overhaul and command/API changes:

| 0.7 area | Upstream change | Rust parity status |
| --- | --- | --- |
| VFR/timestamps | Backends return PTS-backed positions, `FrameTimecode` carries `time_base`/`pts`, and frame rates are rational `Fraction` values. | Partially cloned additively: `FramePosition` already carries `Timestamp`, legacy `FrameTimecode` stays layout-compatible, and `TimestampedFrameTimecode` preserves source PTS/timebase for new VFR-aware workflows. |
| CLI time inputs | Options that previously accepted frame numbers can also accept seconds and timecodes. | Not cloned yet; current scene evaluator still records cut frame numbers. |
| Removed/renamed CLI options | `detect-adaptive --min-delta-hsv` removed; global `--framerate` renamed to `--frame-rate`; `export-html` renamed to `save-html` with a deprecated alias. | Partially cloned additively: `vanalyze` accepts preferred `--frame-rate` and legacy `--framerate` spelling for detection command parsing without changing detector defaults. |
| New output commands | `save-fcp` and `save-qp` were added. | Writer support added in `video-analysis-output`; filesystem-writing CLI commands remain a later explicit scope. |
| Python output helpers | `write_scene_list_edl`, `write_scene_list_fcpx`, `write_scene_list_fcp7`, and `write_scene_list_otio` are available from `scenedetect.output`. | Pure Rust writer helpers now exist, with package-surface previews through `video.output.editListPlan`. |
| Runtime floor | Minimum Python version is now 3.10. | Documentation only for this Rust workspace. |

References: [PySceneDetect changelog](https://www.scenedetect.com/changelog/)
and [PySceneDetect 0.7 migration guide](https://www.scenedetect.com/docs/latest/api/migration_guide.html).

## Feature Matrix

| Area | Rust surface | PySceneDetect reference | Status |
| --- | --- | --- | --- |
| Content cuts | `ContentDetector` | `ContentDetector` | Covered by synthetic hard-cut parity tests |
| Adaptive cuts | `AdaptiveDetector` | `AdaptiveDetector` | Covered by delayed-window parity tests |
| Threshold fades | `ThresholdDetector` | `ThresholdDetector` | Covered by floor/ceiling fade tests |
| Histogram cuts | `HistogramDetector` | `HistogramDetector` | Covered by histogram-change tests |
| Hash cuts | `HashDetector` | `HashDetector` | Covered by perceptual texture-change tests |
| Flash filtering | `FlashFilter` | detector `min_scene_len` filtering | Covered by merge/suppress tests |
| Scene manager | `ScenePipeline` | `SceneManager` | Covered by root pipeline parity tests |
| Stats | `MetricsStore` and output CSV | `StatsManager` | Covered by metrics and stats CSV tests |
| Scene CSV/JSON | `video-analysis-output` | scene list/stats exporters | Covered by output parity tests |
| Splitting | `video-analysis-split` | split-video commands | Covered by split-plan parity tests |
| Image extraction | editing/output crates | `save_images` | Not cloned yet; should be planned as a separate image-export surface |

## Test Coverage Matrix

| PySceneDetect test area | Rust test |
| --- | --- |
| `tests/test_detectors.py` fast-cut detector cases | `crates/video/video-analysis-detectors/tests/pyscenedetect_detector_parity.rs` |
| `tests/test_detectors.py` fade cases | `crates/video/video-analysis-detectors/tests/pyscenedetect_detector_parity.rs` |
| `tests/test_scene_manager.py` scene list behavior | `tests/video_scene_pipeline_pyscenedetect_parity.rs` |
| `tests/test_stats_manager.py` score rows and reuse | `tests/video_scene_pipeline_pyscenedetect_parity.rs`, `tests/video_output_pyscenedetect_parity.rs` |
| `tests/test_frame_timecode.py` parsing and arithmetic | `tests/video_timecode_pyscenedetect_parity.rs` |
| `tests/test_video_splitter.py` split naming and durations | `crates/video/video-analysis-split/tests/split_plan_parity.rs` |
| `benchmark/` accuracy metrics | `scene_dataset_eval.rs` and `scripts/check_video_scene_eval.py` |

## Dataset Policy

Default verification uses deterministic synthetic fixtures generated by
`video-analysis-test-support`. Large videos are not checked in.

External corpora are opt-in:

```bash
scripts/setup_video_scene_benchmarks.sh
scripts/setup_video_scene_benchmarks.sh bbc
scripts/setup_video_scene_benchmarks.sh autoshot
AUTOSHOT_ARCHIVE=/downloads/AutoShot_test.tar.gz scripts/setup_video_scene_benchmarks.sh autoshot
scripts/setup_video_scene_benchmarks.sh verify
```

The setup script writes under `.test-corpora/video-scene/` by default and
accepts `BBC_ROOT` and `AUTOSHOT_ROOT` overrides for valid existing layouts. It
downloads BBC archives from the PySceneDetect benchmark Zenodo links when
`DOWNLOAD_LARGE_CORPORA=1` is set, or uses `BBC_FIXED_ARCHIVE` and
`BBC_VIDEOS_ARCHIVE` local zip overrides.

AutoShot is downloaded from the PySceneDetect Google Drive file through
`gdown`. The script prefers a `gdown` executable on `PATH`; if one is not
available, it creates an ignored local virtual environment under
`.external-test-tools/video-scene-python-venv` and installs
`${GDOWN_PIP_PACKAGE:-gdown==5.2.0}`. If Google Drive blocks the
non-interactive download, download `AutoShot_test.tar.gz` separately and provide
it with `AUTOSHOT_ARCHIVE=/path/to/AutoShot_test.tar.gz`.

Run the bounded BBC smoke evaluation with:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- --dataset bbc --root .test-corpora/video-scene/BBC --detector content --video-id bbc_10 --resize-width 320 --progress --resume --max-runtime-seconds 3300 --output target/video-scene/content-bbc-smoke.json
python3 scripts/check_video_scene_eval.py target/video-scene/content-bbc-smoke.json --allow-partial --tolerance-frames 0
```

For a broader local sample, run:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- --dataset bbc --root .test-corpora/video-scene/BBC --detector content --video-id bbc_03 --video-id bbc_06 --video-id bbc_10 --resize-width 320 --progress --resume --max-runtime-seconds 3300 --output target/video-scene/content-bbc-subset-broad.json
python3 scripts/check_video_scene_eval.py target/video-scene/content-bbc-subset-broad.json --allow-partial --tolerance-frames 0
python3 scripts/summarize_video_scene_eval.py target/video-scene/content-bbc-subset-broad.json --tolerance-frames 0 --output target/video-scene/content-bbc-subset-broad-summary.json
```

Current strict three-video BBC subset baseline:

```json
{
  "dataset": "bbc",
  "detector": "content",
  "videos": ["bbc_03", "bbc_06", "bbc_10"],
  "resizeWidth": 320,
  "toleranceFrames": 0,
  "recall": 0.8505311077389985,
  "precision": 0.7955997161107168,
  "f1": 0.822148881554822,
  "correct": 1121,
  "predicted": 1409,
  "groundTruth": 1318,
  "falsePositives": 288,
  "falseNegatives": 197,
  "avgElapsedMs": 116339.26632966667,
  "complete": true
}
```

Per-video baseline diagnostics identify `bbc_10` as the dominant precision
driver: it contributes 148 false positives, compared with 123 for `bbc_06` and
17 for `bbc_03`. The baseline summary also reports false-positive distance
buckets and clusters; for this run, `bbc_10` contributes 64 clustered false
positives.

The local improvement target for the same strict subset is `F1 >= 0.85`,
`precision >= 0.83`, `recall >= 0.82`, `complete == true`, videos
`bbc_03`, `bbc_06`, `bbc_10`, resize width `320`, and tolerance `0` frames.

Reproduce the content/adaptive parameter sweep with:

```bash
python3 scripts/sweep_video_scene_eval.py \
  --dataset bbc \
  --root .test-corpora/video-scene/BBC \
  --video-id bbc_03 \
  --video-id bbc_06 \
  --video-id bbc_10 \
  --resize-width 320 \
  --max-runtime-seconds 3300 \
  --output target/video-scene/bbc-subset-sweep.json
```

The sweep writes one aggregate JSON file and per-trial evaluator reports under
`target/video-scene/trials/`. Use `--resume` to reuse complete trial reports.
Use `--detector-kind content|adaptive|all`, `--max-trials N`, and repeatable
`--only-trial-id ID` to run long sweeps in stages. The sweep output records the
documented `target`, `bestPassing`, and `bestOverall`. `bestPassing` is set only
when a complete trial meets all target gates; `bestOverall` names the best
complete trial by F1, precision, recall, elapsed time, and content-detector
preference.

`postFilterWindow` is a post-detector duplicate suppression window. A value of
`0` disables it. A positive value retains the first predicted cut and removes
later predicted cuts while `nextCut - keptCut < postFilterWindow`; cuts exactly
at the window distance are retained.

### Tuned BBC subset result

Focused content trials improved precision but did not pass the full target. The
best measured content trial was:

```json
{
  "trialId": "content-th30-min15-merge-post0",
  "recall": 0.8414264036418816,
  "precision": 0.8300898203592815,
  "f1": 0.8357196684250189,
  "correct": 1109,
  "predicted": 1336,
  "groundTruth": 1318,
  "complete": true
}
```

The first adaptive precision trial passed all gates and is promoted as an
explicit benchmark preset. The preset does not change `ContentDetector::default()`,
`AdaptiveDetector` behavior, or any detector defaults; it only applies when
`--preset bbc-content-tuned` is supplied.

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- \
  --dataset bbc \
  --root .test-corpora/video-scene/BBC \
  --preset bbc-content-tuned \
  --video-id bbc_03 \
  --video-id bbc_06 \
  --video-id bbc_10 \
  --resize-width 320 \
  --progress \
  --resume \
  --max-runtime-seconds 3300 \
  --output target/video-scene/bbc-content-tuned-preset.json

python3 scripts/check_video_scene_eval.py \
  target/video-scene/bbc-content-tuned-preset.json \
  --tolerance-frames 0 \
  --min-f1 0.85
```

Effective preset configuration:

```json
{
  "detector": "adaptive",
  "configuration": {
    "preset": "bbc-content-tuned",
    "resizeWidth": 320,
    "adaptiveThreshold": 3.0,
    "adaptiveWindowWidth": 2,
    "adaptiveMinContentVal": 15.0,
    "minSceneLen": 15,
    "postFilterWindow": 0
  }
}
```

Tuned strict three-video BBC subset result:

```json
{
  "dataset": "bbc",
  "detector": "adaptive",
  "videos": ["bbc_03", "bbc_06", "bbc_10"],
  "resizeWidth": 320,
  "toleranceFrames": 0,
  "recall": 0.8672230652503794,
  "precision": 0.957286432160804,
  "f1": 0.9100318471337581,
  "correct": 1143,
  "predicted": 1194,
  "groundTruth": 1318,
  "falsePositives": 51,
  "falseNegatives": 175,
  "avgElapsedMs": 90989.80334166666,
  "complete": true
}
```

The tuned diagnostics reduce aggregate false positives from 288 to 51. `bbc_10`
remains the hardest video, but its false positives drop from 148 to 27.

The full BBC corpus remains an explicit long-running opt-in:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- --dataset bbc --root .test-corpora/video-scene/BBC --detector content --progress --resume --output target/video-scene/content-bbc-full.json
python3 scripts/check_video_scene_eval.py target/video-scene/content-bbc-full.json --tolerance-frames 0
```

The evaluator reports recall, precision, F1, and average elapsed time. Strict
performance thresholds are intentionally not part of default CI because machine
variance would make them noisy.

### AutoShot resize regression

The vertical AutoShot sample `51856804342` is the resize ingest regression case:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- \
  --dataset autoshot \
  --root .test-corpora/video-scene/AutoShot \
  --detector content \
  --video-id 51856804342 \
  --resize-width 320 \
  --output target/video-scene/rust-content-autoshot-51856804342-resize.json

python3 scripts/pyscenedetect_scene_dataset_eval.py \
  --dataset autoshot \
  --root .test-corpora/video-scene/AutoShot \
  --detector content \
  --video-id 51856804342 \
  --resize-width 320 \
  --output target/video-scene/pyscenedetect-content-autoshot-51856804342-resize.json

python3 scripts/compare_scene_detector_speed.py \
  --rust-report target/video-scene/rust-content-autoshot-51856804342-resize.json \
  --pyscenedetect-report target/video-scene/pyscenedetect-content-autoshot-51856804342-resize.json \
  --output target/video-scene/content-autoshot-51856804342-resize-speed-compare.json
```

Current local result for this sample at resize width `320`:

```json
{
  "rustPredictedCuts": [111, 207, 277, 353, 399, 477, 535],
  "pyscenedetectPredictedCuts": [111, 207, 277, 353, 402, 476, 535],
  "onlyRustCuts": [399, 477],
  "onlyPyscenedetectCuts": [402, 476],
  "pairedFrameDeltas": [0, 0, 0, 0, -3, 1, 0],
  "f1": 0.6250000000000001,
  "precision": 0.7142857142857143,
  "recall": 0.5555555555555556
}
```

## Equal Speed Comparison

Use the equal benchmark harness to compare Rust and PySceneDetect on the same
BBC videos, resize width, detector parameters, annotation loading, frame
numbering, and timing scope:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- \
  --dataset bbc \
  --root .test-corpora/video-scene/BBC \
  --detector content \
  --video-id bbc_03 \
  --video-id bbc_06 \
  --video-id bbc_10 \
  --resize-width 320 \
  --progress \
  --resume \
  --max-runtime-seconds 3300 \
  --output target/video-scene/rust-content-bbc-subset-broad.json

python3 scripts/pyscenedetect_scene_dataset_eval.py \
  --dataset bbc \
  --root .test-corpora/video-scene/BBC \
  --detector content \
  --video-id bbc_03 \
  --video-id bbc_06 \
  --video-id bbc_10 \
  --resize-width 320 \
  --output target/video-scene/pyscenedetect-content-bbc-subset-broad.json

python3 scripts/compare_scene_detector_speed.py \
  --rust-report target/video-scene/rust-content-bbc-subset-broad.json \
  --pyscenedetect-report target/video-scene/pyscenedetect-content-bbc-subset-broad.json \
  --output target/video-scene/content-bbc-speed-compare.json
```

Per-video timing fields use these scopes:

- `elapsedMs`: end-to-end evaluator time for the video.
- `decodeResizeElapsedMs`: time spent pulling decoded/resized frames.
- `detectorElapsedMs`: time spent in detector `process_frame` and finalization.
- `frameCount` and `effectiveFps`: processed frame count and end-to-end video
  throughput.

For Rust detector-only timing on real decoded frames, decode once and run the
detector repeatedly:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_detector_real_frame_benchmark -- \
  --input .test-corpora/video-scene/BBC/videos/bbc_03.mp4 \
  --resize-width 320 \
  --detector content \
  --iterations 5 \
  --output target/video-scene/rust-real-frame-content-bbc_03.json
```

## Improvement Policy

New Rust-native detectors or composite strategies may exceed PySceneDetect
accuracy or speed. Existing detector defaults should keep parity tests stable.
When a deliberate improvement changes default detector output, update this
document with the reason, expected behavior, and any compatibility setting that
preserves PySceneDetect-like output.

PySceneDetect-compatible defaults remain unchanged unless a later change
explicitly promotes a measured tuned preset. Local benchmark outputs stay under
`target/video-scene/` and are not committed by default.
