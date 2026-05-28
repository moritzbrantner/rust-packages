import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-processing-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-processing",
  title: "Audio Analysis Processing",
  description: "Realtime-safe audio transforms and processed sources for video-analysis.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-processing",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
