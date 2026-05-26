import initWasm, { initSync } from "./pkg/dense_data_wasm.js";
import * as wasmModule from "./pkg/dense_data_wasm.js";

export async function init() {
  return wasmModule;
}

if (typeof process !== "undefined" && process.versions?.node) {
  const { readFileSync } = await import("node:fs");
  const wasmUrl = new URL("./pkg/dense_data_wasm_bg.wasm", import.meta.url);
  const wasmPath =
    wasmUrl.protocol === "file:"
      ? wasmUrl
      : decodeURIComponent(wasmUrl.pathname.replace(/^\/@fs/, ""));
  initSync({ module: readFileSync(wasmPath) });
} else {
  await initWasm();
}

export const NumericSeriesIndex = wasmModule.NumericSeriesIndex;

export function packageSurface() {
  return wasmModule.packageSurface();
}

export function runOperation(request) {
  return wasmModule.runOperation(request);
}
