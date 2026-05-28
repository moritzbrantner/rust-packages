import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-transform-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-transform",
  title: "Video Analysis Transform",
  description: "Filtering, joins, grouping, and resampling for video-analysis datasets.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-transform",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
