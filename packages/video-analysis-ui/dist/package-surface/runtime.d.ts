import type { HealthPayload, ModelCatalogEntry, PackageAppConfig, PackageSurface, RuntimeMode, SurfaceResponse } from "./types";
export declare function configuredServerBaseUrl(config: PackageAppConfig): string;
export declare function initializeWasmSurface(config: PackageAppConfig): Promise<PackageSurface>;
export declare function fetchHealth(config: PackageAppConfig, mode: RuntimeMode): Promise<HealthPayload>;
export declare function fetchServerSurface(config: PackageAppConfig, mode: RuntimeMode): Promise<PackageSurface>;
export declare function fetchModelCatalog(config: PackageAppConfig, mode: RuntimeMode): Promise<ModelCatalogEntry[]>;
export declare function runOperation(config: PackageAppConfig, mode: RuntimeMode, operation: string, input: unknown): Promise<SurfaceResponse>;
export declare function normalizeModelCatalog(input: unknown[]): ModelCatalogEntry[];
//# sourceMappingURL=runtime.d.ts.map