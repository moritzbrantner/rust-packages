import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-rhythm-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-rhythm",
  title: "Audio Analysis Rhythm",
  description: "Onset and tempo analysis for video-analysis audio pipelines.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-rhythm",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
