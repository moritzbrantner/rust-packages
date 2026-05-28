import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-gaussian-splatting-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-gaussian-splatting",
  title: "Video Analysis Gaussian Splatting",
  description: "Gaussian splatting primitives and CPU compositing for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-gaussian-splatting",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
