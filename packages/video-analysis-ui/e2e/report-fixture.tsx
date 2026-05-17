import { createRoot } from "react-dom/client";

import type { AnalysisObservation, SceneReport, VideoReport } from "../src/types";
import { ObservationList, SceneTimeline, VideoSummaryCards } from "../src/core";

const scenes: SceneReport[] = [
  {
    index: 1,
    start_frame: 0,
    end_frame: 120,
    start_seconds: 0,
    end_seconds: 4,
    observations: [],
  },
  {
    index: 2,
    start_frame: 120,
    end_frame: 240,
    start_seconds: 4,
    end_seconds: 8,
    observations: [],
  },
];

const observations: AnalysisObservation[] = [
  {
    analyzer: "object-command",
    kind: "Object",
    label: "person",
    score: 0.91,
    timestamp_seconds: 1.5,
    frame_index: 45,
  },
];

const video: VideoReport = {
  width: 1920,
  height: 1080,
  frame_rate: "30000/1001",
  duration_seconds: 8,
  frames_processed: 240,
  scenes,
  observations,
};

function Fixture() {
  return (
    <main>
      <h1>Video report fixture</h1>
      <VideoSummaryCards video={video} />
      <SceneTimeline scenes={scenes} durationSeconds={8} />
      <ObservationList observations={observations} />
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<Fixture />);
