import { expect, test } from "bun:test";

import { storyIdsFromIndex } from "./storybook-validation";

test("selects executable stories from the Storybook index", () => {
  expect(
    storyIdsFromIndex({
      entries: {
        "report--default": { id: "report--default", type: "story" },
        "report--docs": { id: "report--docs", type: "docs" },
        malformed: { type: "story" },
      },
    }),
  ).toEqual(["report--default"]);
});

test("rejects a Storybook index without executable stories", () => {
  expect(() => storyIdsFromIndex({ entries: {} })).toThrow(
    "Storybook index did not contain any stories.",
  );
});
