# Vision Core Shared Visual Contracts

The workspace now uses `moritzbrantner-vision-core` for shared visual detections, keypoints, source-aware visual embeddings, and identity match summaries. This is a deliberate departure from growing `video-analysis-core` as the owner of every visual intermediate type: the clean shared image/video language is worth the migration cost, while specialized crates continue to own model execution, tracking, reference libraries, and adapter-specific DTOs.
