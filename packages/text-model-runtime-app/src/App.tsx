import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-model-runtime-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-model-runtime",
  title: "Text Model Runtime",
  description: "Shared tokenizer and native text model runtime traits for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-model-runtime",
    standaloneRoute: "",
  },
  defaultOperation: "runtime.tokenizeSummary",
  featuredOperations: ["runtime.tokenizeSummary", "runtime.softmax", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic tokenizer summary workflows.",
      operations: ["runtime.tokenizeSummary"],
    },
    {
      id: "support",
      label: "Support",
      description: "Run reusable runtime support helpers.",
      operations: ["runtime.softmax"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect package metadata and operation support.",
      operations: ["describe"],
    },
  ],
  presets: [
    {
      id: "tokenize-summary",
      label: "Tokenize",
      operation: "runtime.tokenizeSummary",
      description: "Build a deterministic whitespace-token summary.",
      input: { text: "Rust text runtime", maxTokens: 8 },
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
