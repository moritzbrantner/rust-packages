import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-core-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-core",
  title: "Text Core",
  description: "Shared text documents, tokenization, spans, and statistics for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-core",
    standaloneRoute: "",
  },
  defaultOperation: "text.tokenize",
  featuredOperations: ["text.tokenize", "text.statistics", "text.normalize", "text.boundaries", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run text statistics, normalization, tokenization, and boundary workflows.",
      operations: ["text.tokenize", "text.statistics", "text.normalize", "text.boundaries"],
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
      id: "tokenize-sample",
      label: "Tokenize",
      operation: "text.tokenize",
      description: "Tokenize a short sentence with punctuation spans.",
      input: { text: "Hello, Berlin 2026.", includePunctuation: true },
    },
    {
      id: "normalize-sample",
      label: "Normalize",
      operation: "text.normalize",
      description: "Normalize casing and whitespace.",
      input: { text: "  Hello   WORLD  ", lowercase: true, normalizeWhitespace: true },
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
