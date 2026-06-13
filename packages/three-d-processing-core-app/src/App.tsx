import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/three-d-processing-core-wasm";

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
  defaultOperation: "threeD.camera.project",
  featuredOperations: [
    "threeD.camera.project",
    "threeD.camera.pixelRay",
    "threeD.camera.viewMatrix",
    "threeD.transform.apply",
    "threeD.convert.colmapPose",
  ],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run point cloud, geometry, transform, camera, and conversion workflows.",
      operations: [
        "threeD.pointCloud.summary",
        "threeD.pointCloud.downsample",
        "threeD.geometry.intersections",
        "threeD.transform.compose",
        "threeD.transform.inverse",
        "threeD.transform.apply",
        "threeD.camera.project",
        "threeD.camera.pixelRay",
        "threeD.camera.viewMatrix",
        "threeD.camera.projectionMatrix",
        "threeD.convert.colmapPose",
        "threeD.convert.gltfMatrix",
      ],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "threeD.debug.matrixInspect", "threeD.debug.rotationInspect", "threeD.debug.transformDiagnostics"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
