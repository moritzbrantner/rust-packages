import type { ReactNode } from "react";
export type RuntimeMode = "client-wasm" | "overview-server" | "standalone-server";
export type PackageDomain = "text" | "audio" | "image" | "video" | "vector" | "three-d" | "comfyui" | "data" | "math" | "runtime" | "jobs" | "support" | "animation";
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
export interface SurfaceRequest {
    operation: string;
    input: unknown;
}
export interface SurfaceResponse {
    operation: string;
    value: unknown;
    diagnostics: unknown[];
    artifacts: unknown[];
}
export interface HealthPayload {
    ok: boolean;
    package: string;
    library: string;
    domain?: string;
    linked?: boolean;
    requiredFeature?: string | null;
}
export type ModelRuntime = "deterministic" | "heuristic" | "candle" | "onnx" | "whisper_cpp" | "opencv" | "comfyui" | "external";
export interface ModelCatalogEntry {
    id: string;
    label: string;
    task: string;
    runtime: ModelRuntime;
    supported: boolean;
    fallback?: string;
    requiredFeature?: string;
    source?: string;
    note?: string;
}
export interface PackageAppPreset {
    id: string;
    label: string;
    operation: string;
    input: unknown;
    description?: string;
}
export interface ResultTabDefinition {
    id: string;
    label: string;
    select: (response: SurfaceResponse) => unknown;
}
export interface FileInputDefinition {
    id: string;
    label: string;
    accept?: string;
    targetPath: string[];
    encoding?: "data-url" | "text";
}
export interface PackageAppConfig {
    library: string;
    title: string;
    description: string;
    domain: PackageDomain;
    wasm?: {
        init: () => Promise<unknown>;
        packageSurface: () => Promise<PackageSurface> | PackageSurface;
        runOperation: (request: SurfaceRequest) => Promise<SurfaceResponse> | SurfaceResponse;
    };
    server?: {
        baseUrlEnv?: string;
        scopedRoute: `/api/rust/packages/${string}`;
        standaloneRoute?: "";
    };
    featuredOperations?: string[];
    defaultOperation?: string;
    presets?: PackageAppPreset[];
    resultTabs?: ResultTabDefinition[];
    fileInputs?: FileInputDefinition[];
    children?: ReactNode;
}
//# sourceMappingURL=types.d.ts.map