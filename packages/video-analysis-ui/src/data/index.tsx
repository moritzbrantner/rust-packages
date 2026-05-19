import type { DataBucketReport, StreamBucketReport } from "../types";
import { DataTable, EmptyState, Panel, StatCard } from "../shared/primitives";
import { formatBytes, formatNumber, ratioPercent } from "../shared/utils";

export function DataBucketOverview({ buckets }: { buckets: DataBucketReport[] }) {
  const totalRecords = buckets.reduce((sum, bucket) => sum + bucket.records, 0);
  const totalBytes = buckets.reduce((sum, bucket) => sum + bucket.estimated_bytes, 0);
  const maxRecords = Math.max(...buckets.map((bucket) => bucket.records), 0);

  return (
    <Panel title="Data Buckets" description={`${formatNumber(buckets.length)} summaries`}>
      {buckets.length === 0 ? (
        <EmptyState>No data buckets</EmptyState>
      ) : (
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-3">
            <StatCard label="Buckets" value={formatNumber(buckets.length)} tone="sky" />
            <StatCard label="Records" value={formatNumber(totalRecords)} tone="emerald" />
            <StatCard label="Estimated bytes" value={formatBytes(totalBytes)} tone="amber" />
          </div>
          <div className="space-y-2">
            {buckets.map((bucket) => (
              <div key={bucket.bucket_index} className="rounded-lg border border-zinc-200 p-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="font-medium text-zinc-950">Bucket {bucket.bucket_index}</div>
                  <div className="text-sm text-zinc-500">
                    {formatNumber(bucket.records)} records, {formatBytes(bucket.estimated_bytes)}
                  </div>
                </div>
                <div className="mt-2 h-2 overflow-hidden rounded-full bg-zinc-200">
                  <div
                    className="h-full rounded-full bg-sky-500"
                    style={{ width: `${ratioPercent(bucket.records, maxRecords)}%` }}
                  />
                </div>
                <div className="mt-3">
                  <StreamSummaryTable streams={bucket.streams} compact />
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </Panel>
  );
}

export function StreamSummaryTable({
  streams,
  compact = false,
}: {
  streams: StreamBucketReport[];
  compact?: boolean;
}) {
  if (streams.length === 0) {
    return <EmptyState>No streams</EmptyState>;
  }

  return (
    <DataTable
      rows={streams}
      getRowKey={(stream) => stream.stream_id}
      columns={[
        {
          key: "stream",
          header: "Stream",
          className: "font-medium text-zinc-950",
          cell: (stream) => stream.stream_id,
        },
        {
          key: "records",
          header: "Records",
          className: "tabular-nums text-zinc-700",
          cell: (stream) => formatNumber(stream.records),
        },
        {
          key: "bytes",
          header: "Bytes",
          className: "tabular-nums text-zinc-700",
          cell: (stream) => formatBytes(stream.estimated_bytes),
        },
        ...(!compact
          ? [
              {
                key: "payloads",
                header: "Payloads",
                className: "text-zinc-700",
                cell: (stream: StreamBucketReport) => payloadSummary(stream.payload_counts),
              },
            ]
          : []),
        {
          key: "video",
          header: "Video",
          className: "tabular-nums text-zinc-700",
          cell: (stream) => formatNumber(stream.video_frames),
        },
        {
          key: "audio",
          header: "Audio",
          className: "tabular-nums text-zinc-700",
          cell: (stream) => formatNumber(stream.audio_frames),
        },
        {
          key: "text",
          header: "Text",
          className: "tabular-nums text-zinc-700",
          cell: (stream) => formatNumber(stream.text_segments),
        },
      ]}
    />
  );
}

function payloadSummary(payloads: Record<string, number>): string {
  const entries = Object.entries(payloads);
  if (entries.length === 0) {
    return "n/a";
  }
  return entries.map(([key, value]) => `${key}: ${formatNumber(value)}`).join(", ");
}
