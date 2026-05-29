import initWasm, {
  GeoPointIndex as WasmGeoPointIndex,
  initSync,
  packageSurface as wasmPackageSurface,
  runOperation as wasmRunOperation,
} from "./pkg/geo_viz_core_wasm.js";
import * as wasmModule from "./pkg/geo_viz_core_wasm.js";

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

export class GeoPointIndex {
  constructor(points, options) {
    initializeNodeSync();
    this.inner = new WasmGeoPointIndex(points, options ?? null);
  }

  getBounds() {
    return this.inner.getBounds();
  }

  getPointById(pointId) {
    return this.inner.getPointById(pointId);
  }

  getViewportAggregation(query) {
    return this.inner.getViewportAggregation(query);
  }

  getClusterExpansionZoom(clusterId) {
    return this.inner.getClusterExpansionZoom(clusterId);
  }

  getClusterLeaves(clusterId, limit, offset) {
    return this.inner.getClusterLeaves(clusterId, limit, offset);
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

  const wasmPath = new URL("./pkg/geo_viz_core_wasm_bg.wasm", import.meta.url);
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
      `geo-viz-core-wasm could not synchronously read ${wasmPath.toString()}.`,
    );
  }

  const path =
    wasmPath.protocol === "file:"
      ? wasmPath
      : decodeURIComponent(wasmPath.pathname.replace(/^\/@fs/, ""));

  return fs.readFileSync(path);
}
