import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/three-d-scene-svg-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-scene-svg",
  title: "Three D Scene Svg",
  description: "SVG-inspired declarative 3D scene documents and deterministic SVG preview rendering.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-scene-svg",
    standaloneRoute: "",
  },
  defaultOperation: "threeD.sceneSvg.summary",
  featuredOperations: ["threeD.sceneSvg.summary", "threeD.sceneSvg.exportSvg", "threeD.sceneSvg.renderPlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["threeD.sceneSvg.summary", "threeD.sceneSvg.exportSvg"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "threeD.sceneSvg.renderPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
