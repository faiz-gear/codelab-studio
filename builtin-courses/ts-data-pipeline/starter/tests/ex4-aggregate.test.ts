import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { summarizeByCategory } from "../src/aggregate.ts";

describe("aggregate", () => {
  test("aggregates count totalAmount and maxLatencyMs per category", () => {
    const summary = summarizeByCategory([
      { id: "1", category: "books", amount: 42, latencyMs: 12, timestamp: 1, tags: [] },
      { id: "2", category: "books", amount: 58, latencyMs: 35, timestamp: 2, tags: [] },
      { id: "3", category: "games", amount: 120, latencyMs: 20, timestamp: 3, tags: [] },
    ]);
    assert.deepStrictEqual(summary, {
      books: { count: 2, totalAmount: 100, maxLatencyMs: 35 },
      games: { count: 1, totalAmount: 120, maxLatencyMs: 20 },
    });
  });

  test("rounds floating point amounts to two decimals", () => {
    const summary = summarizeByCategory([
      { id: "1", category: "coffee", amount: 0.1, latencyMs: 3, timestamp: 1, tags: [] },
      { id: "2", category: "coffee", amount: 0.2, latencyMs: 4, timestamp: 2, tags: [] },
    ]);
    assert.strictEqual(summary.coffee?.totalAmount, 0.3);
  });

  test("returns empty object for empty event list", () => {
    assert.deepStrictEqual(summarizeByCategory([]), {});
  });
});
