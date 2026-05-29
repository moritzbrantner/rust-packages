import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/three-d-processing-io-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "three-d-processing-io",
  title: "Three D Processing IO",
  description: "Mesh and point-cloud interchange formats for video-analysis.",
  domain: "three-d",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/three-d-processing-io",
    standaloneRoute: "",
  },
  defaultOperation: "threeD.io.supportedFormats",
  featuredOperations: ["threeD.io.supportedFormats", "threeD.io.objPreview", "threeD.io.plyPreview", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["threeD.io.supportedFormats"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "threeD.io.objPreview", "threeD.io.plyPreview"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
