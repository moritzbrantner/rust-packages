import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-pitch-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-pitch",
  title: "Audio Analysis Pitch",
  description: "Autocorrelation pitch detection for video-analysis audio pipelines.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-pitch",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
