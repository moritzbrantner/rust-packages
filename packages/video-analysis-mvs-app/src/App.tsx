import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-mvs-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-mvs",
  title: "Video Analysis MVS",
  description: "Multi-View Stereo backend contracts and dense reconstruction outputs for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-mvs",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
