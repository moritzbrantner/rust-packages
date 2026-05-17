import { beforeAll, expect, test } from "bun:test";

import init, { extractWordTexts, segmentTextDocument, splitSentences } from "./index.js";

beforeAll(async () => {
  await init();
});

test("segments text through the packaged wasm bindings", () => {
  expect(extractWordTexts("Rust tests, shipped.")).toEqual(["Rust", "tests", "shipped"]);
  expect(splitSentences("First sentence. Second sentence.")).toEqual([
    "First sentence.",
    "Second sentence.",
  ]);

  const document = segmentTextDocument("Hello from wasm.", true, false, true);
  expect(document.sentences[0].text).toBe("Hello from wasm.");
  expect(document.tokens.map((token) => token.text)).toEqual(["Hello", "from", "wasm"]);
});
