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

export type NumericValueMode = "average" | "count" | "max" | "min" | "sum";
export type NumericValueAccessor = "x" | "y";

export interface NumericSeriesQuery extends NumericSeriesKernelQuery {
  valueMode?: NumericValueMode;
}

export interface NumericHistogramQuery {
  bucketCount: number;
  includeEmptyBuckets?: boolean;
  valueAccessor?: NumericValueAccessor;
  valueDomain?: [number, number];
  xDomain?: [number, number];
}

export interface NumericHeatmapQuery {
  includeEmptyCells?: boolean;
  xBinCount: number;
  xDomain: [number, number];
  yBinCount: number;
  yDomain?: [number, number];
  valueAccessor?: NumericValueAccessor;
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

export interface NumericSeriesSample extends NumericSeriesBin {
  x: number;
  y: number | null;
}

export interface NumericSeriesResult {
  bins: NumericSeriesBin[];
  samples: NumericSeriesSample[];
  summary: {
    binCount: number;
    metrics: Record<string, number>;
    pointCount: number;
    sampleCount: number;
    valueMode: NumericValueMode;
    xDomain: [number, number];
  };
}

export interface NumericHistogramBucket {
  averageValue: number | null;
  firstPointIndex: number | null;
  index: number;
  lastPointIndex: number | null;
  maxValue: number | null;
  metrics: Record<string, number>;
  minValue: number | null;
  pointCount: number;
  sumValue: number;
  value: number;
  value0: number;
  value1: number;
}

export interface NumericHistogramResult {
  buckets: NumericHistogramBucket[];
  summary: {
    bucketCount: number;
    metrics: Record<string, number>;
    pointCount: number;
    valueDomain: [number, number];
    xDomain: [number, number] | null;
  };
}

export interface NumericHeatmapCell {
  averageValue: number | null;
  firstPointIndex: number | null;
  index: number;
  lastPointIndex: number | null;
  metrics: Record<string, number>;
  pointCount: number;
  sumValue: number;
  value: number;
  x: number;
  x0: number;
  x1: number;
  xIndex: number;
  y: number;
  y0: number;
  y1: number;
  yIndex: number;
}

export interface NumericHeatmapResult {
  cells: NumericHeatmapCell[];
  summary: {
    maxCellCount: number;
    metrics: Record<string, number>;
    pointCount: number;
    xBinCount: number;
    xDomain: [number, number];
    yBinCount: number;
    yDomain: [number, number];
  };
}

export class NumericSeriesIndex {
  constructor(points: NumericSeriesKernelPoint[]);
  getSeriesBounds(): NumericSeriesBounds | null;
  getBinnedSeries(query: NumericSeriesKernelQuery): NumericSeriesKernelResult;
  getChartSeries(query: NumericSeriesQuery): NumericSeriesResult;
  getHistogram(query: NumericHistogramQuery): NumericHistogramResult;
  getHeatmap(query: NumericHeatmapQuery): NumericHeatmapResult;
}

export function init(): Promise<unknown>;
export function packageSurface(): PackageSurface;
export function runOperation(request: SurfaceRequest): SurfaceResponse;
