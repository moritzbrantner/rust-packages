# video-analysis-detectors

Scene detection algorithms and detector adapters for `moritzbrantner-video-analysis`.

The crate tracks PySceneDetect behavior parity for detector outputs and common
workflows while keeping Rust-first APIs and workspace crate boundaries.

## Feature flags

- No optional feature flags today.

## Runtime surface

- `video.detectors.registry` summarizes detector families and score algorithms.
- `video.detectors.flashFilter` runs the deterministic flash filter over frame
  threshold decisions.
- `video.detectors.compositePlan` validates weighted composite detector
  configuration without processing media.

## Detector families

- `ContentDetector`: HSV/luma/edge frame-difference cuts, matching the
  PySceneDetect content detector workflow.
- `AdaptiveDetector`: content-score ratio over a delayed window.
- `ThresholdDetector`: fade out/in detection with floor and ceiling methods.
- `HistogramDetector`: luma histogram correlation cuts.
- `HashDetector`: perceptual hash distance cuts.
- `WeightedCompositeDetector`: Rust-native composite scoring over detector
  algorithms.

Known intentional differences:

- Public APIs are idiomatic Rust and do not clone PySceneDetect class names or
  CLI flags exactly.
- Default tests use synthetic fixtures. Large PySceneDetect resource videos,
  BBC, and AutoShot are opt-in local corpora.
- Dataset evaluation reports include both predictions and loaded ground-truth
  cuts so downstream scripts can recompute metrics.

## Example

```rust,ignore
use video_analysis_detectors::ContentDetector;

let detector = ContentDetector::new(27.0, 15);
let _ = detector;
```

## Verification

Default detector tests:

```bash
cargo test -p moritzbrantner-video-analysis-detectors
```

Ignored external smoke test:

```bash
cargo test -p moritzbrantner-video-analysis-detectors --test external_me_at_the_zoo -- --ignored
```

Criterion benchmarks:

```bash
cargo bench -p moritzbrantner-video-analysis-detectors
bun run video-native:bench
```

Opt-in dataset evaluation:

```bash
scripts/setup_video_scene_benchmarks.sh
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- --dataset bbc --root .test-corpora/video-scene/BBC --detector content --video-id bbc_10 --resize-width 320 --progress --resume --max-runtime-seconds 3300 --output target/video-scene/content-bbc-smoke.json
python3 scripts/check_video_scene_eval.py target/video-scene/content-bbc-smoke.json --allow-partial
```

For a broader local sample, run three representative BBC videos:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- --dataset bbc --root .test-corpora/video-scene/BBC --detector content --video-id bbc_03 --video-id bbc_06 --video-id bbc_10 --resize-width 320 --progress --resume --max-runtime-seconds 3300 --output target/video-scene/content-bbc-subset.json
python3 scripts/check_video_scene_eval.py target/video-scene/content-bbc-subset.json --allow-partial
```

The full BBC corpus is still available as an explicit long-running benchmark:

```bash
cargo run --release -p moritzbrantner-video-analysis-detectors --example scene_dataset_eval -- --dataset bbc --root .test-corpora/video-scene/BBC --detector content --progress --resume --output target/video-scene/content-bbc-full.json
python3 scripts/check_video_scene_eval.py target/video-scene/content-bbc-full.json
```

## Related crates

- `video-analysis-core`
- `video-analysis-cli`
- `video-analysis-split`
