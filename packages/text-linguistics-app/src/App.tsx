import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-linguistics-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-linguistics",
  title: "Text Linguistics",
  description: "Local model-backed linguistic analysis pipeline for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-linguistics",
    standaloneRoute: "",
  },
  defaultOperation: "linguistics.analyze",
  featuredOperations: ["linguistics.analyze", "linguistics.entities", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic linguistic analysis and entity extraction.",
      operations: ["linguistics.analyze", "linguistics.entities"],
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
      id: "linguistic-analysis",
      label: "Analyze",
      operation: "linguistics.analyze",
      description: "Run the fast deterministic linguistic profile.",
      input: { text: "Alice presented the tokenizer roadmap in Berlin.", profile: "fast" },
    },
    {
      id: "entities",
      label: "Entities",
      operation: "linguistics.entities",
      description: "Extract entities and relations from a short sentence.",
      input: { text: "Alice presented the tokenizer roadmap in Berlin." },
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
