import { beforeAll, expect, test } from "bun:test";

import init, { resampleLineFlat, resampleRingFlat } from "./index.js";

beforeAll(async () => {
  await init();
});

test("resamples flat line coordinates through packaged wasm bindings", () => {
  expect(Array.from(resampleLineFlat(new Float64Array([0, 0, 10, 0]), 3))).toEqual([
    0, 0, 5, 0, 10, 0,
  ]);
});

test("resamples flat open ring coordinates through packaged wasm bindings", () => {
  expect(Array.from(resampleRingFlat(new Float64Array([0, 0, 10, 0, 10, 10, 0, 10]), 4))).toEqual([
    0, 0, 10, 0, 10, 10, 0, 10,
  ]);
});
