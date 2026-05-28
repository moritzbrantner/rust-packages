import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-speakers-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-speakers",
  title: "Audio Analysis Speakers",
  description: "Speaker embeddings, enrollment, identification, VAD, and diarization APIs for video-analysis.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-speakers",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
