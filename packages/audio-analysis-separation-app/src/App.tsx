import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/audio-analysis-separation-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "audio-analysis-separation",
  title: "Audio Analysis Separation",
  description: "Demucs-based audio stem separation command wrapper for video-analysis.",
  domain: "audio",
  defaultOperation: "audio.separation.expectedStems",
  featuredOperations: ["audio.separation.expectedStems", "audio.separation.plan", "audio.separation.models", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["audio.separation.expectedStems"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "audio.separation.models", "audio.separation.plan"],
    },
  ],
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/audio-analysis-separation",
    standaloneRoute: "",
  },
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
