// The frame plan of a shared GIF (#513, R513000, decision D5): a pure function of a clip's views on a
// fixed frame clock. These hold what makes it shareable: the same clip and options give the same plan
// whatever the device's settings or clock, a frame draws only what its step's view shows, and no
// account name can reach it.

import type { ReplayStep } from "@jackioh/engine";
import type { PlayerView } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetFxSettingsForTests, setFxSettings } from "../../fx/settings.ts";
import { HIDDEN_ID } from "../../game/animations.ts";
import { __resetSettingsForTests, writeSettings } from "../../settings/store.ts";
import { baseView, fullBoardView } from "../../test/fixtures.ts";
import { foldedReplaySteps } from "../../test/replay.ts";
import { REPLAY_GIF_FALLBACK_FPS, REPLAY_GIF_FPS, REPLAY_GIF_MAX_SECONDS, REPLAY_GIF_MAX_STEPS, REPLAY_GIF_STEP_HOLD_MS } from "../config.ts";
import { GIF_MARK, faceKey, planFrames } from "./plan.ts";
import type { GifFrame, GifItem, GifPlanOptions } from "./plan.ts";

const OPTS: GifPlanOptions = {
  width: 800,
  height: 450,
  fps: REPLAY_GIF_FPS,
  catalogVersion: "v0.3.6",
  portraits: { p1: "vanilla", p2: "gary" },
};

const steps = foldedReplaySteps();

/** A frame holding a ring: a gold outline over the board, with no fill. */
function ringed(frame: GifFrame): boolean {
  return frame.items.some((item) => item.kind === "rect" && item.fill === null && item.stroke !== null && item.line > 1 && item.w > 0 && item.stroke === "#ffd24a");
}

/** The first turn of at least three steps whose plan rings something: the clip these tests use. */
function clipWithEntries(): ReplayStep[] {
  const turns = [...new Set(steps.map((step) => step.turn))];
  for (const turn of turns) {
    const clip = steps.filter((step) => step.turn === turn);
    if (clip.length >= 3 && planFrames(clip, OPTS).frames.some(ringed)) return clip;
  }
  throw new Error("no turn of the folded game has an animation entry to ring");
}

const clip = clipWithEntries();

/** The definitions the field of a view shows, which are all a frame may draw. */
function fieldDefIds(view: PlayerView): Set<string> {
  const ids = new Set<string>();
  for (const side of [view.you, view.opponent]) {
    for (const held of [...side.units, ...side.backrow]) {
      if (held === null || ("faceDown" in held && held.faceDown)) continue;
      ids.add(held.defId);
    }
  }
  return ids;
}

function itemsOf(frame: GifFrame, kind: GifItem["kind"]): GifItem[] {
  return frame.items.filter((item) => item.kind === kind);
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-10-13T00:00:00Z"));
});

