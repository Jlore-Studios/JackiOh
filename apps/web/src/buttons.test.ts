// #3219: the buttons' four roles (buttons.css), each drawn the same on every screen, and no other
// sheet restating one's skin.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { readSheet } from "./test/setup.ts";

const SRC = dirname(fileURLToPath(import.meta.url));

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

const SHEETS = cssFiles(SRC).map((path) => relative(SRC, path).replaceAll("\\", "/")).sort();
const read = (sheet: string): string => readFileSync(join(SRC, sheet), "utf8");
/** A sheet without its comments. */
const code = (sheet: string): string => read(sheet).replace(/\/\*[\s\S]*?\*\//g, " ");

const ROLES = ["button-primary", "button-secondary", "button-danger", "link-button"];
/** The one-offs #3219 pointed at a role: none keeps a skin of its own. */
const ONE_OFFS = [
  "control-end-turn",
  "concede-dialog__stay",
  "concede-dialog__concede",
  "draw-offer__accept",
  "prompt-submit",
  "practice-play",
  "practice-result__secondary",
  "account-delete__confirm",
  "ws-history-button",
  "ws-cosmetics-button",
  "ws-action",
  "db-chip",
  "sandbox-panel__button",
  "power-button",
];
/** A declaration of a background or a border's color, with its value. */
const SKIN = /(?<![\w-])(?:background(?:-color|-image)?|border(?:-(?:top|right|bottom|left))?(?:-color)?)\s*:\s*([^;}]+)/g;
const INK = /(?<![\w-])color\s*:\s*([^;}]+)/g;
/** A value that draws nothing of its own. */
const PLAIN = /^(?:none|0|inherit|initial|unset|transparent|currentcolor)$/i;

/** `text` split at the `separators` that are outside parentheses. */
function splitTop(text: string, separators: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let part = "";
  for (const char of text) {
    if (char === "(") depth += 1;
    if (char === ")") depth -= 1;
    if (depth === 0 && separators.includes(char)) {
      parts.push(part);
      part = "";
    } else part += char;
  }
  parts.push(part);
  return parts.map((p) => p.trim()).filter((p) => p !== "");
}

/** A selector's subject, its last compound, with its :not() arguments cut. */
function subject(selector: string): string {
  let kept = "";
  let depth = 0;
  for (let at = 0; at < selector.length; at += 1) {
    if (depth === 0 && selector.startsWith(":not(", at)) {
      depth = 1;
      at += 4;
      continue;
    }
    if (depth > 0) {
      if (selector[at] === "(") depth += 1;
      if (selector[at] === ")") depth -= 1;
      continue;
    }
    kept += selector[at];
  }
  return splitTop(kept, " >+~").at(-1) ?? "";
}

const names = (compound: string, name: string): boolean => new RegExp(String.raw`\.${name}(?![\w-])`).test(compound);
const role = (compound: string): boolean => ROLES.some((name) => names(compound, name));
const oneOff = (compound: string): boolean =>
  ONE_OFFS.some((name) => names(compound, name)) || compound.includes('[data-testid="prompt-submit"]');
