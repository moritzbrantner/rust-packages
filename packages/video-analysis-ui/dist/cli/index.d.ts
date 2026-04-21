export interface CliRun {
    command: string;
    args?: string[];
    status?: "pending" | "running" | "succeeded" | "failed" | string;
    exit_code?: number | null;
    output_files?: string[];
    message?: string | null;
}
export declare function CliRunPanel({ run }: {
    run: CliRun;
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=index.d.ts.map