import { PackageSurfaceWorkbench, type PackageAppConfig } from "@video-analysis/ui/package-surface";
import * as wasm from "@mb-rust/text-question-answering-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-question-answering",
  title: "Text Question Answering",
  description: "Question answering APIs, imported span postprocessing, and deterministic fallback handling.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-question-answering",
    standaloneRoute: "",
  },
  defaultOperation: "qa.answer",
  featuredOperations: ["qa.answer", "qa.models", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run extractive question-answering postprocessing.",
      operations: ["qa.answer"],
    },
    {
      id: "debug",
      label: "Debug",
      description: "Inspect package metadata and model catalog helpers.",
      operations: ["qa.models", "describe"],
    },
  ],
  presets: [
    {
      id: "answer",
      label: "Answer",
      operation: "qa.answer",
      description: "Postprocess imported span predictions for a short context.",
      input: {
        question: "What is reliable?",
        context: "Rust is reliable.",
        importedPredictions: [{ text: "Rust", score: 0.9 }],
      },
    },
  ],
  benchmarkScenarios: [
    {
      id: "imported-span",
      label: "Imported Span",
      operation: "qa.answer",
      input: {
        question: "What is reliable?",
        context: "Rust is reliable for deterministic text package benchmarks.",
        importedPredictions: [{ text: "Rust", score: 0.9, attributes: { byte_start: "0", byte_end: "4" } }],
      },
      iterations: 100,
      warmupIterations: 5,
      outputCountPath: ["answers"],
    },
  ],
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
