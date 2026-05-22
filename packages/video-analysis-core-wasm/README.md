# @mb-rust/video-analysis-core-wasm

WASM bindings for browser-friendly video analysis core helpers.

```js
import init, { analyzeVideoFrame } from "@mb-rust/video-analysis-core-wasm";

await init();
const result = analyzeVideoFrame(new Uint8Array([255, 0, 0]), 1, 1, "rgb24", 0, 24, 1, 3);
```
