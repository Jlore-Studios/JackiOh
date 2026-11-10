// #3633: no stylesheet follows a `:has()` with a universal compound. `.app-shell--wide:has(> .game) > *`
// and `> :not(.game)` (board.css, the game screen) made Blink invalidate the whole shell, the board
// and its effects included, whenever anything under it changed: each read the effects director made
// next forced a restyle of every element on the page, about 110 ms a time under 4x CPU throttling,
// and the living game's phone budget (R3351000) broke by hundreds of milliseconds a frame. A compound
// with a class, an id, an attribute or a type keeps Blink's invalidation to the elements it names.
// jsdom matches no selector the way Blink invalidates it, so this reads every stylesheet as text.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

/** apps/web/src, every stylesheet the client ships. */
const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

/** Splits `text` at `at` (a character test) outside brackets and quotes, so `:is(a, b)` and `[x="a b"]` stay whole. */
function splitTop(text: string, at: (ch: string) => boolean): string[] {
  const parts: string[] = [];
  let depth = 0;
  let quote: string | null = null;
  let part = "";
  for (const ch of text) {
    if (quote !== null) {
      if (ch === quote) quote = null;
    } else if (ch === '"' || ch === "'") quote = ch;
    else if (ch === "(" || ch === "[") depth += 1;
    else if (ch === ")" || ch === "]") depth -= 1;
    else if (depth === 0 && at(ch)) {
      parts.push(part);
      part = "";
      continue;
    }
    part += ch;
  }
  parts.push(part);
  return parts.map((p) => p.trim()).filter((p) => p !== "");
}

/** Every selector in a sheet: each style rule's prelude, nested ones included, split at its commas. */
function selectorsOf(css: string): string[] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  return [...code.matchAll(/([^{};]+)\{/g)]
    .map((match) => (match[1] ?? "").trim())
    .filter((prelude) => !prelude.startsWith("@") && !/^(from|to|\d[\d.]*%)(\s*,\s*(from|to|\d[\d.]*%))*$/.test(prelude))
    .flatMap((prelude) => splitTop(prelude, (ch) => ch === ","));
}

/** A selector's compounds, its combinators (` `, `>`, `+`, `~`) dropped. */
function compoundsOf(selector: string): string[] {
  return splitTop(selector.replace(/\s*([>+~])\s*/g, " $1 "), (ch) => /\s/.test(ch)).filter((part) => !/^[>+~]$/.test(part));
}

/** A compound's functional pseudo-classes (`:is(…)`, `:not(…)`), each with its argument, and the rest of it. */
function functionalsOf(compound: string): { rest: string; calls: { name: string; arg: string }[] } {
  const calls: { name: string; arg: string }[] = [];
  let rest = "";
  let at = 0;
  for (let open = compound.indexOf("(", at); open !== -1; open = compound.indexOf("(", at)) {
    const head = /:([a-z-]+)$/i.exec(compound.slice(at, open));
    let depth = 0;
    let close = open;
    for (; close < compound.length; close += 1) {
      if (compound[close] === "(") depth += 1;
      if (compound[close] === ")") depth -= 1;
      if (depth === 0) break;
    }
    rest += compound.slice(at, head === null ? open : open - (head[0]?.length ?? 0));
    if (head !== null) calls.push({ name: (head[1] ?? "").toLowerCase(), arg: compound.slice(open + 1, close) });
    at = close + 1;
  }
  return { rest: rest + compound.slice(at), calls };
}

/**
 * A compound that names no class, id, attribute or type an element must have: `*`, `::before`, or
 * only `:not(.x)`, which every other element matches. `:is()` and `:where()` name what each of their
 * arguments names, so `:is(.a, .b)` names its classes and `:is(.a, *)` names nothing.
 */
function universal(compound: string): boolean {
  const { rest, calls } = functionalsOf(compound);
  if (/[.#[]/.test(rest) || /^[a-z]/i.test(rest)) return false;
  return !calls.some(
    ({ name, arg }) =>
      (name === "is" || name === "where") &&
      splitTop(arg, (ch) => ch === ",").every((selector) => {
        const compounds = compoundsOf(selector);
        const last = compounds[compounds.length - 1];
        return last !== undefined && !universal(last);
      }),
  );
}

/** Selectors where a compound holding `:has(` is followed by a combinator and a universal last compound. */
function offenders(css: string): string[] {
  return selectorsOf(css).filter((selector) => {
    const compounds = compoundsOf(selector);
    const last = compounds[compounds.length - 1];
    return last !== undefined && universal(last) && compounds.slice(0, -1).some((compound) => compound.includes(":has("));
  });
}

describe("#3633 the stylesheets' :has() selectors", () => {
  it("#3633 the reader flags a :has() followed by a universal compound and nothing else", () => {
    const sheet = `
      /* .x:has(> .y) > * in a comment is not code */
      .shell:has(> .game) > * { margin: 0; }
      .shell:has(> .game) > :not(.game) { margin: 0; }
      .shell:has(> .game) ::before { content: ""; }
      @media (max-width: 600px) { .shell:has(> .game) > * { margin: 0; } }
      .shell:has(> .game) > .bar, .shell:has(> .game) > div { margin: 0; }
      .shell:has(> .game) > [data-x] { margin: 0; }
      .shell:has(> .game) :is(.a, .b), .shell:has(> .game) > :where(.a):not(.b) { margin: 0; }
      .shell:has(> .game) > :is(.a, *) { margin: 0; }
      :has(> .cf[data-depth]) { perspective: 1px; }
      .board > * { margin: 0; }
      @keyframes spin { from { rotate: 0deg; } 50% { rotate: 1deg; } to { rotate: 2deg; } }
    `;
    expect(offenders(sheet)).toEqual([
      ".shell:has(> .game) > *",
      ".shell:has(> .game) > :not(.game)",
      ".shell:has(> .game) ::before",
      ".shell:has(> .game) > *",
      ".shell:has(> .game) > :is(.a, *)",
    ]);
  });

  it("#3633 no :has() selector is followed by a universal compound", () => {
    const found = cssFiles(SRC).flatMap((path) => offenders(readFileSync(path, "utf8")).map((selector) => `${relative(SRC, path)}: ${selector}`));
    expect(found).toEqual([]);
  });
});
