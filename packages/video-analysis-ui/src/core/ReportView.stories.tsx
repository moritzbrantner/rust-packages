import type { Meta, StoryObj } from "@storybook/react-vite";

import type { AnalysisEvent, AnalysisObservation, SceneReport, VideoReport } from "../types";
import { EventList, ObservationList, ScenePanel, VideoSummaryCards } from "./index";

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
    end_frame: 300,
    start_seconds: 4,
    end_seconds: 10,
    observations: [],
  },
  {
    index: 3,
    start_frame: 300,
    end_frame: 450,
    start_seconds: 10,
    end_seconds: 15,
    observations: [],
  },
];

const observations: AnalysisObservation[] = [
  {
    analyzer: "object-detector",
    kind: "Object",
    label: "person",
    score: 0.91,
    timestamp_seconds: 1.5,
    frame_index: 45,
    scene_index: 1,
  },
  {
    analyzer: "ocr",
    kind: "Text",
    text: "Chapter 1",
    score: 0.87,
    timestamp_seconds: 6.2,
    frame_index: 186,
    scene_index: 2,
  },
];

const events: AnalysisEvent[] = [
  {
    analyzer: "scene-detector",
    label: "Hard cut",
    score: 0.95,
    timestamp_seconds: 4,
  },
  {
    analyzer: "motion",
    label: "Camera pan",
    score: 0.72,
    timestamp_seconds: 11.4,
  },
];

const video: VideoReport = {
  width: 1920,
  height: 1080,
  frame_rate: "30000/1001",
  duration_seconds: 15,
  frames_processed: 450,
  scenes,
  observations,
};

function ReportPreview() {
  return (
    <main className="min-h-screen bg-zinc-50 p-5 text-zinc-950">
      <div className="mx-auto grid max-w-screen-xl gap-5">
        <VideoSummaryCards video={video} />
        <ScenePanel scenes={scenes} />
        <div className="grid gap-5 lg:grid-cols-2">
          <ObservationList observations={observations} />
          <EventList events={events} />
        </div>
      </div>
    </main>
  );
}

const meta = {
  title: "Core/ReportView",
  component: ReportPreview,
} satisfies Meta<typeof ReportPreview>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SampleReport: Story = {};
