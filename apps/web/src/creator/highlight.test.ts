// #920: the code view's highlighting cuts a kit script into tokens that join back to it exactly.

import { describe, expect, it } from "vitest";

import { tokenLines, tokenize } from "./highlight.ts";

const SCRIPT = [
  "-- A comment",
  "return Card {",
  '  target = "an enemy unit",',
  "  cry = function(c, n)",
  "    if c:count('friendly units') >= 2 then",
  "      c:damage(c:target(), n.dmg) --[[ long",
  "comment ]]",
  "    end",
  "  end,",
  "}",
  "",
].join("\n");

describe("the code view's highlighting", () => {
  it("joins back to the script, and marks the kit's verbs, numbers, keywords, strings and comments", () => {
    const tokens = tokenize(SCRIPT);
    expect(tokens.map((token) => token.text).join("")).toBe(SCRIPT);
    const kinds = (kind: string): string[] => tokens.filter((token) => token.kind === kind).map((token) => token.text);
    expect(kinds("kit")).toEqual(["Card", "count", "damage", "target"]);
    expect(kinds("param")).toEqual(["n.dmg"]);
    expect(kinds("keyword")).toEqual(expect.arrayContaining(["return", "function", "if", "then", "end"]));
    expect(kinds("string")).toEqual(['"an enemy unit"', "'friendly units'"]);
    expect(kinds("number")).toEqual(["2"]);
    expect(kinds("comment")).toEqual(["-- A comment", "--[[ long\ncomment ]]"]);
  });

  it("numbers lines as the script does, a token across a break cut there", () => {
    const lines = tokenLines(SCRIPT);
    expect(lines).toHaveLength(SCRIPT.split("\n").length);
    expect(lines.map((line) => line.map((token) => token.text).join(""))).toEqual(SCRIPT.split("\n"));
  });

  it("an unclosed string or comment still joins back", () => {
    for (const text of ['c:damage("open', "--[[ open", "x = [[ open", "@#$"]) {
      expect(tokenize(text).map((token) => token.text).join("")).toBe(text);
    }
  });
});
