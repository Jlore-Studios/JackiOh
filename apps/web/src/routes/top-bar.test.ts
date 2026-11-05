// #3189: the top bar's one skin. jsdom ignores the cascade, so these read the sheets and prove their structure:
// one rule gives BackLink's `.screen-nav` and the landing's `.landing-corner` the same controls, and no screen's
// sheet keeps a copy of it. The classes the controls carry are asserted where they render (FriendsButton.test.tsx,
// landing.test.tsx, invite-feedback.test.tsx).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
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
    expect(skin).toContain("background: rgba(18, 11, 7, 0.65);");
    expect(skin).toContain("color: var(--tavern-gold-pale);");
    expect(skin).toContain("min-height: 44px;");
    expect(block(css, `${BAR} .top-bar-button:hover:not(:disabled)`)).toContain("0 0 18px rgba(243, 201, 105, 0.3)");
    const round = block(css, `${BAR} .top-bar-button--round`);
    expect(round).toContain("border-radius: 50%;");
    expect(round).toContain("width: 44px;");
    expect(round).toContain("height: 44px;");
    const text = block(css, `${BAR} .top-bar-button--text`);
    expect(text).toContain("text-decoration: none;");
    // The round shape is written after the skin's 999px radius, which it ties with in weight.
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

  it("#3189 the tavern's wood-button and text-link rules skip the bar's buttons, at no weight", () => {
    const tavern = sheet("auth/tavern.css");
    expect(tavern.match(/button:where\(:not\(\.top-bar-button\)\):not\(\.link-button\)/g)).toHaveLength(2);
    expect(tavern.match(/\.link-button:where\(:not\(\.top-bar-button\)\)/g)).toHaveLength(2);
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
    const together = sheet("together/together.css");
    expect(together).toContain(".screen-nav:has(> .friends-button) .top-bar-button--text");
    expect(together).toContain(".landing .landing-corner:has(> .friends-button) .top-bar-button--text");
  });
});
