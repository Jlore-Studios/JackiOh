// #3674 (R3351000, R3350000): a board Mythic's diorama slices, lifted (lift.ts). A slice whose motion is one chain of loops
// around all it draws is drawn as its still picture, byte for byte the slice less its motion style, on layers that artLive.css
// moves as the picture's own style did, a picture unit a percent; every other slice draws as before. The faces that draw
// them are diorama.test.tsx's.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { artMotionOf } from "../../fullArt.ts";
import { fmt } from "../hash.ts";
import { ART_MOVES, CENTER_ORIGIN, FIGURE_SUBJECT_ORIGIN, roleRule, type ArtMove } from "../motion.ts";
import { DIORAMAS, dioramaSliceSvg, dioramaSliceUri } from "./index.ts";
import { LIFTABLE_MOVES, MOTION_STYLE, liftedSlice, liftSvg, sliceWindowUri } from "./lift.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const DATA_PREFIX = "data:image/svg+xml,";

/** A picture's head as the painters write it: 100 units square, its definitions. */
const HEAD = `<svg viewBox="0 0 100 100" width="100" height="100" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="g"/></defs>`;
/** A style inside the reduced-motion gate, as motion.ts and index.ts write one. */
const gate = (rules: string): string => `<style>@media (prefers-reduced-motion: no-preference){${rules}}</style>`;
const frames = (move: ArtMove): string => `@keyframes art-${move}{${ART_MOVES[move].keyframes}}`;
const loop = (cls: string, move: ArtMove, seconds: number, origin = CENTER_ORIGIN): string => roleRule(`.${cls}`, { move, seconds }, origin) + frames(move);

const decoded = (uri: string): string => decodeURIComponent(uri.slice(DATA_PREFIX.length));

/** A length as a lift writes it: zero bare, else a percent of the layer. */
const percent = (units: number): string => (units === 0 ? "0" : `${fmt(units)}%`);

/**
 * A move's keyframes (`ART_MOVES`) as a lift's: each `transform` the transform properties it equals, a scale before a
 * translation scaling the translation (`translate` applies after `scale`), and each picture unit (px) a percent.
 */
function asLift(keyframes: string): string {
  return keyframes.replace(/transform:([^;}]*)/g, (_match, list: string) => {
    let scale: number | null = null;
    let rotate: string | null = null;
    let moved: [number, number] | null = null;
    for (const [, fn = "", arg = ""] of list.matchAll(/(\w+)\(([^)]*)\)/g)) {
      const units = Number(arg.replace(/px$/, "")) * (scale ?? 1);
      if (fn === "scale") scale = Number(arg);
      else if (fn === "rotate") rotate = arg;
      else if (fn === "translateX") moved = [units, moved?.[1] ?? 0];
      else if (fn === "translateY") moved = [moved?.[0] ?? 0, units];
      else throw new Error(`no lift writes ${fn}`);
    }
    return [
      moved === null ? null : `translate:${percent(moved[0])} ${percent(moved[1])}`,
      rotate === null ? null : `rotate:${rotate}`,
      scale === null ? null : `scale:${fmt(scale)}`,
    ]
      .filter((decl) => decl !== null)
      .join(";");
  });
}

