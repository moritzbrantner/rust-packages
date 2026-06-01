import { PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/image-analysis-onnx-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "image-analysis-onnx",
  title: "Image Analysis ONNX",
  description: "ONNX-backed still-image preprocessing and inference adapters for video-analysis.",
  domain: "image",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/image-analysis-onnx",
    standaloneRoute: "",
  },
  defaultOperation: "image.onnx.preprocess",
  featuredOperations: ["image.onnx.preprocess", "image.onnx.decodeDetections", "image.onnx.preprocessing", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic ONNX image preprocessing and output decoding workflows.",
      operations: ["image.onnx.preprocess", "image.onnx.decodeDetections"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect preprocessing configuration and package metadata.",
      operations: ["image.onnx.preprocessing", "describe"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
