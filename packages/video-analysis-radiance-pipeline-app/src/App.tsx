import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/video-analysis-radiance-pipeline-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-radiance-pipeline",
  title: "Video Analysis Radiance Pipeline",
  description: "Typed radiance project loading, validation, summaries, and CPU previews for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-radiance-pipeline",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
