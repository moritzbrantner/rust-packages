import type { Config } from "tailwindcss";
import videoAnalysisPackageContent from "../../../packages/video-analysis-ui/src/tailwind-content";

export default {
  content: [
    "./index.html",
    "./src/**/*.{ts,tsx}",
    "../../../packages/video-analysis-ui/src/**/*.{ts,tsx}",
    "../../../packages/*-app/src/**/*.{ts,tsx}",
    ...videoAnalysisPackageContent,
  ],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
