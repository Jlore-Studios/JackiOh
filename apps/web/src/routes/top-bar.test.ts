// #3189: the top bar's one skin. jsdom ignores the cascade, so these read the sheets and prove their structure:
// one rule gives BackLink's `.screen-nav` and the landing's `.landing-corner` the same controls, and no screen's
// sheet keeps a copy of it. The classes the controls carry are asserted where they render (FriendsButton.test.tsx,
// landing.test.tsx, invite-feedback.test.tsx).
// #3218 (R3218000): the bar's one layout, in the same sheet; the DOM order is nav.test.tsx's, the drawn geometry
// e2e spec 46's.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

/** A sheet of src/, its comments cut out, so the words a comment uses to say what it replaced count for nothing. */
function sheet(file: string): string {
  return readFileSync(join(SRC, file), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

/** The declarations of the first rule that opens with `selector`. */
function block(css: string, selector: string): string {
  const start = css.indexOf(`${selector} {`);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  return css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
}

const BAR = ":is(.screen-nav, .landing-corner)";

describe("#3189 the top bar's one skin", () => {
  it("#3189 one shared rule gives both bars' controls their skin, their hover, and their round and text shapes", () => {
    const css = sheet("routes/top-bar.css");
    const skin = block(css, `${BAR} .top-bar-button`);
    expect(skin).toContain("border: 1px solid var(--tavern-brass);");
    expect(skin).toContain("background: color-mix(in srgb, var(--pal-wood-900) 65%, transparent);");
    expect(skin).toContain("color: var(--tavern-gold-pale);");
    expect(skin).toContain("min-height: var(--hit-min);");
    expect(block(css, `${BAR} .top-bar-button:hover:not(:disabled)`)).toContain(
      "0 0 18px color-mix(in srgb, var(--tavern-gold) 30%, transparent)",
    );
    const round = block(css, `${BAR} .top-bar-button--round`);
    expect(round).toContain("border-radius: var(--radius-round);");
    expect(round).toContain("width: var(--hit-min);");
    expect(round).toContain("height: var(--hit-min);");
    const text = block(css, `${BAR} .top-bar-button--text`);
    expect(text).toContain("text-decoration: none;");
    // The round shape is written after the skin's pill radius, which it ties with in weight.
    expect(css.indexOf(`${BAR} .top-bar-button--round {`)).toBeGreaterThan(css.indexOf(`${BAR} .top-bar-button {`));
  });

  it("#3189 no per-screen copy of the skin remains", () => {
    const tavern = sheet("auth/tavern.css");
    expect(tavern).not.toContain("settings-gear");
    expect(tavern).not.toMatch(/\.screen-nav\s+\.link-button/);
    expect(sheet("routes/landing.css")).not.toMatch(/\.landing-corner\s+\.settings-gear/);
    expect(sheet("routes/landing.css")).not.toContain("landing-corner-link");
    const together = sheet("together/together.css");
    expect(together).not.toMatch(/button\.friends-button\.friends-button--nav/);
    expect(together).not.toMatch(/\.landing-corner\s+\.friends-button\b/);
    expect(together).not.toContain("rgba(18, 11, 7, 0.65)");
    const language = sheet("i18n/language.css");
    expect(language).not.toMatch(/language-button--nav:not/);
    expect(language).not.toContain("rgba(18, 11, 7, 0.65)");
    // The skin's one fill is top-bar.css's own; the Jlore pill's (economy/jlore-pill.css) is drawn alike.
    for (const file of ["routes/landing.css", "settings/settings.css", "auth/tavern.css"]) {
      expect(sheet(file), file).not.toContain("rgba(18, 11, 7, 0.65)");
    }
  });

  it("#3189 the tavern's size rules and buttons.css's link-button skip the bar's buttons, at no weight", () => {
    const tavern = sheet("auth/tavern.css");
    expect(tavern.match(/button:where\(:not\(\.top-bar-button\)\):not\(\.link-button\)/g)).toHaveLength(1);
    expect(tavern.match(/\.link-button:where\(:not\(\.top-bar-button\)\)/g)).toHaveLength(1);
    expect(sheet("buttons.css").match(/\.link-button:where\(:not\(\.top-bar-button\)\)/g)).toHaveLength(3);
  });

  it("#3189 the nav carries the brass colors, so a screen that is not .tavern draws them", () => {
    const tavern = sheet("auth/tavern.css");
    const carried = /^\.tavern,\s*\.screen-nav\s*\{([^}]*)\}/m.exec(tavern)?.[1] ?? "";
    for (const token of ["--tavern-brass", "--tavern-gold", "--tavern-gold-pale"]) {
      expect(carried, token).toContain(`${token}:`);
      expect(tavern.match(new RegExp(`${token}:`, "g")), token).toHaveLength(1);
    }
  });

  it("#3189 the phone's tighter padding follows the shared text pill, in the nav and in the corner", () => {
    const css = sheet("routes/top-bar.css");
    expect(css).toContain(".screen-nav:has(.friends-button) .top-bar-button--text");
    expect(css).toContain(".landing .landing-corner:has(> .friends-button) .top-bar-button--text");
  });
});

/** Every stylesheet under src/, as its path from src/ (the walk palette.test.ts makes). */
function cssFiles(dir = SRC): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "node_modules" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [relative(SRC, path)] : [];
  });
}

