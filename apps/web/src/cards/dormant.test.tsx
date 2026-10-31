// #2435: a Dormant card on the field is unmistakable — greyed and dimmed past "Can't attack"
// over its whole face and bound by the night-violet seal, under the state rail (DormantSeal.tsx,
// dormant.css, MinionFace.tsx's `data-dormant`).

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { cleanup, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { testid } from "../game/contract.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { renderBoard } from "../test/board.tsx";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";
import { must } from "../test/stubServer.ts";
import { DORMANT_DIM, DORMANT_GREY, DORMANT_PULSE_MS } from "./constants.ts";
import {
  DORMANT_GLYPH,
  DORMANT_NEXT_TURN_WORDS,
  DORMANT_WORDS,
  STARSHIP_GLYPH,
  dormantWords,
  starshipWords,
} from "./cardState.ts";

function readSheet(fromWeb: string): string {
  for (const candidate of [fromWeb, `apps/web/${fromWeb}`]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, " ");
  }
  throw new Error(`${fromWeb} not found from ${process.cwd()}`);
}

const dormantCss = readSheet("src/cards/dormant.css");
const cardstateCss = readSheet("src/cards/cardstate.css");
const keywordsCss = readSheet("src/cards/keywords.css");

/** The declarations of the first `selector` rule in `css`. */
function ruleBody(css: string, selector: string): string {
  const escaped = selector.replace(/[[\]".=()]/g, "\\$&").replace(/\s+/g, "\\s+");
  const found = new RegExp(`${escaped}\\s*\\{([^}]*)\\}`).exec(css)?.[1];
  if (found === undefined) throw new Error(`${selector} not found`);
  return found;
}

/** The value of `name` in a rule `body`. */
function declOf(body: string, name: string): string | undefined {
  return new RegExp(`${name}\\s*:\\s*([^;]+);`).exec(body)?.[1]?.trim().replace(/\s+/g, " ");
}

function faceOf(id: string): HTMLElement {
  return must(screen.getByTestId(testid.card(id)).querySelector<HTMLElement>(".cf"), `a face inside ${id}`);
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  setReducedMotion(false);
  window.localStorage.clear();
  __resetSettingsForTests();
});

function dormantBoard() {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { instanceId: "u1", defId: "core-008", slumber: { turns: 2 }, canAct: false }),
        unit("p1", { instanceId: "u2", defId: "core-008", slumber: {}, canAct: false }),
        unit("p1", { instanceId: "u3", defId: "core-008" }),
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", { hand: { count: 0 } }),
  });
}

