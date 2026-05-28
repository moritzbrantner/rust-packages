import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-generation-midi-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-generation-midi",
  title: "Audio Generation Midi",
  description: "MIDI-like note sequencing, Standard MIDI export, and audio rendering helpers for video-analysis.",
  domain: "audio",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-generation-midi",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
