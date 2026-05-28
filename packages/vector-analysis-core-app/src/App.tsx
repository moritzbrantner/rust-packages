import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/vector-analysis-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "vector-analysis-core",
  title: "Vector Analysis Core",
  description: "Dense vector validation and metrics for video-analysis.",
  domain: "vector",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/vector-analysis-core",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
