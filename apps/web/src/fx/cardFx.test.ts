// R502: the unmistakable cast on draw, and the per-card flourishes of #21 Hinder and #27 Blood Ridden
// Glowy Jelly Bean (cardFx.ts), planned the way FxLayer plans them: each entry remembered, then
// planned against the view the runner planned it against, from the redacted stream alone.
//
// The last block drives the real engine (audio/test/realGame.ts) to a game where each card is cast
// as it is drawn, and plans both seats' own redacted windows.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, planEntries, type AnimationEntry } from "../game/animations.ts";
import { testid, type Side } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { realGame } from "../audio/test/realGame.ts";
import { CARD_FX, CARD_RECIPES } from "./cardFx.ts";
import {
  FX_BLOOD_FLIGHT_FRACTION,
  FX_BLOOD_PICK_BASE,
  FX_BLOOD_PICK_STRIDE,
  FX_CRACK_HIT_AT,
  FX_CRACK_STAGGER_MS,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
} from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "./types.ts";

const HINDER = "core-021";
const BLOOD = "core-027";

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv, D?: number): { entry: AnimationEntry; cues: FxCue[] }[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.map((original) => {
    const entry = D === undefined ? original : { ...original, durationMs: D };
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

function cuesFor(results: ReturnType<typeof planAll>, type: GameEvent["type"]): FxCue[] {
  return results.filter((result) => result.entry.events.some((event) => event.type === type)).flatMap((result) => result.cues);
}

/** Where a cue ends, in ms after its entry starts (R200's bound is D + FX_MAX_TAIL_MS). */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const tid = (value: string): FxAnchor => ({ kind: "testid", testid: value });

const drawn = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "drawn", player, instanceId, defId });
const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 0 });
const resolved = (player: PlayerId, instanceId: string, defId: string, radiant = false): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 0,
  radiant,
});
const riderAdded = (player: PlayerId): GameEvent => ({ type: "modifierChanged", player, modifierId: "nextTurnMana", added: true });

/** The view `viewFor` gives once #21 has lowered `victim`'s next refresh by `lower`. */
function afterHinder(view: PlayerView, victim: Side, lower: number): PlayerView {
  const side = view[victim];
  return { ...view, [victim]: { ...side, modifiers: [...side.modifiers, { id: "nextTurnMana", label: `Next refresh −${String(lower)} mana` }] } };
}

/** The opponent (p2) casts Hinder on draw against the viewer (p1): the stream both seats get. */
function hinderStream(caster: PlayerId, victim: PlayerId): GameEvent[] {
  return [drawn(caster, "c21", HINDER), played(caster, "c21", HINDER), riderAdded(victim), resolved(caster, "c21", HINDER)];
}

describe("R502 a cast on draw bursts out of the drawer's Deck pile", () => {
  it("R502 the opponent's Hinder: gold and arcane at their library, with rays, a ring and a shake, and nothing at their hand", () => {
    const cues = cuesFor(planAll(hinderStream("p2", "p1"), fullBoardView(), envOf()), "cardPlayed");
    const pile = tid(animTestid.library("opponent"));
    expect(cues.filter((cue) => "at" in cue && JSON.stringify(cue.at) === JSON.stringify(pile)).map((cue) => cue.kind).sort()).toEqual(
      ["burst", "burst", "ring", "rays"].sort(),
    );
    expect(cues.some((cue) => cue.kind === "shake")).toBe(true);
    expect(JSON.stringify(cues)).not.toContain(animTestid.hand("opponent"));
  });

  it("R502 the viewer's own cast on draw bursts out of the viewer's library, never a hand card it never was", () => {
    const cues = cuesFor(planAll(hinderStream("p1", "p2"), fullBoardView(), envOf()), "cardPlayed");
    expect(JSON.stringify(cues)).toContain(animTestid.library("you"));
    expect(JSON.stringify(cues)).not.toContain(testid.handCard("c21"));
  });

  it("R502 R202 a hidden cast on draw plans the same burst as a readable one, and never a per-card recipe", () => {
    const hidden = [drawn("p2", "hidden", "hidden"), played("p2", "hidden", "hidden"), riderAdded("p1"), resolved("p2", "hidden", "hidden")];
    const readable = planAll(hinderStream("p2", "p1"), fullBoardView(), envOf());
    const secret = planAll(hidden, fullBoardView(), envOf({ next: afterHinder(fullBoardView(), "you", 1) }));
    expect(cuesFor(secret, "cardPlayed")).toEqual(cuesFor(readable, "cardPlayed"));
    // The modifier is only glinted: no card is known to be resolving, so no crack.
    expect(cuesFor(secret, "modifierChanged").some((cue) => cue.kind === "fracture" || cue.kind === "crack")).toBe(false);
  });

  it("R502 a plain play from the hand gets no burst at the pile", () => {
    const cues = cuesFor(planAll([played("p2", "c5", "core-005")], fullBoardView(), envOf()), "cardPlayed");
    expect(JSON.stringify(cues)).not.toContain(animTestid.library("opponent"));
  });
});