describe("#2435 a Dormant card's treatment", () => {
  it("#2435 a Dormant card, with a countdown or none, wears data-dormant and the seal; an awake one wears neither", () => {
    renderBoard(dormantBoard());
    const countdown = faceOf("u1");
    expect(countdown.getAttribute("data-dormant")).toBe("2");
    const countdownSeal = must(countdown.querySelector<HTMLElement>(".cf-dormant"), "u1's seal");
    expect(countdownSeal.parentElement?.classList.contains("cf-scale")).toBe(true);
    expect(countdownSeal.querySelector(".cf-dormant-veil")).not.toBeNull();
    expect(countdownSeal.querySelector(".cf-dormant-bands")).not.toBeNull();

    const waiting = faceOf("u2");
    expect(waiting.getAttribute("data-dormant")).toBe("true");
    expect(waiting.querySelectorAll(".cf-dormant").length).toBe(1);

    const awake = faceOf("u3");
    expect(awake.hasAttribute("data-dormant")).toBe(false);
    expect(awake.querySelector(".cf-dormant")).toBeNull();
  });

  it("#2435 the seal is aria-hidden, adds no text, and the badge keeps its words", () => {
    renderBoard(dormantBoard());
    const face = faceOf("u1");
    const seal = must(face.querySelector<HTMLElement>(".cf-dormant"), "u1's seal");
    expect(seal.getAttribute("aria-hidden")).toBe("true");
    for (const element of seal.querySelectorAll("*")) {
      expect(element.tagName.toLowerCase() === "span" || element.closest('svg[aria-hidden="true"]') !== null).toBe(true);
      expect(["text", "title"].includes(element.tagName.toLowerCase())).toBe(false);
    }
    expect(seal.textContent).toBe("");
    const clone = face.cloneNode(true) as HTMLElement;
    clone.querySelector(".cf-dormant")?.remove();
    expect(face.textContent).toBe(clone.textContent);

    const badge = must(face.querySelector<HTMLElement>('.cf-state[data-state="dormant"]'), "u1's badge");
    expect(badge.textContent).toBe(`${DORMANT_GLYPH}2`);
    expect(badge.getAttribute("aria-label")).toBe(dormantWords(2));
    expect(dormantWords(null)).toBe("Dormant: it can't attack or be attacked and nothing reaches it, until an effect awakens it");
    expect(dormantWords(1)).toBe("Dormant: it awakens at the start of its controller's next turn; until then nothing reaches it");
    expect(dormantWords(3)).toBe("Dormant: it awakens 3 of its controller's turns from now; until then nothing reaches it");
    expect(DORMANT_WORDS).toBe("Dormant: it can't attack or be attacked and nothing reaches it, until an effect awakens it");
    expect(DORMANT_NEXT_TURN_WORDS).toBe("Dormant: it awakens at the start of its controller's next turn; until then nothing reaches it");
  });

  it("#2435 the state rail comes after the seal and paints over it", () => {
    renderBoard(dormantBoard());
    const face = faceOf("u1");
    const seal = must(face.querySelector<HTMLElement>(".cf-dormant"), "u1's seal");
    const rail = must(face.querySelector<HTMLElement>(".cf-states"), "u1's rail");
    expect(rail.closest(".cf-dormant")).toBeNull();
    expect(rail.parentElement).toBe(seal.parentElement);
    expect((seal.compareDocumentPosition(rail) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0).toBe(true);
    const sealZ = Number(declOf(ruleBody(dormantCss, ".cf .cf-dormant"), "z-index"));
    const railZ = Number(declOf(ruleBody(cardstateCss, ".cf .cf-states"), "z-index"));
    expect(sealZ).toBeLessThanOrEqual(railZ);
  });

  it("#2435 the veil greys and dims the whole face, past the Can't attack portrait", () => {
    expect(DORMANT_GREY).toBeGreaterThanOrEqual(0.9);
    expect(DORMANT_GREY).toBeLessThanOrEqual(1);
    expect(DORMANT_DIM).toBeGreaterThanOrEqual(0.5);
    expect(DORMANT_DIM).toBeLessThanOrEqual(0.65);
    const portrait = ruleBody(keywordsCss, '.cf:has(.kw-fx[data-keyword-fx="Can\'t attack"]) .cf-portrait');
    const attacked = /grayscale\(([\d.]+)\)\s*brightness\(([\d.]+)\)/.exec(portrait);
    expect(attacked).not.toBeNull();
    expect(DORMANT_GREY).toBeGreaterThan(Number(attacked?.[1]));
    expect(DORMANT_DIM).toBeLessThan(Number(attacked?.[2]));

    renderBoard(dormantBoard());
    const seal = must(faceOf("u1").querySelector<HTMLElement>(".cf-dormant"), "u1's seal");
    expect(seal.style.getPropertyValue("--cf-dormant-grey")).toBe(String(DORMANT_GREY));
    expect(seal.style.getPropertyValue("--cf-dormant-dim")).toBe(String(DORMANT_DIM));
    const veil = ruleBody(dormantCss, ".cf .cf-dormant-veil");
    expect(declOf(veil, "inset")).toBe("0");
    expect(declOf(veil, "backdrop-filter")).toBe("grayscale(var(--cf-dormant-grey)) brightness(var(--cf-dormant-dim))");
  });

  it("#2435 the runes and the seal loop only while motion is on and hold still under reduced motion", () => {
    renderBoard(dormantBoard());
    const seal = must(faceOf("u1").querySelector<HTMLElement>(".cf-dormant"), "u1's seal");
    const looping = [...seal.querySelectorAll(".cf-dormant-anim")];
    expect(looping.length).toBe(2);
    expect(looping.some((element) => element.classList.contains("cf-dormant-runes"))).toBe(true);
    expect(looping.some((element) => element.classList.contains("cf-dormant-moon"))).toBe(true);
    expect(seal.querySelector(".cf-dormant-ribbons")?.classList.contains("cf-dormant-anim")).toBe(false);
    expect(seal.style.getPropertyValue("--cf-dormant-pulse")).toBe(`${String(DORMANT_PULSE_MS)}ms`);

    const animations = [...dormantCss.matchAll(/animation\s*:\s*([^;]+);/g)].map((match) => (match[1] ?? "").trim());
    const loops = animations.filter((value) => !value.startsWith("none"));
    expect(animations.filter((value) => value.startsWith("none")).length).toBe(2);
    expect(loops.length).toBe(1);
    expect(loops[0]).toContain("cf-dormant-pulse");
    expect(loops[0]).toContain("var(--cf-dormant-pulse)");
    expect(new RegExp("\\.cf\\s+\\.cf-dormant\\s+\\.cf-dormant-anim\\s*\\{[^}]*animation\\s*:").test(dormantCss)).toBe(true);
    const keyframes = /@keyframes\s+cf-dormant-pulse\s*\{([\s\S]*?)\n\}/.exec(dormantCss)?.[1] ?? "";
    expect(keyframes).toContain("from");
    expect(keyframes).toContain("to");
    const names = [...keyframes.matchAll(/([a-z-]+)\s*:/g)].map((match) => match[1]).filter((name) => name !== undefined);
    expect(new Set(names)).toEqual(new Set(["opacity", "transform"]));
  });

  it("#2435 a Dormant Starship is sealed until it launches, its piece count on top", () => {
    const pieces = [{ defId: "core-008" }, { defId: "core-011" }];
    renderBoard(
      baseView({
        you: emptySide("p1", {
          units: [
            unit("p1", { instanceId: "u1", defId: "core-008", slumber: {}, starship: { pieces }, canAct: false }),
            unit("p1", { instanceId: "u2", defId: "core-008" }),
            unit("p1", { instanceId: "u4", defId: "core-008", starship: { pieces, name: "Ember of Departure", launched: 1 } }),
            null,
            null,
          ],
        }),
        opponent: emptySide("p2", { hand: { count: 0 } }),
      }),
    );
    const face = faceOf("u1");
    const seal = must(face.querySelector<HTMLElement>(".cf-dormant"), "the Starship's seal");
    const badge = must(face.querySelector<HTMLElement>('.cf-state[data-state="starship"]'), "the Starship's badge");
    expect(badge.textContent).toBe(`${STARSHIP_GLYPH}2`);
    expect(badge.getAttribute("aria-label")).toBe(starshipWords(2, false));
    expect(badge.closest(".cf-dormant")).toBeNull();
    expect((seal.compareDocumentPosition(badge) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0).toBe(true);

    const launched = faceOf("u4");
    expect(launched.hasAttribute("data-dormant")).toBe(false);
    expect(launched.querySelector(".cf-dormant")).toBeNull();
  });
});
