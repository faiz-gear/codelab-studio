import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { collectSettled } from "../src/settled.ts";

describe("settled", () => {
  test("collects fulfilled values and rejected messages without short-circuiting", async () => {
    const result = await collectSettled([1, 2, 3, 4], async (n) => {
      if (n % 2 === 0) {
        throw new Error(`even ${n} rejected`);
      }
      return n * 10;
    });
    assert.deepStrictEqual(result, {
      fulfilled: [10, 30],
      rejected: ["even 2 rejected", "even 4 rejected"],
    });
  });

  test("preserves fulfilled order matching input sequence", async () => {
    const delays = [30, 5, 15];
    const result = await collectSettled(delays, async (ms) => {
      await new Promise((resolve) => setTimeout(resolve, ms));
      return ms;
    });
    assert.deepStrictEqual(result.fulfilled, [30, 5, 15]);
    assert.deepStrictEqual(result.rejected, []);
  });

  test("handles empty batch cleanly", async () => {
    const result = await collectSettled([], async (item: number) => item);
    assert.deepStrictEqual(result, { fulfilled: [], rejected: [] });
  });
});
