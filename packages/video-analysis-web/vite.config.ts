import react from "@vitejs/plugin-react";
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

const workspaceRoot = fileURLToPath(new URL("../..", import.meta.url));

interface UseCaseFormPayload {
  sourceMode?: string;
  url?: string;
  input?: string;
  output?: string;
  workDir?: string;
  sceneThreshold?: number;
  minSceneLen?: number;
  maxFrames?: string;
  visualSampleEvery?: number;
  skipTranscription?: boolean;
  objectCommand?: string;
  ocrCommand?: string;
  textCommand?: string;
}

interface CommandResult {
  exitCode: number | null;
  stdout: string;
  stderr: string;
}

export default defineConfig({
  plugins: [react(), youtubeAnalysisApi()],
});

function youtubeAnalysisApi(): Plugin {
  return {
    name: "youtube-analysis-api",
    configureServer(server) {
      server.middlewares.use("/api/run-youtube-video", handleRunYoutubeVideo);
    },
    configurePreviewServer(server) {
      server.middlewares.use("/api/run-youtube-video", handleRunYoutubeVideo);
    },
  };
}

async function handleRunYoutubeVideo(req: any, res: any, next: any) {
  if (req.method !== "POST") {
    next();
    return;
  }

  try {
    const form = normalizeForm(await readJsonBody(req));
    const args = buildUseCaseArgs(form);
    const result = await runCommand("cargo", args);
    const outputPath = resolveReportPath(form.output);

    if (result.exitCode !== 0) {
      sendJson(res, 500, {
        run: {
          command: "cargo",
          args,
          status: "failed",
          exit_code: result.exitCode,
          output_files: [form.output],
          message: result.stderr.trim() || result.stdout.trim() || "analysis failed",
        },
        stdout: result.stdout,
        stderr: result.stderr,
      });
      return;
    }

    const report = JSON.parse(await readFile(outputPath, "utf8"));
    sendJson(res, 200, {
      report,
      run: {
        command: "cargo",
        args,
        status: "succeeded",
        exit_code: result.exitCode,
        output_files: [form.output],
        message: result.stdout.trim() || "analysis completed",
      },
      stdout: result.stdout,
      stderr: result.stderr,
    });
  } catch (error) {
    sendJson(res, 400, {
      run: {
        command: "cargo",
        args: [],
        status: "failed",
        exit_code: null,
        output_files: [],
        message: error instanceof Error ? error.message : String(error),
      },
    });
  }
}

function normalizeForm(input: unknown): Required<UseCaseFormPayload> {
  if (!input || typeof input !== "object") {
    throw new Error("request body must be a JSON object");
  }

  const body = input as UseCaseFormPayload;
  const sourceMode = body.sourceMode === "file" ? "file" : "url";
  const output = cleanString(body.output) || "use-case-output/youtube-video/analysis.json";
  const workDir = cleanString(body.workDir) || "use-case-output/youtube-video";
  const sceneThreshold = finiteNumber(body.sceneThreshold, 27);
  const minSceneLen = positiveInteger(body.minSceneLen, 15);
  const visualSampleEvery = positiveInteger(body.visualSampleEvery, 30);
  const maxFrames = cleanString(body.maxFrames);

  if (sourceMode === "url") {
    validateYoutubeUrl(body.url);
  } else if (!cleanString(body.input)) {
    throw new Error("input file is required");
  }

  return {
    sourceMode,
    url: cleanString(body.url),
    input: cleanString(body.input),
    output,
    workDir,
    sceneThreshold,
    minSceneLen,
    maxFrames,
    visualSampleEvery,
    skipTranscription: body.skipTranscription === true,
    objectCommand: cleanString(body.objectCommand),
    ocrCommand: cleanString(body.ocrCommand),
    textCommand: cleanString(body.textCommand),
  };
}

function buildUseCaseArgs(form: Required<UseCaseFormPayload>): string[] {
  const args = [
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
    args.push("--url", form.url);
  } else {
    args.push("--input", form.input);
  }
  if (form.maxFrames) {
    args.push("--max-frames", form.maxFrames);
  }
  if (form.skipTranscription) {
    args.push("--skip-transcription");
  }
  if (form.objectCommand) {
    args.push("--object-command", form.objectCommand);
  }
  if (form.ocrCommand) {
    args.push("--ocr-command", form.ocrCommand);
  }
  if (form.textCommand) {
    args.push("--text-command", form.textCommand);
  }

  return args;
}

function runCommand(command: string, args: string[]): Promise<CommandResult> {
  return new Promise((resolveCommand, reject) => {
    const child = spawn(command, args, {
      cwd: workspaceRoot,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout = appendChunk(stdout, chunk);
    });
    child.stderr.on("data", (chunk) => {
      stderr = appendChunk(stderr, chunk);
    });
    child.on("error", reject);
    child.on("close", (exitCode) => resolveCommand({ exitCode, stdout, stderr }));
  });
}

function appendChunk(current: string, chunk: Buffer): string {
  const next = current + chunk.toString("utf8");
  return next.length > 200_000 ? next.slice(next.length - 200_000) : next;
}

function resolveReportPath(path: string): string {
  return resolve(workspaceRoot, path);
}

function validateYoutubeUrl(value: unknown) {
  const urlText = cleanString(value);
  if (!urlText) {
    throw new Error("YouTube URL is required");
  }
  let url: URL;
  try {
    url = new URL(urlText);
  } catch {
    throw new Error("YouTube URL must be a valid http(s) URL");
  }
  if (!["http:", "https:"].includes(url.protocol)) {
    throw new Error("YouTube URL must use http or https");
  }
  const host = url.hostname.toLowerCase().replace(/^www\./, "");
  if (!["youtube.com", "m.youtube.com", "youtu.be", "music.youtube.com"].includes(host)) {
    throw new Error("URL must point to youtube.com or youtu.be");
  }
}

function cleanString(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function finiteNumber(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function positiveInteger(value: unknown, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fallback;
  }
  return Math.max(1, Math.round(value));
}

function readJsonBody(req: any): Promise<unknown> {
  return new Promise((resolveBody, reject) => {
    let body = "";
    req.on("data", (chunk: Buffer) => {
      body += chunk.toString("utf8");
      if (body.length > 32_000) {
        reject(new Error("request body is too large"));
      }
    });
    req.on("end", () => {
      try {
        resolveBody(body ? JSON.parse(body) : {});
      } catch {
        reject(new Error("request body must be valid JSON"));
      }
    });
    req.on("error", reject);
  });
}

function sendJson(res: any, status: number, body: unknown) {
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json");
  res.end(JSON.stringify(body));
}
