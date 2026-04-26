# video-analysis-use-cases

Runnable end-to-end workflows built from the `video-analysis` crates.

## Feature flags

- No optional feature flags today.

## Example

```bash
cargo run -p video-analysis-use-cases -- youtube-video \
  --url "https://www.youtube.com/watch?v=dQw4w9WgXcQ" \
  --work-dir use-case-output/youtube-video \
  --output use-case-output/youtube-video/analysis.json

cargo run -p video-analysis-use-cases -- video-red-cars \
  --input ./traffic.mp4 \
  --vehicle-detector-command python3 \
  --vehicle-detector-arg scripts/opencv_red_car_detector.py

cargo run -p video-analysis-use-cases -- audio-voice-analysis \
  --input ./voice.wav

cargo run -p video-analysis-use-cases -- image-person-edit \
  --input ./portrait.png \
  --prompt "replace the detected person with a marble statue" \
  --model flux1-dev.safetensors \
  --person-detector-command python3 \
  --person-detector-arg scripts/opencv_person_detector.py
```

## Config files

If `video-analysis-use-cases.conf` is present in the current directory, the
binary reads it automatically as shell-style arguments before the real command
line. Direct CLI flags still win when the same option is set in both places.

## Related crates

- `video-analysis-cli`
- `video-analysis-models`
- `@video-analysis/ui`
