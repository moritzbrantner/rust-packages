import type { Cut, DetectionResult } from "../types";
import { DataTable, EmptyState, Panel, StatCard } from "../shared/primitives";
import { formatNumber, formatScore, formatSeconds, timestampSeconds } from "../shared/utils";

export function DetectionSummary({
  result,
  detector,
}: {
  result: DetectionResult;
  detector?: string;
}) {
  return (
    <Panel title="Detection" description={detector}>
      <div className="grid gap-3 sm:grid-cols-3">
        <StatCard label="Scenes" value={formatNumber(result.scenes.length)} tone="sky" />
        <StatCard label="Cuts" value={formatNumber(result.cuts?.length ?? 0)} tone="emerald" />
        <StatCard label="Frames" value={formatNumber(result.frames_processed)} tone="amber" />
      </div>
      {result.cuts && result.cuts.length > 0 && (
        <div className="mt-4">
          <CutTable cuts={result.cuts} />
        </div>
      )}
    </Panel>
  );
}

export function CutTable({ cuts }: { cuts: Cut[] }) {
  if (cuts.length === 0) {
    return <EmptyState>No cuts</EmptyState>;
  }

  return (
    <DataTable
      rows={cuts}
      getRowKey={(cut, index) => `${cut.position.frame_index}-${cut.detector}-${index}`}
      columns={[
        {
          key: "frame",
          header: "Frame",
          className: "tabular-nums text-zinc-700",
          cell: (cut) => formatNumber(cut.position.frame_index),
        },
        {
          key: "time",
          header: "Time",
          className: "tabular-nums text-zinc-700",
          cell: (cut) => formatSeconds(timestampSeconds(cut.position.timestamp)),
        },
        {
          key: "detector",
          header: "Detector",
          className: "font-medium text-zinc-950",
          cell: (cut) => cut.detector,
        },
        {
          key: "score",
          header: "Score",
          className: "text-zinc-700",
          cell: (cut) => formatScore(cut.score),
        },
      ]}
    />
  );
}
