import { createTextResultTabs, PackageSurfaceWorkbench, type PackageAppConfig } from "@moritzbrantner/video-analysis-ui/package-surface";
import * as wasm from "@moritzbrantner/text-transcripts-wasm";

const packageAppConfig: PackageAppConfig = {
  library: "text-transcripts",
  title: "Text Transcripts",
  description: "Transcript parsing and ASR command adapters for video-analysis.",
  domain: "text",
  wasm: {
    init: wasm.init,
    packageSurface: wasm.packageSurface,
    runOperation: wasm.runOperation,
  },
  server: {
    scopedRoute: "/api/rust/packages/text-transcripts",
    standaloneRoute: "",
  },
  defaultOperation: "transcripts.parse",
  featuredOperations: ["transcripts.parse", "transcripts.normalize", "transcripts.formatSrt", "describe"],
  operationGroups: [
    {
      id: "workflow",
      label: "Workflow",
      description: "Run transcript parsing, normalization, and SRT formatting workflows.",
      operations: ["transcripts.parse", "transcripts.normalize", "transcripts.formatSrt"],
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
      id: "parse-srt",
      label: "Parse SRT",
      operation: "transcripts.parse",
      description: "Parse SRT content into the transcript contract.",
      input: {
        format: "srt",
        content: "1\n00:00:01,000 --> 00:00:02,200\nAlice presented the tokenizer roadmap.\n\n2\n00:00:02,500 --> 00:00:04,000\nBob reviewed transcript retrieval evidence.\n",
      },
    },
    {
      id: "parse-webvtt",
      label: "Parse WebVTT",
      operation: "transcripts.parse",
      description: "Parse WebVTT captions into the transcript contract.",
      input: {
        format: "webVtt",
        content: "WEBVTT\n\n00:00:01.000 --> 00:00:02.200\nAlice presented the tokenizer roadmap.\n\n00:00:02.500 --> 00:00:04.000\nBob reviewed transcript retrieval evidence.\n",
      },
    },
    {
      id: "parse-plain",
      label: "Parse plain lines",
      operation: "transcripts.parse",
      description: "Parse plain transcript lines into segment records.",
      input: { format: "plain", content: "Alice presented the tokenizer roadmap.\nBob reviewed transcript retrieval evidence.\n" },
    },
    {
      id: "normalize",
      label: "Normalize transcript",
      operation: "transcripts.normalize",
      description: "Normalize segment text, final flags, and joined transcript text.",
      input: {
        segments: [
          { index: 0, startSeconds: 1.0, endSeconds: 2.2, text: "  Alice   presented the tokenizer roadmap. ", isFinal: true },
          { index: 1, startSeconds: 2.5, endSeconds: 4.0, text: " Bob reviewed transcript retrieval evidence. ", isFinal: true },
        ],
      },
    },
    {
      id: "format-srt",
      label: "Format SRT",
      operation: "transcripts.formatSrt",
      description: "Format a transcript contract as SRT.",
      input: {
        segments: [
          { index: 0, startSeconds: 1.0, endSeconds: 2.2, text: "Alice presented the tokenizer roadmap.", isFinal: true },
          { index: 1, startSeconds: 2.5, endSeconds: 4.0, text: "Bob reviewed transcript retrieval evidence.", isFinal: true },
        ],
      },
    },
  ],
  benchmarkScenarios: [
    {
      id: "parse-srt",
      label: "Parse SRT",
      operation: "transcripts.parse",
      input: { format: "srt", content: "1\n00:00:01,000 --> 00:00:02,000\nHello benchmark.\n\n2\n00:00:02,000 --> 00:00:03,000\nRust transcript parsing.\n" },
      iterations: 120,
      warmupIterations: 5,
      outputCountPath: ["segments"],
    },
    {
      id: "normalize",
      label: "Normalize",
      operation: "transcripts.normalize",
      input: { segments: [{ index: 0, startSeconds: 1.0, endSeconds: 2.0, text: "  Hello   benchmark.  ", isFinal: true }] },
      iterations: 120,
      warmupIterations: 5,
      outputCountPath: ["segments"],
    },
    {
      id: "format-srt",
      label: "Format SRT",
      operation: "transcripts.formatSrt",
      input: { segments: [{ index: 0, startSeconds: 1.0, endSeconds: 2.0, text: "Hello.", isFinal: true }] },
      iterations: 120,
      warmupIterations: 5,
      outputCountPath: ["content"],
    },
  ],
  resultTabs: createTextResultTabs({
    library: "text-transcripts",
    primaryOperations: {
      "transcripts.parse": {
        title: "Transcript parsing",
        summaryFields: ["segmentCount", "hasText"],
        listFields: ["segments", "words"],
        objectFields: ["metadata", "result"],
        explanation: () => "The parser converted SRT, WebVTT, Whisper JSON, or plain lines into the shared transcript contract and normalized joined text when available.",
      },
      "transcripts.normalize": {
        title: "Transcript normalization",
        summaryFields: ["segmentCount", "hasText"],
        listFields: ["segments", "words"],
        objectFields: ["metadata", "result"],
        explanation: () => "The normalization pass cleaned segment text, preserved timing, and rebuilt contract-level transcript text.",
      },
      "transcripts.formatSrt": {
        title: "SRT formatting",
        summaryFields: ["bytes"],
        objectFields: ["result"],
        explanation: () => "The formatter normalized the transcript contract and emitted SRT caption text with stable timing.",
      },
    },
  }),
};

export function App() {
  return <PackageSurfaceWorkbench config={packageAppConfig} />;
}
