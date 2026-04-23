import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { AnalysisObservation, SceneReport, VideoReport } from "../types";
import { ObservationList, SceneTimeline, VideoSummaryCards } from "./index";

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

describe("core views", () => {
  it("renders summary cards", () => {
    render(<VideoSummaryCards video={video} />);

    expect(screen.getByText("1920x1080")).toBeDefined();
    expect(screen.getByText("240")).toBeDefined();
    expect(screen.getByText("2")).toBeDefined();
  });

  it("renders scene and observation content", () => {
    render(
      <div>
        <SceneTimeline scenes={scenes} durationSeconds={8} />
        <ObservationList observations={observations} />
      </div>,
    );

    expect(screen.getByText("person")).toBeDefined();
    expect(screen.getByText("object-command")).toBeDefined();
    expect(screen.getByTitle("Scene 1 00:00:00.000-00:00:04.000")).toBeDefined();
    expect(screen.getByTitle("Scene 2 00:00:04.000-00:00:08.000")).toBeDefined();
  });
});
