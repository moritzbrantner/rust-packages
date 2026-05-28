import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/three-d-processing-mesh-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-processing-mesh",
  title: "Three D Processing Mesh",
  description: "Triangle mesh validation and geometry helpers for video-analysis.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-processing-mesh",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
