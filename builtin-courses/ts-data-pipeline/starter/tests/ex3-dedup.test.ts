import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { dedupKeepLatest } from "../src/dedup.ts";

describe("dedup", () => {
  test("keeps the record with the largest timestamp per id", () => {
    const result = dedupKeepLatest([
      { id: "e1", category: "books", amount: 10, latencyMs: 5, timestamp: 100, tags: [] },
      { id: "e1", category: "books", amount: 25, latencyMs: 8, timestamp: 200, tags: [] },
      { id: "e1", category: "books", amount: 12, latencyMs: 4, timestamp: 150, tags: [] },
    ]);
    assert.strictEqual(result.length, 1);
    assert.strictEqual(result[0]?.amount, 25);
    assert.strictEqual(result[0]?.timestamp, 200);
  });

  test("breaks timestamp ties by keeping the later input record", () => {
    const result = dedupKeepLatest([
      { id: "e1", category: "books", amount: 10, latencyMs: 5, timestamp: 300, tags: ["old"] },
      { id: "e1", category: "books", amount: 99, latencyMs: 9, timestamp: 300, tags: ["new"] },
    ]);
    assert.strictEqual(result[0]?.amount, 99);
    assert.deepStrictEqual(result[0]?.tags, ["new"]);
  });

  test("returns results sorted ascending by timestamp then id", () => {
    const result = dedupKeepLatest([
      { id: "b", category: "books", amount: 1, latencyMs: 1, timestamp: 200, tags: [] },
      { id: "a", category: "books", amount: 2, latencyMs: 1, timestamp: 200, tags: [] },
      { id: "c", category: "books", amount: 3, latencyMs: 1, timestamp: 100, tags: [] },
    ]);
    assert.deepStrictEqual(
      result.map((item) => item.id),
      ["c", "a", "b"]
    );
  });
});
