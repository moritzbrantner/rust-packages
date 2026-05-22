export interface BertNerPrediction {
  kind?: string | null;
  label?: string | null;
  text?: string | null;
  score?: number | null;
  attributes?: Record<string, string>;
}

export interface TextLinguisticsWasmOptions {
  profile?: "fast" | "balanced" | "rich";
  entityRecognition?: "heuristic" | "local-model" | "bert-base-ner" | "bert-ner";
  bertNerPredictions?: BertNerPrediction[];
}

export function analyzeTextLinguistics(text: string, options?: TextLinguisticsWasmOptions): unknown;
export function initSync(module: WebAssembly.Module | BufferSource): unknown;
export default function init(moduleOrPath?: WebAssembly.Module | BufferSource | string | URL | Request): Promise<unknown>;
