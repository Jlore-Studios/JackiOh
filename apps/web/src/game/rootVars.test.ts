// #3633: the game root's custom properties go onto the elements that read them (rootVars.ts). A rule
// reading one where `ROOT_VARS` does not write it would read nothing, so every stylesheet's
// reads are held to the readers here. jsdom computes no custom property, so the sheets are read as text.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

import { ROOT_VARS, ROOT_VARS_ATTRIBUTE, readRootVar, rootVarCss, rootVars } from "./rootVars.ts";

/** apps/web/src, every stylesheet the client ships. */
const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

/** Each innermost block of a sheet with its prelude (a style rule's selector list, or a keyframe's). */
function blocksOf(css: string): { prelude: string; body: string; keyframe: boolean }[] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  const out: { prelude: string; body: string; keyframe: boolean }[] = [];
  const open: { prelude: string; at: number }[] = [];
  let from = 0;
  for (let at = 0; at < code.length; at += 1) {
    if (code[at] === "{") {
      open.push({ prelude: code.slice(from, at).trim(), at: at + 1 });
      from = at + 1;
    } else if (code[at] === "}") {
      const block = open.pop();
      if (block !== undefined && !code.slice(block.at, at).includes("{")) {
        out.push({ prelude: block.prelude.replace(/\s+/g, " "), body: code.slice(block.at, at), keyframe: open.some((outer) => /^@(-webkit-)?keyframes/.test(outer.prelude)) });
      }
      from = at + 1;
    }
  }
  return out;
}

/** A selector list split at its top-level commas. */
function selectorsIn(prelude: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let part = "";
  for (const ch of prelude) {
    if (ch === "(" || ch === "[") depth += 1;
    if (ch === ")" || ch === "]") depth -= 1;
    if (ch === "," && depth === 0) {
      parts.push(part.trim());
      part = "";
    } else part += ch;
  }
  return [...parts, part.trim()].filter((selector) => selector !== "");
}

/** Whether a selector fragment names `reader` (a class or an attribute) as a whole token. */
function names(fragment: string, reader: string): boolean {
  if (reader.startsWith(".")) return new RegExp(`\\${reader}(?![\\w-])`).test(fragment);
  const attribute = reader.endsWith("]") ? reader.slice(0, -1) : reader;
  return fragment.includes(attribute) && (reader.endsWith("]") ? new RegExp(`\\${attribute}(?=[\\]=~|^$*])`).test(fragment) : true);
}

/** A selector's last compound: what it styles, with its pseudo-element. */
function subjectOf(selector: string): string {
  const compounds = selector.replace(/\s*([>+~])\s*/g, " ").split(/\s+(?![^([]*[)\]])/);
  return compounds[compounds.length - 1] ?? "";
}

/** Every read of a root variable that its readers do not reach: `file: selector reads --name`. */
function unreached(css: string, file: string): string[] {
  const misses: string[] = [];
  for (const block of blocksOf(css)) {
    for (const [name, where] of Object.entries(ROOT_VARS)) {
      if (!new RegExp(`var\\(\\s*${name}[\\s,)]`).test(block.body)) continue;
      // A keyframe has no element of its own to hold a reader to.
      if (block.keyframe) {
        misses.push(`${file}: @keyframes reads ${name}`);
        continue;
      }
      for (const selector of selectorsIn(block.prelude)) {
        const subject = subjectOf(selector);
        // An inherited variable reaches a reader's descendants; one that does not inherit, the reader itself
        // or, where it is written there, the root's own `::after` (`.game…::after`).
        const reached = where.inherits
          ? where.readers.some((reader) => names(selector, reader))
          : where.readers.some((reader) => names(subject.split("::")[0] ?? "", reader)) ||
            (where.rootAfter === true && /\.game(?![\w-])/.test(subject) && subject.endsWith("::after"));
        if (!reached) misses.push(`${file}: ${selector} reads ${name}`);
      }
    }
  }
  return misses;
}

