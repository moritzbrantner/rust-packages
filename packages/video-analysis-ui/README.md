# @moritzbrantner/video-analysis-ui

React components styled with TailwindCSS for the Rust `video-analysis-*` crates.
The package exports one component pack per crate boundary plus a composed
dashboard for `video-analysis-use-cases` JSON reports.
Shared primitives compose `@moritzbrantner/ui`, so consumers need access to the
`@moritzbrantner` package registry when installing dependencies.

```tsx
import { YoutubeVideoReportView } from "@moritzbrantner/video-analysis-ui/use-cases";
import type { YoutubeVideoReport } from "@moritzbrantner/video-analysis-ui";

export function ReportPage({ report }: { report: YoutubeVideoReport }) {
  return <YoutubeVideoReportView report={report} />;
}
```

Add the package output to your Tailwind `content` list so utility classes are
generated for this package and the `@moritzbrantner/ui` primitives it uses:

```js
import videoAnalysisContent from "@moritzbrantner/video-analysis-ui/tailwind-content";

export default {
  content: ["./src/**/*.{ts,tsx}", ...videoAnalysisContent],
};
```

Subpath imports are available for smaller bundles:

```tsx
import { SceneTimeline, ObservationList } from "@moritzbrantner/video-analysis-ui/core";
import { CliRunPanel } from "@moritzbrantner/video-analysis-ui/cli";
import { DataBucketOverview } from "@moritzbrantner/video-analysis-ui/data";
import { CapabilityPanel } from "@moritzbrantner/video-analysis-ui/models";
```