describe("R502 #21 Hinder cracks the victim's crystals", () => {
  it("R502 the number is the view's: with the next refresh 2 lower, the viewer's last two crystals fracture, staggered from the hit", () => {
    const D = 400;
    const env = envOf({ next: afterHinder(fullBoardView(), "you", 2) });
    const cues = cuesFor(planAll(hinderStream("p2", "p1"), fullBoardView(), env, D), "modifierChanged");
    const hit = Math.round(FX_CRACK_HIT_AT * D);
    const fractures = cues.filter((cue) => cue.kind === "fracture");
    // fullBoardView: the viewer's tray draws 4 crystals (max 4), so the lost two are indices 2 and 3.
    expect(fractures.map((cue) => cue.kind === "fracture" && cue.at)).toEqual([
      { kind: "crystal", side: "you", index: 2 },
      { kind: "crystal", side: "you", index: 3 },
    ]);
    expect(fractures.map((cue) => cue.delayMs)).toEqual([hit, hit + FX_CRACK_STAGGER_MS]);
    // A frost bolt from the caster's hero into the tray lands at the hit.
    expect(cues).toContainEqual(
      expect.objectContaining({ kind: "projectile", preset: "frost", from: tid(testid.hero("opponent")), to: tid(animTestid.mana("you")), flightMs: hit }),
    );
    expect(cues.some((cue) => cue.kind === "shake")).toBe(true);
    // It replaces the row's arcane glint.
    expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "arcane")).toBe(false);
  });

  it("R502 on the other seat the same event cracks the opponent's tray, one crystal for a base Hinder", () => {
    const view = fullBoardView();
    const env = envOf({ next: afterHinder(view, "opponent", 1) });
    const cues = cuesFor(planAll(hinderStream("p1", "p2"), view, env), "modifierChanged");
    expect(cues.filter((cue) => cue.kind === "fracture").map((cue) => cue.kind === "fracture" && cue.at)).toEqual([
      { kind: "crystal", side: "opponent", index: 2 },
    ]);
  });

  it("R502 with no number to read (no newer view, or a rider that does not lower the refresh) the whole tray cracks and no crystal is guessed", () => {
    for (const next of [undefined, fullBoardView(), afterHinder(fullBoardView(), "opponent", 3)]) {
      const cues = cuesFor(planAll(hinderStream("p2", "p1"), fullBoardView(), envOf(next === undefined ? {} : { next })), "modifierChanged");
      expect(cues.some((cue) => cue.kind === "fracture")).toBe(false);
      expect(cues).toContainEqual(expect.objectContaining({ kind: "crack", at: tid(animTestid.mana("you")) }));
    }
  });

  it("R502 never more fractures than the tray draws crystals", () => {
    const env = envOf({ next: afterHinder(fullBoardView(), "opponent", 9) });
    const cues = cuesFor(planAll(hinderStream("p1", "p2"), fullBoardView(), env), "modifierChanged");
    expect(cues.filter((cue) => cue.kind === "fracture")).toHaveLength(3);
  });

  it("R502 a modifier spent, or another modifier, is not cracked", () => {
    const spent: GameEvent = { type: "modifierChanged", player: "p1", modifierId: "nextTurnMana", added: false };
    const other: GameEvent = { type: "modifierChanged", player: "p1", modifierId: "m9", added: true };
    const env = envOf({ next: afterHinder(fullBoardView(), "you", 1) });
    const cues = planAll([drawn("p2", "c21", HINDER), played("p2", "c21", HINDER), spent, other], fullBoardView(), env).flatMap((r) =>
      r.entry.events.some((e) => e.type === "modifierChanged") ? r.cues : [],
    );
    expect(cues.some((cue) => cue.kind === "fracture" || cue.kind === "crack")).toBe(false);
  });
});

