# video-analysis-colmap-backend

COLMAP compatibility backend, command planning, and parity reporting for
`video-analysis`.

The current implementation supports COLMAP text models as a hermetic baseline
and exposes explicit status for binary model support. It also exposes a native
server-only `video.colmap.reconstructVideo` operation that extracts frames from
a local sample video, runs COLMAP sparse reconstruction, exports the sparse model
to text, and returns browser-friendly scene data.

The COLMAP app presents the native reconstruction as the primary workflow and
keeps the deterministic inspection helpers under a Debug tab. Default
surface operations are deterministic unless marked server-only:

- `video.colmap.commandPlan` previews the ffmpeg and COLMAP commands that would
  run for a video reconstruction request. It returns stage summaries, output
  paths, shell-readable command strings, and `executes: false`.
- `video.colmap.imageList` summarizes inline image JSON by frame order, detected
  frame range, and camera groups. It does not scan directories on disk.
- `video.colmap.sparseSummary` reports camera, registered image, sparse point,
  model status, and track-length statistics from inline sparse model JSON. It
  does not read COLMAP text or binary files from disk.
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

## Package surface

Workflow operations:

- `video.colmap.commandPlan`
- `video.colmap.reconstructVideo`

Debug operations:

- `describe`
- `video.colmap.imageList`
- `video.colmap.sparseSummary`

Runtime limits:

Debug operations are side-effect free. `video.colmap.reconstructVideo` is server-only and requires local `ffmpeg` and `colmap` binaries.

Invalid input returns a clear error through `run_surface_operation`; successful
responses include `operation`, `title`, `message`, `summary`, and `result` while
keeping existing top-level domain fields for compatibility.
