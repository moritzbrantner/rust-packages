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

export type GeoVizBounds = [west: number, south: number, east: number, north: number];
export type GeoVizMetricRecord = Record<string, number>;

export interface GeoVizPoint<TProperties = unknown> {
  id?: string;
  label?: string;
  longitude: number;
  latitude: number;
  metrics?: GeoVizMetricRecord;
  properties?: TProperties;
}

export interface GeoVizIndexedPoint<TProperties = unknown> {
  id: string;
  sourceIndex: number;
  label: string;
  longitude: number;
  latitude: number;
  metrics: GeoVizMetricRecord;
  properties: TProperties;
}

export interface GeoVizViewportQuery {
  bounds: GeoVizBounds;
  zoom: number;
}

export interface GeoVizAggregationOptions {
  radius?: number;
  extent?: number;
  minZoom?: number;
  maxZoom?: number;
}

export type GeoVizAggregationFeature<TProperties = unknown> =
  | {
      kind: "point";
      coordinates: [longitude: number, latitude: number];
      metrics: GeoVizMetricRecord;
      point: GeoVizIndexedPoint<TProperties>;
    }
  | {
      kind: "cluster";
      clusterId: number;
      coordinates: [longitude: number, latitude: number];
      expansionZoom: number;
      metrics: GeoVizMetricRecord;
      pointCount: number;
      pointCountAbbreviated: string;
    };

export interface GeoVizAggregationSummary {
  bounds: GeoVizBounds;
  zoom: number;
  metrics: GeoVizMetricRecord;
  visiblePointCount: number;
  visibleClusterCount: number;
  visibleUnclusteredCount: number;
}

export interface GeoVizAggregation<TProperties = unknown> {
  features: Array<GeoVizAggregationFeature<TProperties>>;
  summary: GeoVizAggregationSummary;
}

export class GeoPointIndex<TProperties = unknown> {
  constructor(points: Array<GeoVizPoint<TProperties>>, options?: GeoVizAggregationOptions);
  getBounds(): GeoVizBounds | null;
  getPointById(pointId: string): GeoVizIndexedPoint<TProperties> | null;
  getViewportAggregation(query: GeoVizViewportQuery): GeoVizAggregation<TProperties>;
  getClusterExpansionZoom(clusterId: number): number;
  getClusterLeaves(
    clusterId: number,
    limit?: number,
    offset?: number,
  ): Array<GeoVizIndexedPoint<TProperties>>;
  free(): void;
}

export function init(): Promise<unknown>;
export function packageSurface(): PackageSurface;
export function runOperation(request: SurfaceRequest): SurfaceResponse;