/** A compound that draws a button: the element, a class named for one, or a one-off. */
const buttonish = (compound: string): boolean =>
  /(?<![\w.#-])button(?![\w-])/.test(compound) || /\.(?:[\w-]*[-_])?(?:button|btn)(?:[-_][\w-]*)?(?![\w-])/.test(compound) || oneOff(compound);

/** Each innermost rule of `sheet`: its selectors' subjects and its body. */
function rules(sheet: string): [string[], string][] {
  return [...code(sheet).matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => [
    splitTop(match[1] ?? "", ",")
      .filter((selector) => !selector.includes("::") && !selector.startsWith("@"))
      .map(subject),
    match[2] ?? "",
  ]);
}

/** How many of `body`'s declarations that `pattern` finds draw something. */
const stated = (body: string, pattern: RegExp): number => [...body.matchAll(pattern)].filter((d) => !PLAIN.test((d[1] ?? "").trim())).length;

/** The button skins outside buttons.css that are no role's: each sheet's count, frozen, so one only falls. */
const UNROLED: Record<string, number> = {
  // Google's and Roblox's own sign-in buttons, drawn to their brands' rules.
  "auth/auth.css": 7,
  // The collection's strip and tiles: their pressed, equipped and hovered states (the tabs/segmented issue),
  // and a variant ladder's next rung, its dashed teal edge.
  "collection/collection.css": 12,
  // The dev bar's owed and pressed toggles.
  "duos/duos.css": 2,
  // An open modifier badge's edge (`button.modifier-badge[data-open]`).
  "game/board.css": 1,
  // The pressed filter chips (the tabs/segmented issue) and `.db-actions`' green, which no component renders now.
  "game/deckbuilder/deckbuilder.css": 4,
  // A compared deck's chip, its press taking the deck out of the comparison.
  "game/deckbuilder/workshop.css": 2,
  // A reward's pick, a tile more than a button.
  "game/prompt.css": 2,
  // The dev panel's own button.
  "routes/dev-panel.css": 4,
  // The landing's quests and news panels' wood, for their buttons that name no role.
  "routes/landing.css": 2,
  // The stats page's retry and its pager (the tabs/segmented issue moved toggles to tabs.css).
  "routes/stats.css": 5,
  // The top bar's buttons (#3189), which draw themselves.
  "routes/top-bar.css": 4,
  // A lesson's unlocked edge and the field guide's clear button.
  "tutorial/tutorial.css": 2,
};

describe("#3219 the button roles", () => {
  it("#3219 index.css imports buttons.css seventh, after the token sheets, and only buttons.css declares the role tokens", () => {
    expect(read("index.css")).toMatch(/^@import "\.\/fonts\.css";\n(?:@import "[^"]+";\n){5}@import "\.\/buttons\.css";\n/);
    for (const token of ["--primary-face", "--danger"]) {
      const declaring = SHEETS.filter((sheet) => new RegExp(String.raw`${token}\s*:`).test(code(sheet)));
      expect(declaring, token).toEqual(["buttons.css"]);
    }
  });

  describe("#3219 on every screen", () => {
    const styles: HTMLStyleElement[] = [];
    beforeAll(() => {
      // The page's order: palette.css and buttons.css come in through index.css's imports, then
      // main.tsx loads tavern.css. The imports themselves are cut: each sheet is laid in on its own.
      for (const sheet of ["src/palette.css", "src/buttons.css", "src/index.css", "src/auth/tavern.css"]) {
        const style = document.createElement("style");
        style.textContent = readSheet(sheet).replace(/^@import [^;]+;$/gm, "");
        document.head.append(style);
        styles.push(style);
      }
    });
    afterAll(() => {
      for (const style of styles) style.remove();
    });

    const SKINS: Record<string, [string, string]> = {
      "button-primary": ["var(--primary-face)", "var(--primary-ink)"],
      "button-secondary": ["var(--bg-raised)", "var(--text)"],
      "button-danger": ["color-mix(in srgb, var(--danger) 22%, var(--bg-raised))", "var(--text)"],
      "link-button": ["none", "var(--pal-gold-400)"],
    };
    const HOSTS = ["app-shell tavern", "app-shell tavern deckbuilder", "app-shell", "board"];

    it("#3219 each role draws the same background and color inside and outside .tavern", () => {
      const drawn: string[] = [];
      const expected: string[] = [];
      for (const [name, [background, color]] of Object.entries(SKINS)) {
        for (const tag of ["button", "a"] as const) {
          for (const classes of HOSTS) {
            const host = document.createElement("div");
            host.className = classes;
            const control = document.createElement(tag);
            control.className = name;
            if (tag === "a") control.setAttribute("href", "/");
            else control.setAttribute("type", "button");
            control.textContent = name;
            host.append(control);
            document.body.append(host);
            const style = getComputedStyle(control);
            drawn.push(`${name} <${tag}> in "${classes}": ${style.getPropertyValue("background")} / ${style.getPropertyValue("color")}`);
            expected.push(`${name} <${tag}> in "${classes}": ${background} / ${color}`);
            host.remove();
          }
        }
      }
      expect(drawn).toEqual(expected);
    });
  });

  it("#3219 no other sheet restates a role's skin, nor that of a one-off it replaced", () => {
    const found: string[] = [];
    for (const sheet of SHEETS) {
      if (sheet === "buttons.css") continue;
      for (const [subjects, body] of rules(sheet)) {
        for (const compound of subjects) {
          if (compound.includes("[aria-pressed")) continue;
          const named = role(compound);
          if (!named && !oneOff(compound)) continue;
          if (stated(body, SKIN) > 0 || (named && stated(body, INK) > 0)) found.push(`${sheet}: ${compound}`);
        }
      }
    }
    expect(found, "name the role (buttons.css) and keep only the layout").toEqual([]);
  });

  it("#3219 no stylesheet but buttons.css names --primary-face", () => {
    expect(SHEETS.filter((sheet) => code(sheet).includes("--primary-face"))).toEqual(["buttons.css"]);
  });

  it("#3219 every other button skin is a frozen exception, never a new one", () => {
    const counts: Record<string, number> = {};
    for (const sheet of SHEETS) {
      if (sheet === "buttons.css") continue;
      let own = 0;
      for (const [subjects, body] of rules(sheet)) if (subjects.some(buttonish)) own += stated(body, SKIN);
      if (own > 0) counts[sheet] = own;
    }
    expect(counts, "draw it with a role (buttons.css); a sheet that loses some lowers its number in UNROLED").toEqual(UNROLED);
  });
});
