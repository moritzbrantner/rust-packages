export interface SurfaceRequest {
  operation: string;
  input: unknown;
}

export interface SurfaceOperation {
  id: string;
  name: string;
  description?: string;
  inputSchema: unknown;
  outputSchema: unknown;
  exampleRequest: unknown;
  wasmSupported: boolean;
  serverSupported: boolean;
}

export interface PackageSurface {
  library: string;
  version: string;
  operations: SurfaceOperation[];
  capabilities: unknown;
}

export interface SurfaceResponse {
  operation: string;
  value: unknown;
  diagnostics: unknown[];
  artifacts: unknown[];
}

export interface FinanceDataSeriesIndex {
  getBounds(): unknown;
  getBars(query: { startMs: number; endMs: number }): unknown;
  getDownsampledBars(query: { startMs: number; endMs: number; targetCount: number }): unknown;
  getReturns(query: { adjusted?: boolean; method?: "simple" | "log" }): unknown;
  getRiskSummary(query: unknown): unknown;
}

export function init(): Promise<unknown>;
export function packageSurface(): Promise<PackageSurface>;
export function runOperation(request: SurfaceRequest): Promise<SurfaceResponse>;
export function createSeriesIndex(series: unknown): Promise<FinanceDataSeriesIndex>;
