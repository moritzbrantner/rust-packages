import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-colmap-backend-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-colmap-backend",
  title: "Video Analysis Colmap Backend",
  description: "COLMAP compatibility backend and parity reporting for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-colmap-backend",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
