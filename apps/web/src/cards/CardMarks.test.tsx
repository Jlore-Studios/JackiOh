// R437: a marked card shows its mark (#50 K-Pop Fanatic's pending steal on its target). The view's
// `CardView.marks` is read through `marksOf` (marks.ts) and drawn by CardMarks on the board's card
// wrapper (game/Card.tsx): units and face-up backrow cards, on either seat, rendered the way the board
// renders them, from fixture views carrying `marks`.

import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

import { CATALOG } from "@jackioh/cards";
import type { BackrowView, CardMark, PlayerView, UnitView } from "@jackioh/shared";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView } from "../test/fixtures.ts";
import { marksTestid } from "./CardMarks.tsx";
import {
  DEFAULT_MARK_COLOR,
  MARK_PALETTES,
  MARK_WORDS,
  UNKNOWN_MARK_WORDS,
  markColorOf,
  markEventOf,
  markWords,
  marksOf,
  paletteFor,
} from "./marks.ts";

afterEach(() => {
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);
const STEAL: CardMark = { mark: "steal", color: "purple" };

type FaceUp = Extract<BackrowView, { faceDown: false }>;

/** fullBoardView with marks on: the viewer's lane-1 unit, the enemy's lane-1 unit, a face-up backrow card on each seat. */
function markedView(marks: readonly CardMark[] = [STEAL]): { view: PlayerView; ids: Record<string, string> } {
  const view = fullBoardView();
  const mine = view.you.units[0] as UnitView;
  const theirs = view.opponent.units[0] as UnitView;
  const myField = view.you.backrow[0] as FaceUp;
  const theirField = view.opponent.backrow[2] as FaceUp;
  const next: PlayerView = {
    ...view,
    you: {
      ...view.you,
      units: view.you.units.map((u) => (u === mine ? { ...u, marks: [...marks] } : u)),
      backrow: view.you.backrow.map((b) => (b === myField ? { ...myField, marks: [...marks] } : b)),
    },
    opponent: {
      ...view.opponent,
      units: view.opponent.units.map((u) => (u === theirs ? { ...u, marks: [...marks] } : u)),
      backrow: view.opponent.backrow.map((b) => (b === theirField ? { ...theirField, marks: [...marks] } : b)),
    },
  };
  return { view: next, ids: { mine: mine.instanceId, theirs: theirs.instanceId, myField: myField.instanceId, theirField: theirField.instanceId } };
}

function renderBoard(view: PlayerView): HTMLElement {
  return render(
    <CatalogContext.Provider value={lookup}>
      <Board view={view} />
    </CatalogContext.Provider>,
  ).container;
}

function aura(root: ParentNode, instanceId: string): HTMLElement | null {
  return root.querySelector<HTMLElement>(`[data-testid="${marksTestid(instanceId)}"]`);
}

describe("R437 the mark on the board", () => {
  it("R437 renders on units and face-up backrow cards of both seats, inside the card, with the root naming the mark", () => {
    const { view, ids } = markedView();
    const root = renderBoard(view);
    for (const id of Object.values(ids)) {
      const card = root.querySelector(`[data-testid="${testid.card(id)}"]`);
      expect(card, id).not.toBeNull();
      expect(card?.getAttribute("data-marks"), id).toBe("steal");
      const marks = aura(root, id);
      expect(marks, id).not.toBeNull();
      expect(card?.contains(marks ?? null), `${id}'s aura sits on its card`).toBe(true);
      expect(marks?.getAttribute("data-mark-color")).toBe("purple");
      expect(marks?.querySelectorAll(".card-marks__mote").length).toBeGreaterThan(0);
      expect(marks?.querySelector(".card-marks__vignette")).not.toBeNull();
      expect(marks?.querySelector(".card-marks__edge")).not.toBeNull();
    }
  });

  it("R437 the badge says it in words and by shape: a sparkle, a tooltip and hidden text, and it never starts a card testid", () => {
    const { view, ids } = markedView();
    const root = renderBoard(view);
    const badge = aura(root, ids.theirs ?? "")?.querySelector<HTMLElement>(".card-mark-badge");
    expect(badge?.getAttribute("title")).toBe(MARK_WORDS.steal?.text);
    expect(badge?.querySelector(".card-mark-badge__text")?.textContent).toBe(MARK_WORDS.steal?.text);
    expect(badge?.querySelector(".card-mark-badge__glyph")?.textContent).toBe("✦");
    expect(marksTestid("u6").startsWith("card-")).toBe(false);
    // The aura takes no pointer event (marks.css); the card underneath keeps every click.
    expect(aura(root, ids.theirs ?? "")?.querySelector(".card-marks__aura")?.getAttribute("aria-hidden")).toBe("true");
  });

  it("R437 the colour key maps through the palette table, one badge per mark, and an unknown key falls back to the default", () => {
    const { view, ids } = markedView([
      { mark: "steal", color: "green" },
      { mark: "curse", color: "ultraviolet" },
    ]);
    const root = renderBoard(view);
    const marks = aura(root, ids.mine ?? "");
    expect(marks?.getAttribute("data-marks")).toBe("steal curse");
    expect(marks?.style.getPropertyValue("--mark-rim")).toBe(MARK_PALETTES.green.rim);
    const badges = Array.from(marks?.querySelectorAll<HTMLElement>(".card-mark-badge") ?? []);
    expect(badges.map((badge) => badge.getAttribute("data-mark-color"))).toEqual(["green", DEFAULT_MARK_COLOR]);
    expect(badges[1]?.style.getPropertyValue("--mark-rim")).toBe(MARK_PALETTES[DEFAULT_MARK_COLOR].rim);
    expect(badges[1]?.getAttribute("title")).toBe(UNKNOWN_MARK_WORDS.text);
  });

  it("R437 no mark, no aura: the unmarked board renders none, and an empty list renders none", () => {
    const root = renderBoard(fullBoardView());
    expect(root.querySelector(".card-marks")).toBeNull();
    expect(root.querySelector("[data-marks]")).toBeNull();
    cleanup();
    const { view } = markedView([]);
    expect(renderBoard(view).querySelector(".card-marks")).toBeNull();
  });
});

describe("R437 the tables and the adapters", () => {
  it("R437 at least purple, green, crimson, gold, cyan, blue and orange, each with its colours and particles", () => {
    for (const color of ["purple", "green", "crimson", "gold", "cyan", "blue", "orange"]) {
      expect(markColorOf(color)).toBe(color);
      const palette = paletteFor(color);
      for (const value of [palette.rim, palette.core, palette.glow]) expect(value).toMatch(/^#[0-9a-f]{6}$/i);
    }
    expect(markColorOf("ultraviolet")).toBe(DEFAULT_MARK_COLOR);
    expect(paletteFor("")).toBe(MARK_PALETTES[DEFAULT_MARK_COLOR]);
    expect(markColorOf("toString")).toBe(DEFAULT_MARK_COLOR);
  });

  it("R437 a mark's words, and generic words for a mark the table does not know", () => {
    expect(markWords("steal").text).toMatch(/stolen/);
    expect(markWords("hex")).toBe(UNKNOWN_MARK_WORDS);
    expect(markWords("constructor")).toBe(UNKNOWN_MARK_WORDS);
  });

  it("R437 marksOf reads the view's marks and nothing else; markEventOf reads the marked event", () => {
    expect(marksOf(null)).toEqual([]);
    expect(marksOf({})).toEqual([]);
    expect(marksOf({ marks: [STEAL] })).toEqual([STEAL]);
    expect(marksOf({ marks: [{ mark: 3, color: "x" } as unknown as CardMark, STEAL] })).toEqual([STEAL]);
    expect(markEventOf({ type: "marked", instanceId: "u6", mark: "steal", color: "purple", added: true })).toEqual({
      instanceId: "u6",
      mark: "steal",
      color: "purple",
      added: true,
    });
    expect(markEventOf({ type: "turnStarted", player: "p1", turn: 1 })).toBeNull();
  });
});

describe("R437 reduced motion", () => {
  function sheet(): string {
    for (const candidate of ["src/cards/marks.css", "apps/web/src/cards/marks.css"]) {
      const path = resolve(process.cwd(), candidate);
      if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    }
    throw new Error("marks.css not found");
  }

  it("R437 under the media query and under <html data-reduce-motion> every part holds still and stays drawn", () => {
    const css = sheet();
    const media = /@media \(prefers-reduced-motion: reduce\) \{([\s\S]*?)\n\}/.exec(css)?.[1] ?? "";
    for (const part of [".card-marks__vignette", ".card-marks__edge", ".card-marks__mote", ".card-mark-badge__glyph"]) {
      expect(media, `${part} under the media query`).toContain(part);
      expect(css, `${part} under the setting`).toContain(`:root[data-reduce-motion="true"] ${part}`);
    }
    expect(media).toMatch(/animation: none/);
    expect(css).toMatch(/:root\[data-reduce-motion="true"\] \.card-mark-badge__glyph \{\s*animation: none;/);
    // Still drawn: nothing hides the aura, and the motes rest visible.
    expect(css).not.toMatch(/display: none|visibility: hidden/);
    expect(media).toMatch(/\.card-marks__mote \{\s*opacity: 0\.8;/);
  });

  it("R437 every mark keyframe moves only what the compositor runs", () => {
    const css = sheet();
    const bodies = [...css.matchAll(/@keyframes [\w-]+ \{([\s\S]*?)\n\}/g)].map((match) => match[1] ?? "");
    expect(bodies.length).toBeGreaterThanOrEqual(4);
    for (const body of bodies) {
      for (const decl of body.matchAll(/([a-z-]+):/g)) expect(["transform", "opacity"]).toContain(decl[1]);
    }
  });
});
