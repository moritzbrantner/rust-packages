import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-recognition-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-recognition",
  title: "Audio Analysis Recognition",
  description: "Deterministic audio embeddings and similarity search for video-analysis.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-recognition",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
