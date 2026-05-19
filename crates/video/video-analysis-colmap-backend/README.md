# video-analysis-colmap-backend

COLMAP compatibility backend and parity reporting for `video-analysis`.

The current implementation supports COLMAP text models as a hermetic baseline
and exposes explicit status for binary model support. Native COLMAP execution
can be layered behind the same API later without changing callers.

## Feature flags

- No optional feature flags today.

## Related crates

- `video-analysis-radiance-io`
- `video-analysis-sfm`
