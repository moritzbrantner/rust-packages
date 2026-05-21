import type { TextDocumentAnalysis } from "./textCoreWasm";

export type TextSpan = TextDocumentAnalysis["sentences"][number];
export type TextToken = TextDocumentAnalysis["tokens"][number];
export type TokenKind = TextToken["kind"];
export type TokenKindFilter = TokenKind | "all";

export const tokenKindFilters: TokenKindFilter[] = [
  "all",
  "word",
  "number",
  "url",
  "email",
  "mention",
  "hashtag",
  "punctuation",
  "other",
];

export function countSentenceTokens(sentence: TextSpan, tokens: TextToken[]): number {
  return tokens.filter((token) => token.start >= sentence.start && token.end <= sentence.end)
    .length;
}

export function countParagraphSentences(paragraph: TextSpan, sentences: TextSpan[]): number {
  return sentences.filter(
    (sentence) => sentence.start >= paragraph.start && sentence.end <= paragraph.end,
  ).length;
}

export function tokenKindLabel(kind: TokenKindFilter): string {
  switch (kind) {
    case "all":
      return "All";
    case "url":
      return "URL";
    default:
      return kind.charAt(0).toUpperCase() + kind.slice(1);
  }
}

export function tokenKindClass(kind: TokenKind): string {
  switch (kind) {
    case "word":
      return "bg-sky-100 text-sky-900 focus:ring-sky-500";
    case "number":
      return "bg-amber-100 text-amber-900 focus:ring-amber-500";
    case "url":
      return "bg-violet-100 text-violet-900 focus:ring-violet-500";
    case "email":
      return "bg-cyan-100 text-cyan-900 focus:ring-cyan-500";
    case "mention":
      return "bg-fuchsia-100 text-fuchsia-900 focus:ring-fuchsia-500";
    case "hashtag":
      return "bg-lime-100 text-lime-900 focus:ring-lime-500";
    case "punctuation":
      return "bg-zinc-200 text-zinc-800 focus:ring-zinc-500";
    case "other":
      return "bg-rose-100 text-rose-900 focus:ring-rose-500";
  }
}

export function filterTokens(
  tokens: TextToken[],
  kind: TokenKindFilter,
  query: string,
): TextToken[] {
  const needle = query.trim().toLowerCase();
  return tokens.filter((token) => {
    if (kind !== "all" && token.kind !== kind) {
      return false;
    }
    if (!needle) {
      return true;
    }
    return (
      token.text.toLowerCase().includes(needle) || token.normalized.toLowerCase().includes(needle)
    );
  });
}

export function formatNumber(value: number): string {
  if (Number.isInteger(value)) {
    return value.toLocaleString();
  }
  return value.toLocaleString(undefined, { maximumFractionDigits: 2 });
}
