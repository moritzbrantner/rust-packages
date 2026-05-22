export interface PackageMetadata {
  package: string;
  surface: string;
  library: string;
  libraryImport: string;
  cliPackage: string;
  appPackage: string;
  endpoints: string[];
}

export interface HealthPayload {
  ok: boolean;
  package: string;
  library: string;
}

export type AudioTask =
  | "classify"
  | "events"
  | "embed"
  | "transcribe"
  | "diarize"
  | "separate"
  | "generate";

export type AudioFallbackPolicy = "error" | "fast_fallback" | "heuristic_fallback";

export interface AudioModelMetadata {
  id: string;
  modelId: string;
  task: string;
  runtime: string;
  supported: boolean;
  fallback?: string | null;
  note?: string | null;
}

export interface AudioFeatureSummary {
  durationSeconds?: number;
  sampleRate?: number;
  rms?: number;
  peak?: number;
  zeroCrossingRate?: number;
  dominantFrequencyHz?: number;
  spectralCentroidHz?: number;
}

export interface AudioModelSelection {
  modelId?: string;
  fallbackPolicy?: AudioFallbackPolicy;
}

export interface AudioClassPrediction {
  label: string;
  score: number;
}

export interface AudioClassificationPayload {
  accepted: true;
  operation: "classify";
  modelId: string;
  runtime: string;
  predictions: AudioClassPrediction[];
}

export interface AudioEventPayload {
  accepted: true;
  operation: "events";
  modelId: string;
  runtime: string;
  events: Array<{ label: string; score: number; startSeconds: number; endSeconds: number }>;
}

export interface AudioEmbeddingPayload {
  accepted: true;
  operation: "embed";
  modelId: string;
  runtime: string;
  dimensions: number;
  embeddings: number[][];
}

export interface SpeechRecognitionPayload {
  accepted: true;
  operation: "transcribe";
  modelId: string;
  runtime: string;
  text: string;
  segments: Array<{ index: number; startSeconds?: number; endSeconds?: number; text: string }>;
}

export interface SpeakerDiarizationPayload {
  accepted: true;
  operation: "diarize";
  modelId: string;
  runtime: string;
  segments: Array<{ speaker: string; startSeconds: number; endSeconds: number; score?: number }>;
}

export interface SourceSeparationPayload {
  accepted: true;
  operation: "separate";
  modelId: string;
  runtime: string;
  stems: Array<{ stem: string; uri?: string | null; score?: number | null }>;
}

export interface AudioGenerationPayload {
  accepted: true;
  operation: "generate";
  modelId: string;
  runtime: string;
  prompt: string;
  durationSeconds: number;
  plan: string;
}

const configuredServerUrl = import.meta.env.VITE_SERVER_URL as string | undefined;

export const serverBaseUrl = configuredServerUrl ?? "http://127.0.0.1:3000";
export const wrappedLibrary = "audio-analysis-recognition";

export async function fetchHealth(): Promise<HealthPayload> {
  return fetchJson<HealthPayload>("/health");
}

export async function fetchPackageMetadata(): Promise<PackageMetadata> {
  return fetchJson<PackageMetadata>("/api/package");
}

export async function listAudioModels(task?: AudioTask): Promise<AudioModelMetadata[]> {
  return fetchJson<AudioModelMetadata[]>(task ? `/api/models/${task}` : "/api/models");
}

export async function runAudioTask(task: AudioTask, body: unknown): Promise<unknown> {
  const response = await fetch(`${serverBaseUrl}/api/${task}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Server returned ${response.status}`);
  }
  return response.json() as Promise<unknown>;
}

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(`${serverBaseUrl}${path}`);
  if (!response.ok) {
    throw new Error(`Server returned ${response.status}`);
  }
  return response.json() as Promise<T>;
}

