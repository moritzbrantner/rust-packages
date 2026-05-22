import init, { initSync } from "./pkg/index";

export interface FrameTimecodeResult {
  frameIndex: number;
  seconds: number;
  timecode: string;
}

export interface Rgb8 {
  r: number;
  g: number;
  b: number;
}

export interface MeanRgb {
  r: number;
  g: number;
  b: number;
}

export interface VideoFrameAnalysis {
  width: number;
  height: number;
  pixelFormat: "rgb24" | "bgr24";
  pixelCount: number;
  frameIndex: number;
  seconds: number;
  timecode: string;
  topLeft: Rgb8;
  center: Rgb8;
  meanRgb: MeanRgb;
}

export interface SceneInterval {
  startFrame: number;
  endFrame: number;
  startSeconds: number;
  endSeconds: number;
}

export function frameTimecode(
  frameIndex: number,
  fpsNumerator: number,
  fpsDenominator: number,
  precision: number,
): FrameTimecodeResult;

export function parseFrameTimecode(
  input: string,
  fpsNumerator: number,
  fpsDenominator: number,
  precision: number,
): FrameTimecodeResult;

export function analyzeVideoFrame(
  data: Uint8Array | number[],
  width: number,
  height: number,
  pixelFormat: "rgb24" | "bgr24",
  frameIndex: number,
  fpsNumerator: number,
  fpsDenominator: number,
  precision: number,
): VideoFrameAnalysis;

export function scenesFromCutFrames(
  cutFrames: number[],
  totalFrames: number,
  fpsNumerator: number,
  fpsDenominator: number,
): SceneInterval[];

export { initSync };
export default init;
