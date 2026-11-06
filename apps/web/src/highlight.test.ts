// #3260 (follows #2015): a phone highlights nothing but the chat. The page draws no tap flash at any
// pointer; under a coarse pointer it selects no text and opens no long-press callout, while a field
// keeps its text (iOS will not type into one that cannot be selected) and the chat panel gives all
// three back. jsdom matches no media query, so these read the stylesheets as text.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));
const COARSE = "@media (pointer: coarse)";
const CHAT_SHEET = "together/together.css";

type Rule = { selector: string; body: string };

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

/** A stylesheet's code (path from src/), its comments gone. */
function code(sheet: string): string {
  return readFileSync(join(SRC, sheet), "utf8").replace(/\/\*[\s\S]*?\*\//g, " ");
}

/** Where the block opened at `open` closes, nested blocks counted. */
function closeOf(css: string, open: number): number {
  let depth = 0;
  for (let at = open; at < css.length; at += 1) {
    if (css[at] === "{") depth += 1;
    if (css[at] === "}") depth -= 1;
    if (depth === 0) return at;
  }
  return css.length;
}

/** The bodies of the `@media` blocks whose query is exactly `query`, and the sheet with every `@media` block cut out. */
function split(css: string, query: string): { inside: string[]; outside: string } {
  const inside: string[] = [];
  let outside = "";
  let from = 0;
  for (let at = css.indexOf("@media", from); at !== -1; at = css.indexOf("@media", from)) {
    const open = css.indexOf("{", at);
    if (open === -1) break;
    const close = closeOf(css, open);
    outside += css.slice(from, at);
    if (css.slice(at, open).trim() === query) inside.push(css.slice(open + 1, close));
    from = close + 1;
  }
  return { inside, outside: outside + css.slice(from) };
}

/** The flat rules of a piece of CSS, selectors with their whitespace collapsed. */
function rulesIn(css: string): Rule[] {
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
    selector: (match[1] ?? "").replace(/\s+/g, " ").trim(),
    body: match[2] ?? "",
  }));
}

/** The declarations of every rule whose selector is exactly `selector`, joined. */
function bodyOf(rules: Rule[], selector: string): string {
  return rules
    .filter((rule) => rule.selector === selector)
    .map((rule) => rule.body)
    .join("\n");
}

const UNPREFIXED_SELECT = (value: string): RegExp => new RegExp(`(?<![\\w-])user-select:\\s*${value};`);

/** index.css cut into what holds at every pointer and what holds only under a coarse one. */
function indexSheet(): { inside: string[]; pageRules: Rule[]; coarseRules: Rule[] } {
  const index = split(code("index.css"), COARSE);
  return { inside: index.inside, pageRules: rulesIn(index.outside), coarseRules: index.inside.flatMap(rulesIn) };
}

describe("#3260 a phone highlights nothing but the chat", () => {
  it("#3260 html takes no tap highlight, at every pointer", () => {
    const { pageRules } = indexSheet();
    expect(bodyOf(pageRules, "html")).toMatch(/-webkit-tap-highlight-color:\s*transparent;/);
  });

  it("#3260 html selects as ever at a fine pointer", () => {
    // A narrow desktop window still selects text: selection and the callout wait for a coarse pointer.
    const { inside, pageRules } = indexSheet();
    expect(inside.length).toBeGreaterThan(0);
    const html = bodyOf(pageRules, "html");
    expect(html).not.toMatch(/user-select/);
    expect(html).not.toMatch(/touch-callout/);
  });

  it("#3260 under a coarse pointer html takes no selection and no callout", () => {
    const { coarseRules } = indexSheet();
    const html = bodyOf(coarseRules, "html");
    expect(html).toMatch(/-webkit-touch-callout:\s*none;/);
    expect(html).toMatch(/-webkit-user-select:\s*none;/);
    expect(html).toMatch(UNPREFIXED_SELECT("none"));
  });

  it("#3260 under a coarse pointer an input, a textarea and a contenteditable take their text back", () => {
    const { coarseRules } = indexSheet();
    const fields = coarseRules.filter((rule) => UNPREFIXED_SELECT("text").test(rule.body));
    expect(fields.length).toBeGreaterThan(0);
    const selectors = fields.map((rule) => rule.selector).join(", ");
    expect(selectors).toContain("input");
    expect(selectors).toContain("textarea");
    expect(selectors).toContain("[contenteditable");
    expect(fields.map((rule) => rule.body).join("\n")).toMatch(/-webkit-user-select:\s*text;/);
  });

  it("#3260 .chat-panel gives the tap highlight, the callout and the selection back", () => {
    // Outside any `@media`, so a message can be pressed, selected and copied at every pointer.
    const chat = bodyOf(rulesIn(split(code(CHAT_SHEET), COARSE).outside), ".chat-panel");
    expect(chat).toMatch(/-webkit-tap-highlight-color:\s*initial;/);
    expect(chat).toMatch(/-webkit-touch-callout:\s*default;/);
    expect(chat).toMatch(/-webkit-user-select:\s*text;/);
    expect(chat).toMatch(UNPREFIXED_SELECT("text"));
  });

  it("#3260 no stylesheet but the chat's sets the tap highlight to anything but transparent", () => {
    // `-webkit-tap-highlight-color` is inherited, so one `initial` on a zone's children (#2015) gave the
    // flash back on every card in it.
    const found: string[] = [];
    for (const path of cssFiles(SRC)) {
      const rel = relative(SRC, path).replaceAll("\\", "/");
      for (const match of code(rel).matchAll(/-webkit-tap-highlight-color:\s*([^;}]+)/g)) {
        found.push(`${rel}: ${(match[1] ?? "").trim()}`);
      }
    }
    expect(found.filter((entry) => !/: (transparent|inherit)$/.test(entry))).toEqual([`${CHAT_SHEET}: initial`]);
    expect(code("game/board.css")).not.toMatch(/tap-highlight-color:\s*initial/);
  });

  it("#3260 the two-step key stays copyable under a finger: it has no copy button", () => {
    const key = bodyOf(rulesIn(code("auth/two-step.css")), ".tavern.app-shell .account-two-step__secret");
    expect(key).toMatch(/-webkit-user-select:\s*all;/);
    expect(key).toMatch(UNPREFIXED_SELECT("all"));
  });
});
