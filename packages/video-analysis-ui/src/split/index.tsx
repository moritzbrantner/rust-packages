import type { TimelineScene } from "../types";
import { DataTable, Panel } from "../shared/primitives";
import {
  formatSeconds,
  sceneEndFrame,
  sceneEndSeconds,
  sceneIndex,
  sceneStartFrame,
  sceneStartSeconds,
} from "../shared/utils";

export function SplitPlanTable({
  scenes,
  videoName = "video",
  template = "$VIDEO_NAME-Scene-$SCENE_NUMBER.mp4",
}: {
  scenes: TimelineScene[];
  videoName?: string;
  template?: string;
}) {
  return (
    <Panel title="Split Plan" description={`${scenes.length} output clips`}>
      <DataTable
        rows={scenes}
        empty="No scene clips"
        getRowKey={(scene, index) => `${sceneStartFrame(scene)}-${sceneEndFrame(scene)}-${index}`}
        columns={[
          {
            key: "output",
            header: "Output",
            className: "font-medium text-zinc-950",
            cell: (scene, index) =>
              formatOutputName(template, videoName, sceneIndex(scene, index + 1), scene),
          },
          {
            key: "start",
            header: "Start",
            className: "tabular-nums text-zinc-700",
            cell: (scene) => formatSeconds(sceneStartSeconds(scene)),
          },
          {
            key: "duration",
            header: "Duration",
            className: "tabular-nums text-zinc-700",
            cell: (scene) => formatSeconds(sceneEndSeconds(scene) - sceneStartSeconds(scene)),
          },
          {
            key: "frames",
            header: "Frames",
            className: "tabular-nums text-zinc-700",
            cell: (scene) => `${sceneStartFrame(scene)}-${sceneEndFrame(scene)}`,
          },
        ]}
      />
    </Panel>
  );
}

function formatOutputName(
  template: string,
  videoName: string,
  sceneNumber: number,
  scene: TimelineScene,
): string {
  const digits = Math.max(3, String(sceneNumber).length);
  return template
    .split("$VIDEO_NAME")
    .join(videoName)
    .split("$SCENE_NUMBER")
    .join(String(sceneNumber).padStart(digits, "0"))
    .split("$START_FRAME")
    .join(String(sceneStartFrame(scene)))
    .split("$END_FRAME")
    .join(String(sceneEndFrame(scene)));
}
