import { describe, expect, it } from "vitest";
import type { TranscriptSegmentReport } from "./types";

interface TranscriptSegmentContract {
  index: number;
  startSeconds?: number | null;
  endSeconds?: number | null;
  text: string;
  language?: string | null;
  speaker?: string | null;
  confidence?: number | null;
  isFinal: boolean;
  words: unknown[];
  attributes: Record<string, string>;
}

function reportSegmentFromContract(
  segment: TranscriptSegmentContract,
): TranscriptSegmentReport {
  return {
    index: segment.index,
    start_seconds: segment.startSeconds,
    end_seconds: segment.endSeconds,
    text: segment.text,
  };
}

describe("report transcript type projection", () => {
  it("keeps report segments as a projection of transcript contracts", () => {
    const contract = {
      index: 7,
      startSeconds: 1.5,
      endSeconds: 2.25,
      text: "stable transcript contract",
      language: "en",
      speaker: "speaker_0",
      confidence: 0.9,
      isFinal: true,
      words: [],
      attributes: {},
    } satisfies TranscriptSegmentContract;

    const report = reportSegmentFromContract(contract);

    expect(report).toEqual({
      index: 7,
      start_seconds: 1.5,
      end_seconds: 2.25,
      text: "stable transcript contract",
    });
  });
});
