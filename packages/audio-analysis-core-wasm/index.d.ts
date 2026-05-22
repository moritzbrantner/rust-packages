import init, { initSync } from "./pkg/index";

export interface AudioAnalysisOptions {
  sampleRate?: number;
  channels?: number;
  channelMix?: "average" | "first";
  frameSize?: number;
  hopSize?: number;
  fftSize?: number;
  minFrequencyHz?: number;
  maxFrequencyHz?: number;
  confidenceThreshold?: number;
}

export interface AudioPitchEstimate {
  frequencyHz: number | null;
  confidence: number;
  midiNote: number | null;
  noteName: string | null;
}

export interface AudioSampleAnalysis {
  sampleRate: number;
  channels: number;
  sampleCount: number;
  samplesPerChannel: number;
  durationSeconds: number;
  rms: number;
  peak: number;
  meanAbsolute: number;
  zeroCrossingRate: number;
  frameCount: number;
  dominantFrequencyHz: number | null;
  pitch: AudioPitchEstimate;
}

export interface AudioFramePlan {
  frameSize: number;
  hopSize: number;
  sampleCount: number;
  frameCount: number;
  starts: number[];
}

export function analyzeAudioSamples(
  samples: Float32Array | number[],
  options?: AudioAnalysisOptions,
): AudioSampleAnalysis;

export function mixToMono(
  samples: Float32Array | number[],
  channels: number,
  mix?: "average" | "first",
): number[];

export function planAudioFrames(
  samplesLen: number,
  frameSize: number,
  hopSize: number,
): AudioFramePlan;

export { initSync };
export default init;
