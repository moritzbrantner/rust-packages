export type SourceMode = "url" | "file";

export interface UseCaseForm {
  sourceMode: SourceMode;
  url: string;
  input: string;
  output: string;
  workDir: string;
  sceneThreshold: number;
  minSceneLen: number;
  maxFrames: string;
  visualSampleEvery: number;
  skipTranscription: boolean;
  objectCommand: string;
  ocrCommand: string;
  textCommand: string;
}

export const initialForm: UseCaseForm = {
  sourceMode: "url",
  url: "",
  input: "./video.mp4",
  output: "use-case-output/youtube-video/analysis.json",
  workDir: "use-case-output/youtube-video",
  sceneThreshold: 27,
  minSceneLen: 15,
  maxFrames: "",
  visualSampleEvery: 30,
  skipTranscription: false,
  objectCommand: "",
  ocrCommand: "",
  textCommand: "",
};

export function buildCommand(form: UseCaseForm): string[] {
  const command = [
    "cargo",
    "run",
    "-p",
    "video-analysis-use-cases",
    "--",
    "youtube-video",
    "--work-dir",
    form.workDir,
    "--output",
    form.output,
    "--scene-threshold",
    String(form.sceneThreshold),
    "--min-scene-len",
    String(form.minSceneLen),
    "--visual-sample-every",
    String(form.visualSampleEvery),
  ];

  if (form.sourceMode === "url") {
    command.push("--url", form.url);
  } else {
    command.push("--input", form.input);
  }
  if (form.maxFrames.trim()) {
    command.push("--max-frames", form.maxFrames.trim());
  }
  if (form.skipTranscription) {
    command.push("--skip-transcription");
  }
  if (form.objectCommand.trim()) {
    command.push("--object-command", form.objectCommand.trim());
  }
  if (form.ocrCommand.trim()) {
    command.push("--ocr-command", form.ocrCommand.trim());
  }
  if (form.textCommand.trim()) {
    command.push("--text-command", form.textCommand.trim());
  }

  return command;
}

export function getRunValidation(form: UseCaseForm): string | null {
  if (form.sourceMode === "url") {
    const url = form.url.trim();
    if (!url) {
      return "Enter a YouTube URL.";
    }
    try {
      const parsed = new URL(url);
      const host = parsed.hostname.toLowerCase().replace(/^www\./, "");
      if (!["youtube.com", "m.youtube.com", "youtu.be", "music.youtube.com"].includes(host)) {
        return "Use a youtube.com or youtu.be URL.";
      }
      if (!["http:", "https:"].includes(parsed.protocol)) {
        return "Use an http or https URL.";
      }
    } catch {
      return "Enter a valid YouTube URL.";
    }
  }
  if (form.sourceMode === "file" && !form.input.trim()) {
    return "Enter a local video file path.";
  }
  if (!form.output.trim()) {
    return "Enter an output JSON path.";
  }
  if (!form.workDir.trim()) {
    return "Enter a work directory.";
  }
  if (!Number.isFinite(form.sceneThreshold) || form.sceneThreshold < 0) {
    return "Scene threshold must be zero or greater.";
  }
  if (!Number.isFinite(form.minSceneLen) || form.minSceneLen < 1) {
    return "Minimum scene frames must be at least 1.";
  }
  if (!Number.isFinite(form.visualSampleEvery) || form.visualSampleEvery < 1) {
    return "Visual sample step must be at least 1.";
  }
  if (form.maxFrames.trim() && !/^[1-9]\d*$/.test(form.maxFrames.trim())) {
    return "Max frames must be a positive integer.";
  }
  return null;
}

export function shellQuote(value: string): string {
  if (/^[A-Za-z0-9_./:=+-]+$/.test(value)) {
    return value;
  }
  return `'${value.replace(/'/g, "'\\''")}'`;
}
