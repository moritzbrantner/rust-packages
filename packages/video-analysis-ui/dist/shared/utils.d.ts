import type { Scene, SceneReport, Timestamp, TimelineScene } from "../types";
export declare function cn(...classes: Array<string | false | null | undefined>): string;
export declare function timestampSeconds(timestamp?: Timestamp | null): number | null;
export declare function sceneStartSeconds(scene: TimelineScene): number;
export declare function sceneEndSeconds(scene: TimelineScene): number;
export declare function sceneStartFrame(scene: TimelineScene): number;
export declare function sceneEndFrame(scene: TimelineScene): number;
export declare function sceneIndex(scene: TimelineScene, fallback: number): number;
export declare function formatSeconds(value?: number | null): string;
export declare function formatNumber(value?: number | null): string;
export declare function formatBytes(value?: number | null): string;
export declare function formatScore(value?: number | null): string;
export declare function ratioPercent(value: number, max: number): number;
export declare function isSceneReport(scene: Scene | SceneReport): scene is SceneReport;
//# sourceMappingURL=utils.d.ts.map