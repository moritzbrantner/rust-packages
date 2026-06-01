import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/audio-analysis-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-core",
  title: "Audio Analysis Core",
  description: "Shared audio frame conversion, windowing, and streaming helpers for video-analysis.",
  domain: "audio",
  defaultOperation: "audio.levels",
  featuredOperations: ["audio.levels", "audio.frames", "audio.timestamps", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["audio.levels", "audio.frames"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "audio.timestamps"],
    },
  ],
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-core",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
