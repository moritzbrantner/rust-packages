# @mb-rust/text-linguistics-wasm

WASM bindings for `text-linguistics`.

```js
import init, { analyzeTextLinguistics } from "@mb-rust/text-linguistics-wasm";

await init();
const analysis = analyzeTextLinguistics("Alice works at OpenAI in Berlin.", {
  entityRecognition: "heuristic",
});
```

Native BERT-NER inference uses the server/CLI Candle runtime. Browser callers can
pass BERT-NER token predictions as `bertNerPredictions` to reuse the Rust entity
merging, canonicalization, and analysis payload through wasm.
