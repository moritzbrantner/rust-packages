import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-radiance-fields-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-radiance-fields",
  title: "Video Analysis Radiance Fields",
  description: "Radiance-field cameras, rays, and volumes for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-radiance-fields",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
