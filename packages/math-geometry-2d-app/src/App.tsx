import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/math-geometry-2d-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-geometry-2d",
  title: "Math Geometry 2d",
  description: "Shared 2D geometry contracts for multimodal image, video, and layout processing.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-geometry-2d",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
