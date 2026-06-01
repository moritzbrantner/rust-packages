import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/three-d-processing-mesh-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-processing-mesh",
  title: "Three D Processing Mesh",
  description: "Triangle mesh validation and geometry helpers for video-analysis.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-processing-mesh",
    standaloneRoute: "",
  },
  defaultOperation: "threeD.mesh.diagnostics",
  featuredOperations: ["threeD.mesh.diagnostics", "threeD.mesh.sample", "threeD.mesh.repairPreview", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["threeD.mesh.diagnostics", "threeD.mesh.sample"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "threeD.mesh.repairPreview"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
