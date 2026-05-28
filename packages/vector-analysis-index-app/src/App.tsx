import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/vector-analysis-index-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "vector-analysis-index",
  title: "Vector Analysis Index",
  description: "Exact in-memory vector search for video-analysis.",
  domain: "vector",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/vector-analysis-index",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
