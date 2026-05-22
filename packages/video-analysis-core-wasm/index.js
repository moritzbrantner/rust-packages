import initGenerated, {
  analyzeVideoFrame,
  frameTimecode,
  initSync,
  parseFrameTimecode,
  scenesFromCutFrames,
} from "./pkg/index.js";

const wasmUrl = new URL("./pkg/index_bg.wasm", import.meta.url);

export { analyzeVideoFrame, frameTimecode, initSync, parseFrameTimecode, scenesFromCutFrames };

export default async function init(moduleOrPath) {
  if (moduleOrPath) {
    return initGenerated(moduleOrPath);
  }

  if (typeof process !== "undefined" && process.versions?.node) {
    const { readFile } = await import("node:fs/promises");
    const wasmPath = resolveNodeWasmPath(wasmUrl);

    if (wasmPath) {
      return initGenerated(await readFile(wasmPath));
    }

    return initGenerated(wasmUrl);
  }

  return initGenerated(wasmUrl);
}

function resolveNodeWasmPath(url) {
  if (url.protocol === "file:") {
    return url;
  }

  if ((url.protocol === "http:" || url.protocol === "https:") && url.pathname.startsWith("/@fs/")) {
    return decodeURIComponent(url.pathname.slice(4));
  }

  return null;
}
