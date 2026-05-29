import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/animation-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "animation-core",
  title: "Animation Core",
  description: "Shared timeline, keyframe, track, clip, and skeleton contracts for animation workflows.",
  domain: "animation",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/animation-core",
    standaloneRoute: "",
  },
  defaultOperation: "animation.timeline.summary",
  featuredOperations: ["animation.timeline.summary", "animation.keyframes.sample", "animation.easing.preview", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["animation.timeline.summary", "animation.keyframes.sample"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "animation.easing.preview"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
