# @mb-rust/audio-analysis-core-wasm

WASM bindings for browser-friendly audio analysis helpers.

```js
import init, { analyzeAudioSamples } from "@mb-rust/audio-analysis-core-wasm";

await init();
const result = analyzeAudioSamples(new Float32Array([0, 1, 0, -1]), {
  sampleRate: 48000,
});
```
