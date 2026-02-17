import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { compareVersionsDesc } from "./downloader.js";

describe("compareVersionsDesc", () => {
  it("sorts simple versions in descending order", () => {
    const input = ["v0.1.0", "v0.2.0", "v0.1.5"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v0.2.0", "v0.1.5", "v0.1.0"]);
  });

  it("handles v0.9.0 vs v0.10.0 correctly (not lexicographic)", () => {
    const input = ["v0.9.0", "v0.10.0", "v0.2.0"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v0.10.0", "v0.9.0", "v0.2.0"]);
  });

  it("handles major version differences", () => {
    const input = ["v1.0.0", "v2.0.0", "v0.9.0"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v2.0.0", "v1.0.0", "v0.9.0"]);
  });

  it("handles patch version differences", () => {
    const input = ["v0.1.1", "v0.1.3", "v0.1.2"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v0.1.3", "v0.1.2", "v0.1.1"]);
  });

  it("handles double-digit version components", () => {
    const input = ["v1.2.3", "v1.12.0", "v1.2.30"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v1.12.0", "v1.2.30", "v1.2.3"]);
  });

  it("keeps equal versions stable", () => {
    const input = ["v0.1.6", "v0.1.6"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v0.1.6", "v0.1.6"]);
  });

  it("handles single element", () => {
    const input = ["v0.1.0"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, ["v0.1.0"]);
  });

  it("handles empty array", () => {
    const input: string[] = [];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, []);
  });

  it("handles realistic release sequence", () => {
    const input = ["v0.1.0", "v0.1.6", "v0.1.5", "v0.2.0", "v0.1.10"];
    const result = [...input].sort(compareVersionsDesc);
    assert.deepStrictEqual(result, [
      "v0.2.0",
      "v0.1.10",
      "v0.1.6",
      "v0.1.5",
      "v0.1.0",
    ]);
  });
});
