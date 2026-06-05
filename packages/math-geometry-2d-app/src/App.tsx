import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/math-geometry-2d-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "math-geometry-2d",
  title: "Math Geometry 2d",
  description: "Shared 2D geometry contracts for multimodal image, video, and layout processing.",
  domain: "math",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/math-geometry-2d",
    standaloneRoute: "",
  },
  defaultOperation: "geometry.bounds",
  featuredOperations: [
    "geometry.bounds",
    "geometry.transform",
    "geometry.intersections",
    "geometry.overlap",
    "geometry.segmentIntersection",
    "geometry.polygonSummary",
    "describe",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: [
        "geometry.bounds",
        "geometry.transform",
        "geometry.intersections",
        "geometry.overlap",
        "geometry.segmentIntersection",
        "geometry.polygonSummary",
      ],
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
