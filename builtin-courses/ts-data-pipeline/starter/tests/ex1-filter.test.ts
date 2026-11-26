import { describe, test } from "node:test";
import assert from "node:assert/strict";
import { getAdultNames } from "../src/filter.ts";

describe("filter", () => {
  test("filters underage users and returns names only", () => {
    const users = [
      { name: "小林", age: 26 },
      { name: "阿杰", age: 16 },
      { name: "美玲", age: 19 },
    ];
    assert.deepStrictEqual(getAdultNames(users), ["小林", "美玲"]);
  });

  test("treats age 18 as an adult", () => {
    const users = [
      { name: "阿明", age: 18 },
      { name: "小雨", age: 17 },
    ];
    assert.deepStrictEqual(getAdultNames(users), ["阿明"]);
  });

  test("returns empty array without mutating input", () => {
    const input = [
      { name: "Alice", age: 21 },
      { name: "Bob", age: 14 },
    ];
    const snapshot = JSON.stringify(input);
    const result = getAdultNames(input);
    assert.deepStrictEqual(getAdultNames([]), []);
    assert.deepStrictEqual(result, ["Alice"]);
    assert.strictEqual(JSON.stringify(input), snapshot);
  });
});
