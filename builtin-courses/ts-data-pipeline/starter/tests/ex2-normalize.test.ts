import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { normalizeEvents } from "../src/normalize.ts";

describe("normalize", () => {
  test("trims fields and lowercases category and tags", () => {
    const normalized = normalizeEvents([
      {
        id: "  evt-1 ",
        category: "  Orders ",
        amount: 49.5,
        latencyMs: 18,
        timestamp: 1000,
        tags: [" VIP ", "Express"],
      },
    ]);
    assert.deepStrictEqual(normalized, [
      {
        id: "evt-1",
        category: "orders",
        amount: 49.5,
        latencyMs: 18,
        timestamp: 1000,
        tags: ["vip", "express"],
      },
    ]);
  });

  test("drops records with blank id or category", () => {
    const normalized = normalizeEvents([
      {
        id: "   ",
        category: "orders",
        amount: 10,
        latencyMs: 5,
        timestamp: 100,
        tags: [],
      },
      {
        id: "evt-2",
        category: "  ",
        amount: 10,
        latencyMs: 5,
        timestamp: 200,
        tags: [],
      },
      {
        id: "evt-3",
        category: "books",
        amount: 20,
        latencyMs: 8,
        timestamp: 300,
        tags: [],
      },
    ]);
    assert.deepStrictEqual(
      normalized.map((item) => item.id),
      ["evt-3"]
    );
  });

  test("clamps negative latency to zero and removes duplicate tags", () => {
    const normalized = normalizeEvents([
      {
        id: "evt-4",
        category: "Books",
        amount: 15,
        latencyMs: -12,
        timestamp: 500,
        tags: [" Sale ", "", "sale", "NEW", "new"],
      },
    ]);
    assert.strictEqual(normalized[0]?.latencyMs, 0);
    assert.deepStrictEqual(normalized[0]?.tags, ["sale", "new"]);
  });
});