describe("R3351000 lifting a diorama's slice", () => {
  it("R3351000 a slice wrapped in one loop lifts it: the still picture is the slice less its motion style", () => {
    const body = `<g class="dio"><path d="M0 0h10v10z"/><g/><circle r="2"/></g>`;
    const svg = `${HEAD}${gate(loop("dio", "bob", 4, "50px 60px"))}${body}</svg>`;
    expect(liftSvg(svg)).toEqual({ svg: `${HEAD}${body}</svg>`, moves: [{ move: "bob", seconds: 4, origin: [50, 60] }] });
    // A move about a piece's own box that ignores its origin is lifted with none.
    expect(liftSvg(`${HEAD}${gate(loop("dio", "pulse", 3.5))}${body}</svg>`)?.moves).toEqual([{ move: "pulse", seconds: 3.5, origin: null }]);
  });

  it("R3351000 a loop around a role layer lifts both, outermost first, each with the origin its rule names", () => {
    // The picture's own style (its roles' rules), then the slice's loop's, as dioramaSliceSvg writes them.
    const picture = gate(
      roleRule(".art-m-subject", { move: "sway", seconds: 3.5 }, FIGURE_SUBJECT_ORIGIN) +
        roleRule(".art-m-back", { move: "drift", seconds: 9 }, CENTER_ORIGIN) +
        frames("sway") +
        frames("drift"),
    );
    const body = `<g class="dio"><g class="art-m-back"><rect width="100" height="100"/><path d="M0 0h1"/></g></g>`;
    expect(liftSvg(`${HEAD}${picture}${gate(loop("dio", "float", 5))}${body}</svg>`)).toEqual({
      svg: `${HEAD}${body}</svg>`,
      moves: [
        { move: "float", seconds: 5, origin: null },
        { move: "drift", seconds: 9, origin: [50, 50] },
      ],
    });
    // A role layer alone, where no loop runs, about the subject's foot.
    const subject = `<g class="art-m-subject"><path d="M0 0h1"/></g>`;
    expect(liftSvg(`${HEAD}${picture}${subject}</svg>`)?.moves).toEqual([{ move: "sway", seconds: 3.5, origin: [50, 100] }]);
  });

  it("R3351000 a staggered piece, a second animated group, a particle or a spin is not lifted", () => {
    const styles = gate(roleRule(".art-m-back", { move: "drift", seconds: 9 }, CENTER_ORIGIN) + roleRule(".art-m-light", { move: "pulse", seconds: 4 }, CENTER_ORIGIN) + frames("drift") + frames("pulse"));
    const looped = (move: ArtMove = "bob"): string => styles + gate(loop("dio", move, 4));
    const piece = `<path d="M0 0h1"/>`;
    const unlifted: Record<string, string> = {
      staggered: `${HEAD}${looped()}<g class="dio" style="animation-delay:-0s">${piece}</g><g class="dio" style="animation-delay:-2s">${piece}</g></svg>`,
      "one staggered": `${HEAD}${looped()}<g class="dio" style="animation-delay:-0s">${piece}</g></svg>`,
      "two roles": `${HEAD}${looped()}<g class="dio"><g class="art-m-back">${piece}</g><g class="art-m-light">${piece}</g></g></svg>`,
      "a role beside a piece": `${HEAD}${looped()}<g class="dio">${piece}<g class="art-m-light">${piece}</g></g></svg>`,
      "a loop in a role in a loop": `${HEAD}${looped()}<g class="dio"><g class="art-m-back"><g class="art-m-light">${piece}</g></g></g></svg>`,
      "a particle": `${HEAD}${looped()}<g class="dio"><g class="art-m-back">${piece}</g></g><g class="art-m-particle"><g transform="translate(30 40)"><circle r="1"/></g></g></svg>`,
      "a placed subject": `${HEAD}${looped()}<g class="art-m-back"><g transform="translate(10 0)"><g class="dio">${piece}</g></g></g></svg>`,
      "a spin": `${HEAD}${looped("spin")}<g class="dio">${piece}</g></svg>`,
      "a flare": `${HEAD}${looped("flare")}<g class="dio">${piece}</g></svg>`,
      "a loop with no rule": `${HEAD}${styles}<g class="dio">${piece}</g></svg>`,
      "no motion": `${HEAD}<g class="dio">${piece}</g></svg>`,
      "pieces outside the loop": `${HEAD}${looped()}${piece}<g class="dio">${piece}</g></svg>`,
      "not 100 square": `<svg viewBox="0 0 30 20" width="30" height="20" xmlns="http://www.w3.org/2000/svg"><defs/>${looped()}<g class="dio">${piece}</g></svg>`,
    };
    for (const [what, svg] of Object.entries(unlifted)) expect(liftSvg(svg), what).toBeNull();
  });

  it("R3351000 every lift in DIORAMAS is its slice less the motion style, and stinkyscape-006, -074, -086, meditative-049 and core-100 each lift a slice", () => {
    const lifting = new Set<string>();
    for (const [id, diorama] of DIORAMAS) {
      // As a board face draws them: every slice on its loop, moving as the card's art does.
      const motion = artMotionOf(id);
      for (const radiant of [false, true]) {
        diorama.slices.forEach((_, k) => {
          const lift = liftedSlice(id, radiant, k, true, motion);
          const slice = dioramaSliceSvg(id, radiant, k, true, motion);
          // The farthest is the window's own background, which CardArt never lifts: in every diorama it holds the sky beside
          // pieces of other roles, so it cannot be.
          if (k === 0) expect(lift, `${id} 0`).toBeNull();
          if (lift === null) {
            expect(sliceWindowUri(id, radiant, k, true, motion), `${id} ${String(k)}`).toBe(dioramaSliceUri(id, radiant, k, true, motion));
            return;
          }
          lifting.add(id);
          const still = decoded(lift.uri);
          expect(still, `${id} ${String(k)}`).toBe(slice.replace(MOTION_STYLE, ""));
          expect(still).not.toContain("prefers-reduced-motion");
          expect(lift.moves.length).toBeGreaterThanOrEqual(1);
          expect(lift.moves.length).toBeLessThanOrEqual(2);
          for (const move of lift.moves) expect(LIFTABLE_MOVES.has(move.move), `${id} ${move.move}`).toBe(true);
          expect(sliceWindowUri(id, radiant, k, true, motion)).toBe(lift.uri);
        });
      }
    }
    for (const id of ["stinkyscape-006", "stinkyscape-074", "stinkyscape-086", "meditative-049", "core-100"]) expect(lifting.has(id), id).toBe(true);
  });

  it("R3351000 artLive.css runs each liftable move as motion.ts writes it, px as percent, in transform properties, inside the reduced-motion gate", () => {
    const flat = (css: string): string => css.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\s+/g, "").replace(/;\}/g, "}");
    const css = flat(readFileSync(join(HERE, "..", "artLive.css"), "utf8"));
    const gateAt = css.indexOf("@media(prefers-reduced-motion:no-preference){");
    expect(gateAt).toBeGreaterThan(0);
    const [outside, inside] = [css.slice(0, gateAt), css.slice(gateAt)];
    // The page names its curves through motion.css (#2110): the picture's ease-in-out is its `--ease-in-out`.
    const timing: Readonly<Record<string, string>> = { linear: "linear", "ease-in-out": "var(--ease-in-out)" };
    for (const move of LIFTABLE_MOVES) {
      const spec = ART_MOVES[move];
      expect(timing[spec.timing], move).toBeDefined();
      const rule = flat(`:root:not([data-reduce-motion="true"]) .cf-art-lift[data-lift-move="${move}"]{animation:cf-lift-${move} var(--lift-seconds) ${timing[spec.timing] ?? ""} infinite}`);
      const keyframes = flat(`@keyframes cf-lift-${move}{${asLift(spec.keyframes)}}`);
      expect(inside, move).toContain(rule);
      expect(inside, move).toContain(keyframes);
      expect(outside).not.toContain(`cf-lift-${move}`);
      // Transform and opacity alone, as the picture's own moves are (R2715001), and the lift's in transform properties.
      expect(spec.keyframes.replace(/transform:[^;}]*|opacity:[^;}]*/g, "")).toMatch(/^[0-9%,{};]*$/);
      expect(keyframes).not.toContain("transform:");
    }
  });
});
