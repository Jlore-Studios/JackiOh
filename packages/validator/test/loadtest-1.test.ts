// #375's load test for #318: `reverseWords` reverses word order, collapsing runs of spaces to
// one and dropping the empties a leading or trailing space would split off.

import { describe, expect, it } from "vitest";
import { reverseWords } from "../src/loadtest-1";

describe("reverseWords", () => {
  it("reverses words split on runs of spaces, joined by one space", () => {
    expect(reverseWords("a  b c")).toBe("c b a");
  });

  it("returns the empty string unchanged", () => {
    expect(reverseWords("")).toBe("");
  });
});
