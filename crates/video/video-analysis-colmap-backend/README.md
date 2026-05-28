# video-analysis-colmap-backend

COLMAP compatibility backend, command planning, and parity reporting for
`video-analysis`.

The current implementation supports COLMAP text models as a hermetic baseline
and exposes explicit status for binary model support. It also exposes a native
server-only `video.colmap.reconstructVideo` operation that extracts frames from
a local sample video, runs COLMAP sparse reconstruction, exports the sparse model
to text, and returns browser-friendly scene data.

Default surface operations are deterministic unless marked server-only:

- `video.colmap.commandPlan` returns the ffmpeg and COLMAP stages that would run
  for a video reconstruction request without executing them.
- `video.colmap.imageList` summarizes image ordering and camera grouping for
  COLMAP ingestion.
- `video.colmap.sparseSummary` reports camera, registered image, sparse point,
  and track-length counts from inline sparse model metadata.
- `video.colmap.reconstructVideo` is available only through the native server or
  overview server dispatch path because it shells out to `ffmpeg` and `colmap`.

The optional COLMAP test video is not checked in. Create the ignored local file
with:

```bash
bun run setup:colmap-video
```

That command writes
`prototypes/web/video-analysis-web/public/samples/video/test-video.mp4`, which is
also exposed by the shared package workbench video sample catalog for other video
package UIs.

## Feature flags

- No optional feature flags today.

## Related crates

- `video-analysis-radiance-io`
- `video-analysis-sfm`
