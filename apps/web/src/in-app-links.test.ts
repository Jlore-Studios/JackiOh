// #2562: every link to a screen of the app moves in place. An `<a>` in a non-test `.tsx` under
// `apps/web/src` whose `href` is a `paths.` entry or an address starting with `/` carries
// `onClick={followInApp(…)}` (`routes/nav.tsx`) or opens a new tab: a bare one reloads the site, and
// the read cache, the engine's WebAssembly and the audio unlock go with it. An `href` passed in a
// variable is its caller's to pair; FULL_LOADS lists the links that load the page on purpose.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** Every non-test `.tsx` under `dir`, `test/` directories aside, as writing.test.ts walks them. */
function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : sources(path);
    return name.endsWith(".tsx") && !name.endsWith(".test.tsx") ? [path] : [];
  });
}

/** One `<a` opening tag. */
type Anchor = {
  /** Its file, relative to `apps/web/src`. */
  file: string;
  /** `file:line` of its `<a`. */
  at: string;
  /** Its `data-testid` as written (`shellTestid.loadFailedHome`), "" without one. */
  testid: string;
  /** Its `href` is a `paths.` entry or an address starting with `/`. */
  internal: boolean;
  /** Its `onClick` calls `followInApp`. */
  follows: boolean;
  /** It opens a new tab (`target="_blank"`). */
  newTab: boolean;
  /** Where its `onClick` attribute starts and ends in the file's text, null without one. */
  onClick: { start: number; end: number } | null;
};

/** An `href` inside the app: `paths.…`, or a string or template starting with `/` or with `${paths.…}`. */
const INTERNAL_HREF = /^(?:paths\.|\/|["'`]\/|`\$\{\s*paths\.)/u;

/** An attribute's value as written: a string's text, a `{…}` expression's source, "" for none. */
function valueOf(attribute: ts.JsxAttribute | undefined, source: ts.SourceFile): string {
  const value = attribute?.initializer;
  if (value === undefined) return "";
  if (ts.isStringLiteral(value)) return value.text;
  if (ts.isJsxExpression(value)) return value.expression?.getText(source) ?? "";
  return value.getText(source);
}

/** Every `<a` opening tag in one file's text, however many lines it spans. */
function anchorsIn(file: string, text: string): Anchor[] {
  const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const out: Anchor[] = [];
  const visit = (node: ts.Node): void => {
    if ((ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) && node.tagName.getText(source) === "a") {
      const attributes = new Map<string, ts.JsxAttribute>();
      for (const property of node.attributes.properties) {
        if (ts.isJsxAttribute(property)) attributes.set(property.name.getText(source), property);
      }
      const onClick = attributes.get("onClick");
      const line = source.getLineAndCharacterOfPosition(node.getStart(source)).line + 1;
      out.push({
        file,
        at: `${file}:${String(line)}`,
        testid: valueOf(attributes.get("data-testid"), source),
        internal: INTERNAL_HREF.test(valueOf(attributes.get("href"), source)),
        follows: valueOf(onClick, source).includes("followInApp("),
        newTab: valueOf(attributes.get("target"), source) === "_blank",
        onClick: onClick === undefined ? null : { start: onClick.getStart(source), end: onClick.getEnd() },
      });
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return out;
}

/** The internal links that load the page on purpose, each with its reason. */
const FULL_LOADS: readonly { file: string; testid: string; why: string }[] = [
  {
    file: "main.tsx",
    testid: "shellTestid.loadFailedHome",
    why: "a screen's code failed to load: Home is a real page load, so it fetches the current build (RouteErrorBoundary)",
  },
  {
    file: "routes/landing.tsx",
    testid: "landingTestid.hotseat",
    why: "the dev-only hotseat boots from a full page load's query",
  },
];

function exempt(anchor: Anchor): boolean {
  return FULL_LOADS.some(({ file, testid }) => anchor.file === file && anchor.testid === testid);
}

/** Where an internal link would reload the site. */
function strays(anchors: readonly Anchor[]): string[] {
  return anchors
    .filter((anchor) => anchor.internal && !anchor.follows && !anchor.newTab && !exempt(anchor))
    .map((anchor) => anchor.at);
}

const TREE = sources(SRC).map((path) => ({ file: relative(SRC, path), text: readFileSync(path, "utf8") }));
const TEXTS = new Map(TREE.map(({ file, text }) => [file, text] as const));
const ANCHORS: readonly Anchor[] = TREE.flatMap(({ file, text }) => anchorsIn(file, text));

describe("#2562 every link to a screen of the app moves in place", () => {
  it("every internal link calls followInApp, opens a new tab or is a listed full load", () => {
    expect(ANCHORS.filter((a) => a.internal && a.follows).length).toBeGreaterThan(50);
    expect(strays(ANCHORS)).toEqual([]);
  });

  it("each listed full load is still there, an internal link that does not call followInApp", () => {
    for (const { file, testid, why } of FULL_LOADS) {
      const found = ANCHORS.filter((a) => a.file === file && a.testid === testid);
      expect(found, why).toHaveLength(1);
      expect(found[0]?.internal, why).toBe(true);
      expect(found[0]?.follows, why).toBe(false);
    }
  });

  it("taking the onClick off any one internal link that calls followInApp fails the scan at that link", () => {
    const followed = ANCHORS.filter((a) => a.internal && a.follows && !a.newTab);
    expect(followed.length).toBeGreaterThan(50);
    for (const anchor of followed) {
      const text = TEXTS.get(anchor.file) ?? "";
      const click = anchor.onClick;
      if (click === null) throw new Error(`${anchor.at} calls followInApp with no onClick`);
      const without = text.slice(0, click.start) + text.slice(click.end);
      expect(strays(anchorsIn(anchor.file, without)), anchor.at).toContain(anchor.at);
    }
  });

  it("reads an opening tag across lines, and leaves an external link, a new tab and a variable href alone", () => {
    const sample = [
      "const bare = <a href={paths.decks}>Decks</a>;",
      "const literal = (",
      "  <a",
      '    className="link"',
      '    href="/terms"',
      "  >",
      "    Terms",
      "  </a>",
      ");",
      "const template = <a href={`/stats?tab=cards`}>Stats</a>;",
      "const followed = <a href={paths.decks} onClick={followInApp(paths.decks)}>Decks</a>;",
      'const tab = <a href={paths.terms} target="_blank" rel="noreferrer">Terms</a>;',
      "const contact = <a href={CONTACT_URL}>Contact</a>;",
      'const away = <a href="https://example.com">Away</a>;',
      "const passed = <a href={action.to}>Go</a>;",
    ].join("\n");
    expect(strays(anchorsIn("sample.tsx", sample))).toEqual(["sample.tsx:1", "sample.tsx:3", "sample.tsx:10"]);
  });
});
