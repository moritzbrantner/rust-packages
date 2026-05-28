import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/math-sparse-data-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-sparse-data",
  title: "Math Sparse Data",
  description: "Sparse vector and matrix contracts for text, retrieval, and feature indexing.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-sparse-data",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