describe("#3633 the game root's variables", () => {
  afterEach(() => {
    document.head.querySelectorAll(`style[${ROOT_VARS_ATTRIBUTE}]`).forEach((style) => style.remove());
    document.body.replaceChildren();
  });

  it("#3633 writes each variable onto its readers, scoped to its root, and some onto the root's ::after", () => {
    expect(rootVarCss("g7", "--anim-squeeze", "0.800")).toBe(`[data-root-vars="g7"] :is(${(ROOT_VARS["--anim-squeeze"]?.readers ?? []).join(", ")}) { --anim-squeeze: 0.800; }`);
    expect(rootVarCss("g7", "--set-piece-hold", "1000ms")).toBe(
      ['[data-root-vars="g7"] :is([data-animating], .card, .zone, .field) { --set-piece-hold: 1000ms; }', '[data-root-vars="g7"]::after { --set-piece-hold: 1000ms; }'].join("\n"),
    );
    expect(() => rootVarCss("g7", "--nope", "1")).toThrow(/no root variable/);
  });

  it("#3633 the owners of one root share its rules, one sheet a variable, and the last to let go removes them", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    const game = rootVars(root);
    const layer = rootVars(root);
    const id = root.getAttribute(ROOT_VARS_ATTRIBUTE);
    expect(id).not.toBeNull();
    const sheetOf = (name: string): Element | null => document.head.querySelector(`style[${ROOT_VARS_ATTRIBUTE}="${String(id)} ${name}"]`);
    game.set("--set-piece-hold", "1000ms");
    layer.set("--anim-squeeze", "0.500");
    const hold = sheetOf("--set-piece-hold");
    const squeeze = sheetOf("--anim-squeeze");
    expect(hold?.textContent).toContain("--set-piece-hold: 1000ms;");
    expect(squeeze?.textContent).toContain("--anim-squeeze: 0.500;");
    expect(hold?.textContent).not.toContain("--anim-squeeze");
    expect(readRootVar(root, "--anim-squeeze")).toBe("0.500");
    // Nothing on the root's own style, which restyled the whole game.
    expect(root.getAttribute("style")).toBeNull();
    // A value set again writes nothing; unset empties its sheet and leaves the others.
    layer.set("--anim-squeeze", "");
    expect(readRootVar(root, "--anim-squeeze")).toBe("");
    expect(squeeze?.textContent).toBe("");
    expect(hold?.textContent).toContain("--set-piece-hold: 1000ms;");
    game.release();
    expect(hold?.isConnected).toBe(true);
    layer.release();
    layer.release();
    expect(hold?.isConnected).toBe(false);
    expect(squeeze?.isConnected).toBe(false);
    expect(root.hasAttribute(ROOT_VARS_ATTRIBUTE)).toBe(false);
    expect(readRootVar(root, "--set-piece-hold")).toBe("");
  });

  it("#3633 the reader check flags a read its readers do not reach, and nothing else", () => {
    const sheet = `
      .game[data-set-piece] [data-animating] { animation-duration: var(--set-piece-hold, 0ms); }
      .game[data-slam-tier="massive"]::after { animation-delay: var(--slam-anticipation); }
      [data-animating="x"] .tag { animation-duration: calc(1ms * var(--anim-squeeze, 1)); }
      .game[data-takeover] .board .hero { animation-delay: var(--takeover-land-ms); }
      .zone .tag { animation-delay: var(--set-piece-hold); }
      .tag { animation-duration: calc(1ms * var(--anim-squeeze, 1)); }
      @keyframes spin { to { rotate: var(--deal-ms); } }
    `;
    expect(unreached(sheet, "x.css")).toEqual([
      "x.css: .game[data-takeover] .board .hero reads --takeover-land-ms",
      "x.css: .zone .tag reads --set-piece-hold",
      "x.css: .tag reads --anim-squeeze",
      "x.css: @keyframes reads --deal-ms",
    ]);
  });

  it("#3633 every stylesheet reads a root variable where its readers get it", () => {
    const misses = cssFiles(SRC).flatMap((path) => unreached(readFileSync(path, "utf8"), relative(SRC, path)));
    expect(misses).toEqual([]);
  });
});