describe("R3218000 the top bar's one layout", () => {
  it("R3218000 only top-bar.css places the gear, the language button, the Jlore pill and Friends in the bar", () => {
    const placement = /(^|[;\s])(margin(-left|-right|-inline(-start|-end)?)?|order)\s*:/;
    const placed = /\.(settings-gear|language-button|jlore-pill|friends-button)\b/;
    const sheets = cssFiles().filter((file) => file !== "routes/top-bar.css");
    expect(sheets.length).toBeGreaterThan(30);
    for (const file of sheets) {
      const css = sheet(file);
      expect(css, file).not.toContain(".screen-nav__");
      for (const [, selector = "", declarations = ""] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
        if (selector.includes(".screen-nav") && placed.test(selector)) {
          expect(declarations, `${file}: ${selector.trim()}`).not.toMatch(placement);
        }
      }
    }
  });

  it("R3218000 the bar has fixed slots: the line takes the room, the end group is pushed to the edge, a phone moves the line under", () => {
    const css = sheet("routes/top-bar.css");
    expect(block(css, ".screen-nav__line")).toContain("flex: 1 1 0;");
    expect(block(css, ".screen-nav__end")).toContain("margin-left: auto;");
    const phone = css.slice(css.indexOf("@media (max-width: 600px)"));
    expect(block(phone, ".screen-nav__line")).toContain("order: 1;");
    expect(block(phone, ".screen-nav__line")).toContain("flex: 1 1 100%;");
  });

  it("R3218000 the bar's phone breakpoint is the touch rule's, 600 px, and no bar rule is left at 560 px", () => {
    const css = sheet("routes/top-bar.css");
    const queries = [...css.matchAll(/@media\s*([^{]*)\{/g)].map((match) => match[1]?.trim());
    expect(queries.length).toBeGreaterThan(0);
    for (const query of queries) expect(query).toBe("(max-width: 600px)");
    expect(sheet("index.css")).toContain("@media (pointer: coarse), (max-width: 600px)");
    for (const file of ["settings/settings.css", "i18n/language.css", "economy/jlore-pill.css", "together/together.css"]) {
      for (const [, body = ""] of sheet(file).matchAll(/@media \(max-width: 560px\) \{([\s\S]*?)\n\}/g)) {
        expect(body, file).not.toContain(".screen-nav");
      }
    }
  });

  it("R3218000 the skin is on the tokens: no raw radius, duration, hit size or color where one exists", () => {
    const css = sheet("routes/top-bar.css");
    expect(css).not.toMatch(/\b999px\b|border-radius:\s*50%|\b\d+ms\b|\b44px\b|rgba?\(/);
    for (const token of ["--radius-pill", "--radius-round", "--motion-fast", "--ease-standard", "--hit-min"]) {
      expect(css, token).toContain(`var(${token})`);
    }
  });
});
