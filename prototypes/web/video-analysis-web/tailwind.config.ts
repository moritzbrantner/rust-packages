import type { Config } from "tailwindcss";
import videoAnalysisContent from "@video-analysis/ui/tailwind-content";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}", ...videoAnalysisContent],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
