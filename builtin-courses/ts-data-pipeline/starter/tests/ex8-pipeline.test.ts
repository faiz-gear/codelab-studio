import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { runDataPipeline } from "../src/pipeline.ts";

describe("pipeline", () => {
  test("normalizes deduplicates enriches and summarizes end-to-end", async () => {
    const summary = await runDataPipeline(
      [
        { id: " ", category: "books", amount: 99, latencyMs: 5, timestamp: 1, tags: [] },
        { id: "e1", category: " Books ", amount: 40, latencyMs: 10, timestamp: 100, tags: ["New"] },
        { id: "e1", category: "books", amount: 55.5, latencyMs: 15, timestamp: 200, tags: ["VIP"] },
        { id: "e2", category: "Tools", amount: 30, latencyMs: -5, timestamp: 150, tags: [] },
      ],
      2,
      async (event) => ({
        ...event,
        amount: event.amount + 4.5,
      })
    );
    assert.deepStrictEqual(summary, {
      processedCount: 2,
      categories: {
        books: { count: 1, totalAmount: 60, maxLatencyMs: 15 },
        tools: { count: 1, totalAmount: 34.5, maxLatencyMs: 0 },
      },
      topCategory: "books",
    });
  });

  test("enforces concurrency limit during enrichment", async () => {
    let active = 0;
    let peak = 0;
    await runDataPipeline(
      [
        { id: "a", category: "x", amount: 1, latencyMs: 1, timestamp: 1, tags: [] },
        { id: "b", category: "x", amount: 1, latencyMs: 1, timestamp: 2, tags: [] },
        { id: "c", category: "x", amount: 1, latencyMs: 1, timestamp: 3, tags: [] },
        { id: "d", category: "x", amount: 1, latencyMs: 1, timestamp: 4, tags: [] },
      ],
      2,
      async (event) => {
        active += 1;
        peak = Math.max(peak, active);
        await new Promise((resolve) => setTimeout(resolve, 10));
        active -= 1;
        return event;
      }
    );
    assert.strictEqual(peak, 2);
  });

  test("returns null topCategory when no valid events remain", async () => {
    const summary = await runDataPipeline(
      [{ id: "", category: "books", amount: 10, latencyMs: 1, timestamp: 1, tags: [] }],
      2,
      async (event) => event
    );
    assert.deepStrictEqual(summary, {
      processedCount: 0,
      categories: {},
      topCategory: null,
    });
  });
});
