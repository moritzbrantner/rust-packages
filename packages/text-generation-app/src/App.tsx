import { createTextResultTabs, PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/text-generation-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-generation",
  title: "Text Generation",
  description: "Deterministic Markov-chain prediction and text synthesis for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-generation",
    standaloneRoute: "",
  },
  defaultOperation: "generation.markovGenerate",
  featuredOperations: ["generation.markovGenerate", "generation.markovPredict", "generation.synthesizeTerms", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run deterministic Markov prediction, generation, and term synthesis workflows.",
      operations: ["generation.markovGenerate", "generation.markovPredict", "generation.synthesizeTerms"],
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
      id: "markov-predict",
      label: "Predict next transcript token",
      operation: "generation.markovPredict",
      description: "Train a transient Markov chain and predict next tokens.",
      input: {
        trainingTexts: [
          "rust text analysis supports transcript search",
          "rust text generation predicts transcript terms",
          "caption retrieval supports editorial review",
        ],
        context: ["rust", "text"],
        order: 2,
        topK: 5,
      },
    },
    {
      id: "markov-generate",
      label: "Generate Markov caption",
      operation: "generation.markovGenerate",
      description: "Train a transient Markov chain and generate text.",
      input: {
        trainingTexts: [
          "rust text analysis supports transcript search",
          "rust text generation supports caption synthesis",
          "transcript search supports editorial evidence review",
        ],
        seed: ["rust", "text"],
        order: 2,
        maxTokens: 12,
      },
    },
    {
      id: "term-synthesis",
      label: "Synthesize weighted terms",
      operation: "generation.synthesizeTerms",
      description: "Synthesize deterministic text from weighted terms.",
      input: {
        id: "caption-summary",
        terms: [
          { term: "rust", weight: 2.0 },
          { term: "transcript", weight: 1.8 },
          { term: "retrieval", weight: 1.4 },
          { term: "editorial", weight: 1.0 },
        ],
      },
    },
  ],
  benchmarkScenarios: [
    {
      id: "markov-predict",
      label: "Markov Predict",
      operation: "generation.markovPredict",
      input: { trainingTexts: ["rust text analysis supports transcript search", "rust crates analyze captions"], order: 2, prefix: ["rust"] },
      iterations: 100,
      warmupIterations: 5,
      outputCountPath: ["predictions"],
    },
    {
      id: "markov-generate",
      label: "Markov Generate",
      operation: "generation.markovGenerate",
      input: { trainingTexts: ["rust text analysis supports crates"], order: 2, maxTokens: 12 },
      iterations: 80,
      warmupIterations: 5,
      outputCountPath: ["tokens"],
    },
  ],
  resultTabs: createTextResultTabs({
    library: "text-generation",
    primaryOperations: {
      "generation.markovPredict": {
        title: "Markov prediction",
        summaryFields: ["order", "contexts", "predictionCount"],
        listFields: ["predictions"],
        objectFields: ["result"],
        explanation: () => "The operation trained a transient deterministic Markov chain from the sample text and predicted the next likely tokens for the supplied context.",
      },
      "generation.markovGenerate": {
        title: "Markov generation",
        summaryFields: ["order", "contexts", "generatedTokenCount"],
        listFields: ["generation.tokens"],
        objectFields: ["generation", "result"],
        explanation: () => "The generator used a deterministic Markov chain and seed tokens to produce a repeatable token sequence.",
      },
      "generation.synthesizeTerms": {
        title: "Term synthesis",
        summaryFields: ["id", "language", "assumptionCount"],
        listFields: ["trace.assumptions", "trace.notes", "terms"],
        objectFields: ["value", "trace"],
        explanation: () => "The synthesizer converted weighted terms into deterministic text and records assumptions in the trace rather than calling a model.",
      },
    },
  }),
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
