import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { runWithConcurrency } from "../src/concurrency.ts";

describe("concurrency", () => {
  test("never exceeds the configured concurrency limit", async () => {
    let active = 0;
    let peak = 0;
    const output = await runWithConcurrency([1, 2, 3, 4, 5, 6], 2, async (item) => {
      active += 1;
      peak = Math.max(peak, active);
      await new Promise((resolve) => setTimeout(resolve, 10));
      active -= 1;
      return item * 2;
    });
    assert.deepStrictEqual(output, [2, 4, 6, 8, 10, 12]);
    assert.strictEqual(peak, 2);
  });

  test("preserves input ordering despite out-of-order completion", async () => {
    const output = await runWithConcurrency([30, 5, 20, 1], 3, async (ms) => {
      await new Promise((resolve) => setTimeout(resolve, ms));
      return `done:${ms}`;
    });
    assert.deepStrictEqual(output, ["done:30", "done:5", "done:20", "done:1"]);
  });

  test("rejects non-positive concurrency limit", async () => {
    await assert.rejects(() => runWithConcurrency([1], 0, async (n) => n), RangeError);
    await assert.rejects(() => runWithConcurrency([1], -1, async (n) => n), RangeError);
  });
});
