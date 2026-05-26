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

export interface NumericSeriesKernelPoint {
  sourceIndex: number;
  x: number;
  y: number;
  metrics?: Record<string, number>;
}

export interface NumericSeriesKernelQuery {
  xDomain: [number, number];
  targetBinCount: number;
  includeEmptyBins?: boolean;
}

export interface NumericSeriesBounds {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
}

export interface NumericSeriesBin {
  averageY: number | null;
  firstPointIndex: number | null;
  index: number;
  lastPointIndex: number | null;
  maxY: number | null;
  metrics: Record<string, number>;
  minY: number | null;
  pointCount: number;
  sumY: number;
  x0: number;
  x1: number;
}

export interface NumericSeriesKernelResult {
  bins: NumericSeriesBin[];
}

export class NumericSeriesIndex {
  constructor(points: NumericSeriesKernelPoint[]);
  getSeriesBounds(): NumericSeriesBounds | null;
  getBinnedSeries(query: NumericSeriesKernelQuery): NumericSeriesKernelResult;
}

export function init(): Promise<unknown>;
export function packageSurface(): PackageSurface;
export function runOperation(request: SurfaceRequest): SurfaceResponse;