afterEach(() => {
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

describe("R513000 what a shared GIF holds", () => {
  it("R513000 the same clip and options give the same frame plan, whatever the device's settings", () => {
    const first = planFrames(clip, OPTS);
    expect(first.frames.length).toBeGreaterThan(clip.length);
    setFxSettings({ speed: 3, motion: "reduce" });
    writeSettings({ reduceMotion: true });
    vi.setSystemTime(new Date("2031-03-04T05:06:07Z"));
    const second = planFrames(clip, OPTS);
    expect(second).toEqual(first);
    expect(JSON.stringify(second)).toBe(JSON.stringify(first));
    expect(JSON.stringify(planFrames(clip, OPTS))).toBe(JSON.stringify(first));
  });

  it("R513000 a frame draws only the cards of the view it is drawn from, a face-down card as its back", () => {
    const plan = planFrames(clip, OPTS);
    const byStep = new Map(clip.map((step) => [step.step, step]));
    let cards = 0;
    for (const frame of plan.frames) {
      const step = byStep.get(frame.step);
      expect(step, `frame of step ${String(frame.step)} is in the clip`).toBeDefined();
      if (step === undefined) continue;
      const shown = fieldDefIds(step.view);
      for (const item of itemsOf(frame, "card")) {
        if (item.kind !== "card") continue;
        cards += 1;
        expect(item.defId).not.toBe(HIDDEN_ID);
        expect(shown.has(item.defId), `${item.defId} is on step ${String(frame.step)}'s field`).toBe(true);
      }
    }
    expect(cards).toBeGreaterThan(0);

    // A board with face-down cards: each is a back, and the cards are exactly the face-up ones.
    const view = fullBoardView();
    const hidden = [view.you, view.opponent].flatMap((side) => [...side.units, ...side.backrow]).filter((held) => held !== null && "faceDown" in held && held.faceDown).length;
    expect(hidden).toBeGreaterThan(0);
    const [frame] = planFrames([{ step: 0, turn: view.turn, view }], OPTS).frames;
    expect(frame && itemsOf(frame, "back")).toHaveLength(hidden);
    const faceUp = [view.you, view.opponent].flatMap((side) => [...side.units, ...side.backrow]).filter((held) => held !== null && !("faceDown" in held && held.faceDown)).length;
    expect(frame && itemsOf(frame, "card")).toHaveLength(faceUp);

    // The sentinel a redacted view names in a card's place is a back too, never a face.
    const redacted = baseView();
    const unit = fullBoardView().opponent.units[0];
    if (unit === null || unit === undefined) throw new Error("fullBoardView() has no enemy unit in lane 1");
    redacted.opponent.units[0] = { ...unit, defId: HIDDEN_ID };
    const [masked] = planFrames([{ step: 0, turn: 1, view: redacted }], OPTS).frames;
    expect(masked && itemsOf(masked, "back")).toHaveLength(1);
    expect(masked && itemsOf(masked, "card")).toHaveLength(0);
  });

  it("R513000 no frame plan holds an account name", () => {
    const allowed = /^(You|Opponent|JackiOh|v\d+\.\d+\.\d+|Turn \d+|Hand \d+|Mana \d+\/\d+|\d+|Draw|You win|You lose)$/u;
    const plan = planFrames(clip, OPTS);
    const texts = plan.frames.flatMap((frame) => frame.items.flatMap((item) => (item.kind === "text" ? [item.text] : [])));
    expect(texts.length).toBeGreaterThan(0);
    for (const text of texts) expect(text).toMatch(allowed);
    expect(texts).toContain(GIF_MARK);
    expect(texts).toContain("v0.3.6");
    expect(texts).toContain("You");
    expect(texts).toContain("Opponent");

    // A name planted wherever a view could carry one comes out nowhere.
    const view = fullBoardView({ result: { winner: "p1", reason: "concede" } });
    const planted = { ...view, name: "Ada Lovelace", you: { ...view.you, name: "Ada Lovelace" }, opponent: { ...view.opponent, name: "Ada Lovelace" } };
    const json = JSON.stringify(planFrames([{ step: 0, turn: view.turn, view: planted }], OPTS));
    expect(json).not.toContain("Ada");
    expect(json).toContain("You win");
  });

  it("the frame clock holds each entry and step for fixed times and the plan stays inside both caps", () => {
    const plan = planFrames(clip, OPTS);
    const tickCs = Math.round(100 / REPLAY_GIF_FPS);
    const holdCs = Math.round(REPLAY_GIF_STEP_HOLD_MS / 10);
    const entryFrames = plan.frames.filter(ringed);
    expect(entryFrames.length).toBeGreaterThan(0);
    for (const frame of entryFrames) expect(frame.delayCs).toBe(tickCs);
    // Each step ends on its own hold, and every frame is an entry's tick or a step's hold.
    const holds = plan.frames.filter((frame) => !ringed(frame));
    expect(holds.map((frame) => frame.step)).toEqual(clip.map((step) => step.step));
    for (const frame of holds) expect(frame.delayCs).toBe(holdCs);
    expect(plan.frames.reduce((sum, frame) => sum + frame.delayCs, 0)).toBeLessThanOrEqual(REPLAY_GIF_MAX_SECONDS * 100);
    expect(new Set(plan.frames.map((frame) => frame.step)).size).toBeLessThanOrEqual(REPLAY_GIF_MAX_STEPS);
    expect([plan.width, plan.height, plan.fps]).toEqual([800, 450, REPLAY_GIF_FPS]);

    // Half the frame rate, half the frames of each entry, and a tick twice as long.
    const slow = planFrames(clip, { ...OPTS, fps: REPLAY_GIF_FALLBACK_FPS });
    expect(slow.frames.filter(ringed)).toHaveLength(entryFrames.length / 2);
    for (const frame of slow.frames.filter(ringed)) expect(frame.delayCs).toBe(tickCs * 2);
  });

  it("a long clip is cut at the step cap and the second cap", () => {
    const long: ReplayStep[] = Array.from({ length: REPLAY_GIF_MAX_STEPS + 20 }, (_, step) => ({ step, turn: 1, view: baseView() }));
    const plan = planFrames(long, OPTS);
    expect(plan.frames).toHaveLength(Math.min(REPLAY_GIF_MAX_STEPS, Math.floor((REPLAY_GIF_MAX_SECONDS * 1000) / REPLAY_GIF_STEP_HOLD_MS)));
    expect(plan.frames.reduce((sum, frame) => sum + frame.delayCs, 0)).toBeLessThanOrEqual(REPLAY_GIF_MAX_SECONDS * 100);
  });

  it("draws both presets inside their canvas, with numbers rounded", () => {
    const view = fullBoardView();
    for (const size of [{ width: 800, height: 450 }, { width: 540, height: 540 }]) {
      const [frame] = planFrames([{ step: 0, turn: view.turn, view }], { ...OPTS, ...size }).frames;
      expect(frame).toBeDefined();
      for (const item of frame?.items ?? []) {
        if (item.kind === "text") {
          expect(Number.isInteger(item.x) && Number.isInteger(item.y) && Number.isInteger(item.size)).toBe(true);
          continue;
        }
        expect([item.x, item.y, item.w, item.h].every(Number.isInteger)).toBe(true);
        expect(item.x).toBeGreaterThanOrEqual(0);
        expect(item.y).toBeGreaterThanOrEqual(0);
        expect(item.x + item.w).toBeLessThanOrEqual(size.width);
        expect(item.y + item.h).toBeLessThanOrEqual(size.height);
      }
    }
  });

  it("faceKey tells a Radiant face from its base", () => {
    expect(faceKey("core-001", false)).toBe("core-001");
    expect(faceKey("core-001", true)).toBe("core-001+radiant");
  });
});
