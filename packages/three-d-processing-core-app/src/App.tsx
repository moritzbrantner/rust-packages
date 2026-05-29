import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/three-d-processing-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-processing-core",
  title: "Three D Processing Core",
  description: "Shared 3D geometry primitives and transforms for video-analysis.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-processing-core",
    standaloneRoute: "",
  },
  defaultOperation: "threeD.pointCloud.summary",
  featuredOperations: ["threeD.pointCloud.summary", "threeD.pointCloud.downsample", "threeD.geometry.intersections", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["threeD.pointCloud.summary", "threeD.pointCloud.downsample", "threeD.geometry.intersections"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
