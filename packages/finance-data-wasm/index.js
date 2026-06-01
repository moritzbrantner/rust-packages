import initWasm, {
  FinanceDataSeriesIndex as WasmFinanceDataSeriesIndex,
  initSync,
  packageSurface as wasmPackageSurface,
  runOperation as wasmRunOperation,
} from "./pkg/finance_data_wasm.js";
import * as wasmModule from "./pkg/finance_data_wasm.js";

let initialized = false;
let wasmModulePromise;

export async function init() {
  if (!initialized) {
    if (isNodeLikeRuntime()) {
      initializeNodeSync();
    } else {
      wasmModulePromise ??= initWasm().then(() => {
        initialized = true;
        return wasmModule;
      });
      await wasmModulePromise;
    }
  }

  return wasmModule;
}

export async function packageSurface() {
  const module = await init();
  return module.packageSurface();
}

export async function runOperation(request) {
  const module = await init();
  return module.runOperation(request);
}

export async function createSeriesIndex(series) {
  const module = await init();
  return new module.FinanceDataSeriesIndex(series);
}

if (!isNodeLikeRuntime()) {
  await init();
}

export class FinanceDataSeriesIndex {
  constructor(series) {
    initializeNodeSync();
    this.inner = new WasmFinanceDataSeriesIndex(series);
  }

  getBounds() {
    return this.inner.getBounds();
  }

  getBars(query) {
    return this.inner.getBars(query);
  }

  getDownsampledBars(query) {
    return this.inner.getDownsampledBars(query);
  }

  getReturns(query) {
    return this.inner.getReturns(query);
  }

  getCompactReturns(query) {
    return this.inner.getCompactReturns(query);
  }

  getRiskSummary(query) {
    return this.inner.getRiskSummary(query);
  }

  free() {
    this.inner.free();
  }
}

export function packageSurfaceSync() {
  initializeNodeSync();
  return wasmPackageSurface();
}

export function runOperationSync(request) {
  initializeNodeSync();
  return wasmRunOperation(request);
}

function initializeNodeSync() {
  if (initialized || !isNodeLikeRuntime()) {
    return;
  }

  const wasmFile = "./pkg/" + "finance_data_wasm_bg.wasm";
  const wasmPath = new URL(wasmFile, import.meta.url);
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
      `finance-data-wasm could not synchronously read ${wasmPath.toString()}.`,
    );
  }

  const path =
    wasmPath.protocol === "file:"
      ? wasmPath
      : decodeURIComponent(wasmPath.pathname.replace(/^\/@fs/, ""));

  return fs.readFileSync(path);
}
