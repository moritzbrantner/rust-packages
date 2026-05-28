import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-test-support-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-test-support",
  title: "Audio Analysis Test Support",
  description: "Runtime surface for the audio-analysis-test-support library crate.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-test-support",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
