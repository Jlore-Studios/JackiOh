// No source of the dashboard renders raw HTML (apps/web/src/auth/no-raw-html.test.ts is the pattern): a
// news title, a summary, a username or an audit entry is React text, never markup. Sources are read as
// `?raw`; the patterns are built from pieces so this file does not match itself.

import { describe, expect, it } from "vitest";

const SOURCES = import.meta.glob(["./**/*.{ts,tsx}", "../worker/**/*.ts", "../*.ts"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** React's escape hatch for raw markup, by name. */
const RAW_PROP = new RegExp(["dangerously", "Set", "Inner", "HTML"].join(""));

/** innerHTML or outerHTML at all, read or written: the dashboard has no reason to touch either. */
const INNER = new RegExp(`\\b(?:${["inner", "HTML"].join("")}|${["outer", "HTML"].join("")})\\b`);

/** Markup written straight into the document. */
const WRITES = new RegExp(`\\b(?:${["insertAdjacent", "HTML"].join("")}|document\\.${"write"}(?:ln)?\\s*\\()`);

/** This file: its own patterns are data. */
const SELF = /\/no-raw-html\.test\.ts$/u;

const scanned = (): [string, string][] => Object.entries(SOURCES).filter(([path]) => !SELF.test(path));

describe("no raw HTML in the dashboard", () => {
  it("scans every source of the page, the Worker and the build", () => {
    const paths = Object.keys(SOURCES);
    for (const expected of ["App.tsx", "News.tsx", "Audit.tsx", "Leaderboard.tsx", "SignIn.tsx", "worker/index.ts", "headers.ts"]) {
      expect(
        paths.some((path) => path.endsWith(`/${expected}`)),
        `${expected} is scanned`,
      ).toBe(true);
    }
  });

  it("the patterns catch what they are meant to catch", () => {
    const prop = ["dangerously", "SetInnerHTML"].join("");
    const inner = ["inner", "HTML"].join("");
    expect(RAW_PROP.test(`<div ${prop}={{ __html: x }} />`)).toBe(true);
    expect(INNER.test(`node.${inner} = text;`)).toBe(true);
    expect(INNER.test(`expect(node.${inner}).toBe("")`)).toBe(true);
    expect(WRITES.test(`node.${["insertAdjacent", "HTML"].join("")}("beforeend", x)`)).toBe(true);
    expect(WRITES.test(`document.${"write"}(x)`)).toBe(true);
    expect(INNER.test("textContent")).toBe(false);
  });

  it("no source uses React's raw-markup prop, innerHTML, outerHTML or a document write", () => {
    const offenders = scanned()
      .filter(([, source]) => RAW_PROP.test(source) || INNER.test(source) || WRITES.test(source))
      .map(([path]) => path);
    expect(offenders).toEqual([]);
  });
});
