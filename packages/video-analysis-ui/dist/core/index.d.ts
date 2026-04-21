import type { AnalysisEvent, AnalysisObservation, SceneReport, TimelineScene, VideoReport } from "../types";
export declare function VideoSummaryCards({ video }: {
    video: VideoReport;
}): import("react/jsx-runtime").JSX.Element;
export declare function SceneTimeline({ scenes, durationSeconds, activeSceneIndex, onSelectScene, className, }: {
    scenes: TimelineScene[];
    durationSeconds?: number | null;
    activeSceneIndex?: number | null;
    onSelectScene?: (scene: TimelineScene, index: number) => void;
    className?: string;
}): import("react/jsx-runtime").JSX.Element;
export declare function SceneTable({ scenes, onSelectScene, }: {
    scenes: TimelineScene[];
    onSelectScene?: (scene: TimelineScene, index: number) => void;
}): import("react/jsx-runtime").JSX.Element;
export declare function ObservationList({ observations, title, }: {
    observations: AnalysisObservation[];
    title?: string;
}): import("react/jsx-runtime").JSX.Element;
export declare function EventList({ events, title, empty, }: {
    events: AnalysisEvent[];
    title?: string;
    empty?: string;
}): import("react/jsx-runtime").JSX.Element;
export declare function ScenePanel({ scenes }: {
    scenes: SceneReport[] | TimelineScene[];
}): import("react/jsx-runtime").JSX.Element;
//# sourceMappingURL=index.d.ts.map