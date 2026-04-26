import { describe, expect, it } from "vitest";

import { buildCommand, getRunValidation, initialForm } from "./run-config";

describe("run config", () => {
  it("builds the youtube use-case command with optional flags", () => {
    const command = buildCommand({
      ...initialForm,
      url: "https://www.youtube.com/watch?v=demo",
      maxFrames: "120",
      skipTranscription: true,
      objectCommand: "python detect.py",
    });

    expect(command).toContain("--url");
    expect(command).toContain("https://www.youtube.com/watch?v=demo");
    expect(command).toContain("--max-frames");
    expect(command).toContain("120");
    expect(command).toContain("--skip-transcription");
    expect(command).toContain("--object-command");
  });

  it("rejects invalid input before a run starts", () => {
    expect(getRunValidation({ ...initialForm, url: "" })).toBe("Enter a YouTube URL.");
    expect(getRunValidation({ ...initialForm, url: "https://example.com/video" })).toBe(
      "Use a youtube.com or youtu.be URL.",
    );
    expect(getRunValidation({ ...initialForm, url: "https://youtu.be/demo", maxFrames: "0" })).toBe(
      "Max frames must be a positive integer.",
    );
  });

  it("accepts a valid file-based run configuration", () => {
    expect(
      getRunValidation({
        ...initialForm,
        sourceMode: "file",
        url: "",
        input: "./fixtures/video.mp4",
      }),
    ).toBeNull();
  });
});
