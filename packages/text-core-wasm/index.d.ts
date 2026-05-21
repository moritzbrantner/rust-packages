import init, { initSync } from "./pkg/index";

export type TokenKind =
  | "word"
  | "number"
  | "url"
  | "email"
  | "mention"
  | "hashtag"
  | "punctuation"
  | "other";

export interface TextProcessingOptions {
  lowercase?: boolean;
  normalizeUnicode?: boolean;
  keepApostrophes?: boolean;
  includePunctuation?: boolean;
  includeTokens?: boolean;
}

export interface TextSpan {
  start: number;
  end: number;
  text: string;
}

export interface TextToken {
  start: number;
  end: number;
  text: string;
  normalized: string;
  kind: TokenKind;
}

export interface TextStats {
  bytes: number;
  chars: number;
  words: number;
  lines: number;
  sentences: number;
  paragraphs: number;
  tokens: number;
  uniqueTokens: number;
  averageWordsPerSentence: number;
  averageCharsPerWord: number;
}

export interface ScriptProfile {
  scripts: Record<string, number>;
  digits: number;
  whitespace: number;
  punctuation: number;
  other: number;
  dominantScript: string | null;
  isMixed: boolean;
}

export interface TextDocumentAnalysis {
  stats: TextStats;
  scriptProfile: ScriptProfile;
  paragraphs: TextSpan[];
  sentences: TextSpan[];
  tokens: TextToken[];
}

export function analyzeTextDocument(
  text: string,
  options?: TextProcessingOptions,
): TextDocumentAnalysis;

export function extractWordTexts(text: string): string[];

export function splitSentences(text: string): string[];

export function segmentTextDocument(
  text: string,
  keepApostrophes: boolean,
  includePunctuation: boolean,
  includeTokens: boolean,
): {
  paragraphs: TextSpan[];
  sentences: TextSpan[];
  tokens: Array<Omit<TextToken, "normalized">>;
};

export { initSync };
export default init;
