import initWasm, {
  NumericSeriesIndex as WasmNumericSeriesIndex,
  initSync,
  packageSurface as wasmPackageSurface,
  runOperation as wasmRunOperation,
} from "./pkg/moenarch_dense_data_wasm.js";
import * as wasmModule from "./pkg/moenarch_dense_data_wasm.js";

let initialized = false;

export async function init() {
  if (!initialized) {
    if (isNodeLikeRuntime()) {
      initializeNodeSync();
    } else {
      await initWasm();
      initialized = true;
    }
  }

  return wasmModule;
}

if (isNodeLikeRuntime()) {
  initializeNodeSync();
} else {
  await init();
}

export class NumericSeriesIndex {
  constructor(points) {
    initializeNodeSync();
    this.inner = new WasmNumericSeriesIndex(points);
  }

  getSeriesBounds() {
    return this.inner.getSeriesBounds();
  }

  getBinnedSeries(query) {
    return this.inner.getBinnedSeries(query);
  }

  getChartSeries(query) {
    return this.inner.getChartSeries(query);
  }

  getHistogram(query) {
    return this.inner.getHistogram(query);
  }

  getHeatmap(query) {
    return this.inner.getHeatmap(query);
  }

  free() {
    this.inner.free();
  }
}

export function packageSurface() {
  initializeNodeSync();
  return wasmPackageSurface();
}

export function runOperation(request) {
  initializeNodeSync();
  return wasmRunOperation(request);
}

function initializeNodeSync() {
  if (initialized || !isNodeLikeRuntime()) {
    return;
  }

  const wasmPath = new URL(
    "./pkg/moenarch_dense_data_wasm_bg.wasm",
    import.meta.url,
  );
  const bytes = readNodeFileSync(wasmPath);

  initSync({ module: bytes });
  initialized = true;
}

function isNodeLikeRuntime() {
  return typeof process !== "undefined" && Boolean(process.versions?.node);
}

function readNodeFileSync(wasmPath) {
  const fs = process.getBuiltinModule?.("fs");

  if (!fs?.readFileSync) {
    throw new Error(
      `dense-data-wasm could not synchronously read ${wasmPath.toString()}.`,
    );
  }

  const path =
    wasmPath.protocol === "file:"
      ? wasmPath
      : decodeURIComponent(wasmPath.pathname.replace(/^\/@fs/, ""));

  return fs.readFileSync(path);
}
