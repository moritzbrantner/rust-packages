import type { YoutubeVideoReport } from "./types";

export const youtubeVideoReportContractFixture = {
  use_case: "youtube-video",
  source: {
    url: null,
    local_video: "video.mp4",
  },
  assets: {
    work_dir: "use-case-output/youtube-video",
    report_path: "use-case-output/youtube-video/analysis.json",
    audio_wav: null,
  },
  capabilities: {
    completed: ["scene_detection"],
    skipped: ["transcription: disabled by --skip-transcription"],
  },
  video: {
    width: 1920,
    height: 1080,
    frame_rate: "30/1",
    duration_seconds: 10,
    frames_processed: 300,
    scenes: [
      {
        index: 0,
        start_frame: 0,
        end_frame: 299,
        start_seconds: 0,
        end_seconds: 9.966667,
        observations: [],
      },
    ],
    observations: [],
  },
  transcription: {
    status: "skipped",
    text: null,
    segments: [],
    message: "disabled by --skip-transcription",
  },
  audio: {
    status: "skipped",
    frames_processed: 0,
    events: [],
    message: "not available",
  },
  text: {
    status: "skipped",
    segments_processed: 0,
    events: [],
    message: "no transcript available",
  },
  data_buckets: [
    {
      bucket_index: 0,
      records: 1,
      estimated_bytes: 1024,
      streams: [
        {
          stream_id: "video",
          records: 1,
          estimated_bytes: 1024,
          payload_counts: { video: 1 },
          video_frames: 1,
          audio_frames: 0,
          text_segments: 0,
        },
      ],
    },
  ],
} satisfies YoutubeVideoReport;