describe("R502 #27 Blood Ridden's blood drain", () => {
  const bloodStream = (caster: PlayerId, radiantId: string, radiantDef: string, count = 1): GameEvent[] => [
    drawn(caster, "c27", BLOOD),
    played(caster, "c27", BLOOD),
    ...Array.from({ length: count }, (): GameEvent => ({ type: "radiantSet", instanceId: radiantId, defId: radiantDef, zone: { z: "hand", player: caster } })),
    { type: "healthLost", player: caster, amount: 5 },
    resolved(caster, "c27", BLOOD),
  ];

  it("R502 on the caster's seat a crimson stream runs from their hero into the named hand card, which bursts gold on arrival", () => {
    const view = fullBoardView();
    const hand = view.you.hand;
    const target = Array.isArray(hand) ? (hand[0]?.instanceId ?? "") : "";
    const D = 400;
    const cues = cuesFor(planAll(bloodStream("p1", target, "core-002"), view, envOf(), D), "radiantSet");
    const flight = Math.round(FX_BLOOD_FLIGHT_FRACTION * D);
    expect(cues).toContainEqual(
      expect.objectContaining({ kind: "projectile", preset: "blood", from: tid(testid.hero("you")), to: tid(testid.handCard(target)), flightMs: flight }),
    );
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: "gold", at: tid(testid.handCard(target)), delayMs: flight }));
  });

  it("R502 R202 on the other seat the stream lands on a back picked by the play's own count, and the cues name no card", () => {
    const cues = planAll(bloodStream("p2", "hidden", "hidden", 2), fullBoardView(), envOf());
    const radiant = cuesFor(cues, "radiantSet");
    const picks = radiant.flatMap((cue) => (cue.kind === "projectile" && cue.to.kind === "handCard" ? [cue.to] : []));
    expect(picks).toEqual([
      { kind: "handCard", side: "opponent", pick: FX_BLOOD_PICK_BASE },
      { kind: "handCard", side: "opponent", pick: FX_BLOOD_PICK_BASE + FX_BLOOD_PICK_STRIDE },
    ]);
    expect(JSON.stringify(radiant)).not.toMatch(/core-\d+|hand-card-/);
    // Whatever card it hides, the same cues.
    const other = planAll(bloodStream("p2", "hidden", "hidden", 2), fullBoardView(), envOf());
    expect(cuesFor(other, "radiantSet")).toEqual(radiant);
  });

  it("R502 its blood price spills at the caster's hero on top of the row's own drain and splat", () => {
    const cues = cuesFor(planAll(bloodStream("p2", "hidden", "hidden"), fullBoardView(), envOf()), "healthLost");
    expect(cues.filter((cue) => cue.kind === "burst" && cue.preset === "blood").length).toBeGreaterThanOrEqual(2);
    expect(cues).toContainEqual(expect.objectContaining({ kind: "splat", tone: "loss", amount: 5 }));
    expect(cues).toContainEqual(expect.objectContaining({ kind: "burst", preset: "void" }));
  });

  it("R502 health another seat loses while it resolves, or a Radiant pick outside the caster's hand, is not the drain's", () => {
    const events: GameEvent[] = [
      drawn("p2", "c27", BLOOD),
      played("p2", "c27", BLOOD),
      { type: "healthLost", player: "p1", amount: 2 },
      { type: "radiantSet", instanceId: "u1", defId: "core-004", zone: { z: "field", player: "p1", row: "units", lane: 1 } },
    ];
    const cues = planAll(events, fullBoardView(), envOf()).flatMap((r) => (r.entry.events[0]?.type === "cardPlayed" ? [] : r.cues));
    expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "blood")).toBe(false);
  });
});

