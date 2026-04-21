export interface MediaMetadata {
    input: string;
    mode?: "Recorded" | "Live" | string;
    width?: number | null;
    height?: number | null;
    frame_rate?: string | null;
    duration_seconds?: number | null;
    sample_rate?: number | null;
    channels?: number | null;
}
export declare function MediaMetadataPanel({ metadata }: {
    metadata: MediaMetadata;
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=index.d.ts.map