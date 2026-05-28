import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-opencv-backend-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-opencv-backend",
  title: "Video Analysis Opencv Backend",
  description: "Optional OpenCV backend contracts for COLMAP-like video-analysis pipelines.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-opencv-backend",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