describe("R502 the per-card table", () => {
  it("R502 one table keys each recipe by definition, and every recipe it names exists", () => {
    expect(CARD_FX).toEqual({ [HINDER]: "manaCrack", [BLOOD]: "bloodDrain" });
    for (const key of Object.values(CARD_FX)) expect(typeof CARD_RECIPES[key]).toBe("function");
  });

  it("R200 every cue the cast on draw and both recipes plan starts inside its entry and ends within FX_MAX_TAIL_MS of its end, for short and long entries", () => {
    for (const D of [120, 200, 400, 732, 1400]) {
      const results = [
        ...planAll(hinderStream("p2", "p1"), fullBoardView(), envOf({ next: afterHinder(fullBoardView(), "you", 2) }), D),
        ...planAll(
          [drawn("p2", "c27", BLOOD), played("p2", "c27", BLOOD), { type: "radiantSet", instanceId: "hidden", defId: "hidden", zone: { z: "hand", player: "p2" } }, { type: "healthLost", player: "p2", amount: 5 }],
          fullBoardView(),
          envOf(),
          D,
        ),
      ];
      const cues = results.flatMap((r) => r.cues);
      expect(cues.length).toBeGreaterThan(10);
      for (const cue of cues) {
        expect(cue.delayMs, `${cue.kind} at D=${String(D)}`).toBeGreaterThanOrEqual(0);
        expect(cue.delayMs, `${cue.kind} at D=${String(D)}`).toBeLessThanOrEqual(D);
        if (cue.kind === "projectile") expect(cue.delayMs + cue.flightMs).toBeLessThanOrEqual(D);
        expect(endOf(cue), `${cue.kind} at D=${String(D)}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
      }
    }
  });

  it("R200 intensity 0 (the effects off, or reduced motion never planning) draws none of it", () => {
    const results = planAll(hinderStream("p2", "p1"), fullBoardView(), envOf({ intensity: 0, next: afterHinder(fullBoardView(), "you", 2) }));
    expect(results.flatMap((r) => r.cues)).toEqual([]);
  });
});

/* ------------------------------------------------------------------------------------------- *
 * The real engine: both seats' redacted windows
 * ------------------------------------------------------------------------------------------- */

const DECK_A = [
  "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-008", "core-009", "core-010", "core-011",
  "core-012", "core-013", "core-014", "core-015", "core-016", "core-017", "core-018", "core-019", "core-020", "core-022",
];
const DECK_B = [
  "core-021", "core-027", "core-030", "core-032", "core-033", "core-034", "core-035", "core-036", "core-037", "core-038",
  "core-040", "core-042", "core-043", "core-044", "core-045", "core-046", "core-047", "core-048", "core-051", "core-052",
];

type Seats = { p1: PlayerView; p2: PlayerView };

/**
 * The first seed whose opening turns cast `defId` on draw for p2 and leave both seats' views as
 * `settled` wants them (the rider still standing, the Radiant pick made).
 */
function castOnDrawGame(defId: string, settled: (seats: Seats) => boolean): Seats {
  for (let seed = 0; seed < 400; seed += 1) {
    const game = realGame(`r502-${String(seed)}`, [DECK_A, DECK_B]);
    const p1 = game.view("p1");
    const at = p1.events.findIndex((event) => event.type === "cardPlayed" && event.defId === defId && event.player === "p2");
    const seats = { p1, p2: game.view("p2") };
    if (at > 0 && p1.events[at - 1]?.type === "drawn" && settled(seats)) return seats;
  }
  throw new Error(`no seed casts ${defId} on draw in its opening turns`);
}

/** Plans a whole window, each event against the view (the board both seats show then). */
function planWindow(view: PlayerView): { entry: AnimationEntry; cues: FxCue[] }[] {
  const env = envOf({ next: view });
  return planEntries(view.events, view, false).map((entry) => {
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, view, env) };
  });
}

describe("R502 real games, both seats", () => {
  it("R502 #21 Hinder cast on draw: both seats burst it out of p2's Deck pile and crack p1's crystals by the view's number", () => {
    const { p1, p2 } = castOnDrawGame(HINDER, ({ p1: view }) => view.you.modifiers.some((modifier) => modifier.id === "nextTurnMana"));
    for (const [view, caster, victim] of [
      [p1, "opponent", "you"],
      [p2, "you", "opponent"],
    ] as const) {
      const results = planWindow(view);
      const cast = results.find((r) => r.entry.events.some((e) => e.type === "cardPlayed" && e.defId === HINDER));
      expect(JSON.stringify(cast?.cues), `${view.viewer}'s burst`).toContain(animTestid.library(caster));
      const crack = results.find((r) => r.entry.events.some((e) => e.type === "modifierChanged" && e.modifierId === "nextTurnMana" && e.added));
      const fractures = (crack?.cues ?? []).filter((cue) => cue.kind === "fracture");
      // viewFor lists the rider as "Next refresh −1 mana" on both seats, so one crystal fractures.
      expect(fractures, `${view.viewer}'s crack`).toHaveLength(1);
      expect(JSON.stringify(fractures)).toContain(`"side":"${victim}"`);
    }
  });

  it("R502 R202 #27 Blood Ridden cast on draw: the caster's seat drains into the named card, the other seat into a back that names nothing", () => {
    const { p1, p2 } = castOnDrawGame(BLOOD, ({ p2: view }) => view.events.some((event) => event.type === "radiantSet" && event.instanceId !== "hidden"));
    const named = p2.events.find((e): e is Extract<GameEvent, { type: "radiantSet" }> => e.type === "radiantSet");
    expect(named?.instanceId).not.toBe("hidden");
    const own = planWindow(p2).find((r) => r.entry.events.some((e) => e.type === "radiantSet"));
    expect(own?.cues).toContainEqual(expect.objectContaining({ kind: "projectile", preset: "blood", to: tid(testid.handCard(named?.instanceId ?? "")) }));

    const theirs = planWindow(p1).find((r) => r.entry.events.some((e) => e.type === "radiantSet"));
    expect(theirs?.cues).toContainEqual(expect.objectContaining({ kind: "projectile", preset: "blood", to: expect.objectContaining({ kind: "handCard", side: "opponent" }) }));
    const said = JSON.stringify(theirs?.cues);
    expect(said).not.toContain(named?.instanceId ?? "");
    expect(said).not.toContain(named?.defId ?? "");
  });
});
