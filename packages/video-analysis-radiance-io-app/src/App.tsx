import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-radiance-io-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-radiance-io",
  title: "Video Analysis Radiance IO",
  description: "COLMAP, Nerfstudio, and PLY I/O for video-analysis radiance workflows.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-radiance-io",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
