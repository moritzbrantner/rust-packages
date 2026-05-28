import type { PackageAppPreset, SurfaceOperation } from "./types";
export declare function OperationWorkbench({ error, input, operation, operations, presets, running, selectedOperation, onInputChange, onPreset, onRun, onSelectOperation, }: {
    error: string | null;
    input: string;
    operation: SurfaceOperation | null;
    operations: SurfaceOperation[];
    presets?: PackageAppPreset[];
    running: boolean;
    selectedOperation: string;
    onInputChange: (input: string) => void;
    onPreset: (preset: PackageAppPreset) => void;
    onRun: () => void;
    onSelectOperation: (operation: string) => void;
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=OperationWorkbench.d.ts.map