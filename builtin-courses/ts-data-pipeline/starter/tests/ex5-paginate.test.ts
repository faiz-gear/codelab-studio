import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { paginateCursor } from "../src/paginate.ts";

describe("paginate", () => {
  const rows = [
    { id: "a", value: 1 },
    { id: "b", value: 2 },
    { id: "c", value: 3 },
    { id: "d", value: 4 },
    { id: "e", value: 5 },
  ];

  test("returns first page and nextCursor when more items remain", () => {
    const page = paginateCursor(rows, null, 2);
    assert.deepStrictEqual(
      page.items.map((row) => row.id),
      ["a", "b"]
    );
    assert.strictEqual(page.nextCursor, "b");
    assert.strictEqual(page.hasMore, true);
  });

  test("continues after given cursor until last page", () => {
    const page2 = paginateCursor(rows, "b", 2);
    assert.deepStrictEqual(
      page2.items.map((row) => row.id),
      ["c", "d"]
    );
    assert.strictEqual(page2.nextCursor, "d");
    assert.strictEqual(page2.hasMore, true);

    const page3 = paginateCursor(rows, "d", 2);
    assert.deepStrictEqual(
      page3.items.map((row) => row.id),
      ["e"]
    );
    assert.strictEqual(page3.nextCursor, null);
    assert.strictEqual(page3.hasMore, false);
  });

  test("rejects non-positive limit and handles unknown cursor", () => {
    assert.throws(() => paginateCursor(rows, null, 0), RangeError);
    assert.throws(() => paginateCursor(rows, null, -3), RangeError);
    assert.deepStrictEqual(paginateCursor(rows, "missing", 2), {
      items: [],
      nextCursor: null,
      hasMore: false,
    });
  });
});
