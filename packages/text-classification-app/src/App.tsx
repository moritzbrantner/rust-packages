import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-classification-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-classification",
  title: "Text Classification",
  description: "Text classification APIs, runtime selection, imported prediction handling, and deterministic fallbacks.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-classification",
    standaloneRoute: "",
  },
  defaultOperation: "classification.classify",
  featuredOperations: ["classification.classify", "classification.sentiment", "classification.zeroShot", "classification.models", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run classification, sentiment, and zero-shot workflows.",
      operations: ["classification.classify", "classification.sentiment", "classification.zeroShot"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect package metadata and model catalog helpers.",
      operations: ["classification.models", "describe"],
    },
  ],
  presets: [
    {
      id: "classify",
      label: "Classify",
      operation: "classification.classify",
      description: "Classify text with lexical fallback behavior.",
      input: { text: "rust is reliable", labels: ["positive", "negative"], model: { fallbackPolicy: "lexical_fallback" } },
    },
    {
      id: "zero-shot",
      label: "Zero-shot",
      operation: "classification.zeroShot",
      description: "Score candidate labels for short text.",
      input: { text: "rust text", labels: ["code", "music"], model: { fallbackPolicy: "lexical_fallback" } },
    },
  ],
  benchmarkScenarios: [
    {
      id: "lexical-classify",
      label: "Lexical Classify",
      operation: "classification.classify",
      input: { text: "rust text analysis is reliable and fast", labels: ["positive", "negative"], model: { fallbackPolicy: "lexical_fallback" } },
      iterations: 100,
      warmupIterations: 5,
      outputCountPath: ["predictions"],
    },
    {
      id: "sentiment",
      label: "Sentiment",
      operation: "classification.sentiment",
      input: { text: "the transcript workflow is reliable", model: { fallbackPolicy: "lexical_fallback" } },
      iterations: 100,
      warmupIterations: 5,
      outputCountPath: ["predictions"],
    },
    {
      id: "zero-shot",
      label: "Zero-shot",
      operation: "classification.zeroShot",
      input: { text: "rust text retrieval", labels: ["code", "sports", "music"], model: { fallbackPolicy: "lexical_fallback" } },
      iterations: 80,
      warmupIterations: 5,
      outputCountPath: ["predictions"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
