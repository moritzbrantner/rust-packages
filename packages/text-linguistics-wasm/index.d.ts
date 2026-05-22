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
export function postprocessEntities(text: string, predictions: BertNerPrediction[]): unknown;
export function postprocessClassification(text: string, predictions: BertNerPrediction[]): unknown;
export function postprocessSentiment(text: string, predictions: BertNerPrediction[]): unknown;
export function postprocessEmbeddings(embeddings: number[][]): unknown;
export function postprocessZeroShot(text: string, labels: string[], predictions: BertNerPrediction[]): unknown;
export function summarizeLexical(text: string, maxSentences: number): unknown;
export function summarizeEmbeddingExtractiveFromImportedEmbeddings(
  text: string,
  maxSentences: number,
  sentenceEmbeddings: number[][],
): unknown;
export function rerankFromImportedScores(query: string, documents: string[], scores: number[]): unknown;
export function initSync(module: WebAssembly.Module | BufferSource): unknown;
export default function init(moduleOrPath?: WebAssembly.Module | BufferSource | string | URL | Request): Promise<unknown>;
