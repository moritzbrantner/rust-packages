import initGenerated, { initSync, resampleLineFlat, resampleRingFlat } from "./pkg/index.js";

const wasmUrl = new URL("./pkg/index_bg.wasm", import.meta.url);

export { initSync, resampleLineFlat, resampleRingFlat };

export default async function init(moduleOrPath) {
  if (moduleOrPath) {
    return initGenerated({ module_or_path: moduleOrPath });
  }

  if (typeof process !== "undefined" && process.versions?.node) {
    const { readFile } = await import("node:fs/promises");
    const wasmPath = resolveNodeWasmPath(wasmUrl);

    if (wasmPath) {
      return initGenerated({ module_or_path: await readFile(wasmPath) });
    }

    return initGenerated({ module_or_path: wasmUrl });
  }

  return initGenerated({ module_or_path: wasmUrl });
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
