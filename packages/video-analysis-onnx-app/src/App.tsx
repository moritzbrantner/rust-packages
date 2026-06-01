import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/video-analysis-onnx-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "video-analysis-onnx",
  title: "Video Analysis ONNX",
  description: "ONNX-backed video model inference adapters for video-analysis.",
  domain: "video",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/video-analysis-onnx",
    standaloneRoute: "",
  },
  defaultOperation: "video.onnx.modelSummary",
  featuredOperations: ["video.onnx.modelSummary", "video.onnx.decodeDetections", "video.onnx.inputPlan", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run the main package workflow.",
      operations: ["video.onnx.modelSummary", "video.onnx.decodeDetections"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect inputs, plans, metadata, and diagnostic helpers.",
      operations: ["describe", "video.onnx.inputPlan"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
