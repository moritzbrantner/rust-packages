import { createTextResultTabs, PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/text-question-answering-wasm";

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
      id: "imported-span-answer",
      label: "Use imported span answer",
      operation: "qa.answer",
      description: "Postprocess imported span predictions for a short context.",
      input: {
        question: "Who presented the tokenizer roadmap?",
        context: "Alice presented the tokenizer roadmap in Berlin while Bob reviewed transcript retrieval evidence.",
        importedPredictions: [
          { text: "Alice", score: 0.94, attributes: { byte_start: "0", byte_end: "5" } },
          { text: "Bob", score: 0.21, attributes: { byte_start: "55", byte_end: "58" } },
        ],
        topK: 2,
      },
    },
    {
      id: "lexical-style-imported-answer",
      label: "Review lexical-style answer",
      operation: "qa.answer",
      description: "Demonstrate the fallback-style path by supplying deterministic imported spans.",
      input: {
        question: "What helps editors find evidence?",
        context: "Caption retrieval helps editors find evidence in long transcripts.",
        importedPredictions: [
          { text: "Caption retrieval", score: 0.88, attributes: { byte_start: "0", byte_end: "17" } },
        ],
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
  resultTabs: createTextResultTabs({
    library: "text-question-answering",
    primaryOperations: {
      "qa.answer": {
        title: "Question answering",
        summaryFields: ["answerCount"],
        listFields: ["answers"],
        objectFields: ["model", "result"],
        explanation: () => "The current browser-safe workflow postprocesses supplied extractive span predictions and reports question, answer, score, span, and runtime metadata.",
      },
    },
  }),
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
