// R502: the unmistakable cast on draw, and the per-card flourishes of #21 Hinder, #27 Blood Ridden
// Glowy Jelly Bean and Classic+ #24 Crushing Walls (cardFx.ts), planned the way FxLayer plans them,
// from the redacted stream alone. The last block drives the real engine (audio/test/realGame.ts).
// R3332000: every entrance an entry names is a recipe, and a Field Spell's plays as it lands face up.
// R3389000: a signature's recipe claims what its card does outside its own play: a cast its play makes,
// an event naming it as the source, and a whole pile's one entry; never for a hidden card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, entranceEntries, planEntries, type AnimationEntry, type EntranceResolver } from "../game/animations.ts";
import { testid, type Side } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { castOnDrawViews } from "../audio/test/realGame.ts";
import { CATALOG } from "../wire/cards.ts";
import { entranceKeyOf, planCardFx, planSignatureZone, signaturePlayOf, sourceIdOf } from "./cardFx.ts";
import { CARD_RECIPES } from "./cards/bespoke.ts";
import { STINKYSCAPE_TWO_SIGNATURES } from "./cards/stinkyscape/signature.ts";
import { FX_CARD_TABLE, parseFxCardTable } from "./cards/tables.ts";
import { ENTRANCES } from "./entrances.ts";
import { entranceCues } from "./takeover.ts";
import {
  FX_BLOOD_FLIGHT_FRACTION,
  FX_BLOOD_PICK_BASE,
  FX_BLOOD_PICK_STRIDE,
  FX_CRACK_HIT_AT,
  FX_CRACK_STAGGER_MS,
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  FX_WALLS_HIT_AT,
  FX_WALLS_REACH,
  FX_WALLS_TAIL_MS,
} from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "./types.ts";

const HINDER = "core-021";
const BLOOD = "core-027";
const WALLS = "classicplus-024";

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
    { type: "healthLost", player: caster, amount: 7 },
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
    expect(cues).toContainEqual(expect.objectContaining({ kind: "splat", tone: "loss", amount: 7 }));
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

/** A unit Crushing Walls destroys in an outer lane, as the engine reports it. */
const destroyedIn = (owner: PlayerId, instanceId: string): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId: "core-008",
  owner,
  controller: owner,
  attack: 4,
  maxHealth: 4,
  killerId: null,
});

/** p1 casts Crushing Walls and it destroys `count` cards, alternating sides, then resolves. */
function wallsStream(count: number): GameEvent[] {
  const crushed = Array.from({ length: count }, (_, k) => destroyedIn(k % 2 === 0 ? "p1" : "p2", `w${String(k)}`));
  return [played("p1", "c24", WALLS), ...crushed, resolved("p1", "c24", WALLS)];
}

function wallsOf(cues: readonly FxCue[]): Extract<FxCue, { kind: "walls" }>[] {
  return cues.filter((cue): cue is Extract<FxCue, { kind: "walls" }> => cue.kind === "walls");
}

describe("Patch v0.2.3c: Classic+ #24 Crushing Walls closes in", () => {
  it("the walls close in once, on the board, at the first card the play destroys, lasting D + their tail", () => {
    const D = 400;
    const results = planAll(wallsStream(4), fullBoardView(), envOf(), D);
    const destroyed = results.filter((result) => result.entry.events.some((event) => event.type === "destroyed"));
    expect(destroyed).toHaveLength(4);
    expect(destroyed.map((result) => wallsOf(result.cues).length)).toEqual([1, 0, 0, 0]);
    expect(wallsOf(results.flatMap((result) => result.cues))).toEqual([
      { kind: "walls", at: tid(testid.board), reach: FX_WALLS_REACH, delayMs: 0, durationMs: D + FX_WALLS_TAIL_MS },
    ]);
  });

  it("they hit with dust at both inner edges and a shake at FX_WALLS_HIT_AT of the cue, and each card's own death still plays", () => {
    const D = 400;
    const first = planAll(wallsStream(1), fullBoardView(), envOf(), D).find((result) =>
      result.entry.events.some((event) => event.type === "destroyed"),
    );
    const cues = first?.cues ?? [];
    const hit = Math.round(FX_WALLS_HIT_AT * (D + FX_WALLS_TAIL_MS));
    const dust = cues.filter((cue) => cue.kind === "burst" && cue.preset === "dust" && cue.delayMs === hit);
    expect(dust.map((cue) => (cue.kind === "burst" ? cue.at : null))).toEqual([
      { kind: "testid", testid: testid.board, at: { x: FX_WALLS_REACH, y: expect.any(Number) } },
      { kind: "testid", testid: testid.board, at: { x: 1 - FX_WALLS_REACH, y: expect.any(Number) } },
    ]);
    expect(cues.some((cue) => cue.kind === "shake" && cue.delayMs === hit)).toBe(true);
    // The row's own recipe for the death is kept: the walls add to it, never replace it.
    const plain = planAll([destroyedIn("p1", "w0")], fullBoardView(), envOf(), D)[0]?.cues ?? [];
    expect(plain.length).toBeGreaterThan(0);
    expect(cues.length).toBeGreaterThan(plain.length);
  });

  it("no walls when it destroys nothing, when it is countered, or for a card another play destroys", () => {
    const nothing = planAll(wallsStream(0), fullBoardView(), envOf());
    const countered = planAll(
      [played("p1", "c24", WALLS), { type: "countered", player: "p1", instanceId: "c24", defId: WALLS, byInstanceId: null, to: "graveyard" }, destroyedIn("p2", "w9")],
      fullBoardView(),
      envOf(),
    );
    const other = planAll([played("p1", "c5", "core-005"), destroyedIn("p2", "w9"), resolved("p1", "c5", "core-005")], fullBoardView(), envOf());
    for (const results of [nothing, countered, other]) expect(wallsOf(results.flatMap((result) => result.cues))).toEqual([]);
  });

  it("R202 a hidden play keys no walls, though its destroyed cards are public", () => {
    const results = planAll([played("p2", "hidden", "hidden"), destroyedIn("p1", "w0"), resolved("p2", "hidden", "hidden")], fullBoardView(), envOf());
    expect(wallsOf(results.flatMap((result) => result.cues))).toEqual([]);
  });
});

describe("R502 the per-card table", () => {
  it("R502 one table keys each recipe by definition, and every recipe it names exists", () => {
    expect(FX_CARD_TABLE.get(HINDER)).toEqual({ kind: "bespoke", key: "manaCrack" });
    expect(FX_CARD_TABLE.get(BLOOD)).toEqual({ kind: "bespoke", key: "bloodDrain" });
    expect(FX_CARD_TABLE.get(WALLS)).toEqual({ kind: "bespoke", key: "crushingWalls" });
    for (const [defId, entry] of FX_CARD_TABLE) {
      if (entry.kind === "bespoke") expect(typeof CARD_RECIPES[entry.key], defId).toBe("function");
      // R3332000: and every entrance it names, whatever the entry's kind.
      const key = entranceKeyOf(defId);
      if (key !== undefined) expect(typeof ENTRANCES[key], defId).toBe("function");
    }
  });

  it("R3332000 a Field Spell's own entrance plays as it lands face up and the shared Legendary entrance yields", () => {
    // Classic+ #78 Claude's Datacenter, a Legendary Field Spell, given an entrance of its own beside its hooks
    // and aura (R3402000: Combo-Index, the first example, has its own in the shipped table now, as R3389001
    // Classic's do; R3405001: and so does Meditative #40 Feng Shui).
    const FIELD = "classicplus-078";
    const table = parseFxCardTable("test.json5", `{ "${FIELD}": { cast: { motif: "conjure" }, aura: { look: "storm", scope: "zone" }, entrance: "titanBloom" } }`);
    expect(entranceKeyOf(FIELD, table)).toBe("titanBloom");
    expect(entranceKeyOf(FIELD)).toBeUndefined();
    const resolve: EntranceResolver = (defId) => {
      const def = CATALOG[defId];
      return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
    };
    const view = fullBoardView();
    const landing: GameEvent = { type: "summoned", player: "p1", instanceId: "f1", defId: FIELD, row: "backrow", lane: 1 };
    const events: GameEvent[] = [played("p1", "f1", FIELD), landing];
    const [entry] = entranceEntries(planEntries(events, withEvents(view, events), false), resolve, { seen: new Set() });
    if (entry === undefined) throw new Error("no entry");
    expect(entry.entrance?.tier).toBe("legendary");
    const env = envOf();
    env.memory.remember(entry.events);
    const D = entry.durationMs;
    const tgt = testid.zone("you", "backrow", 1);
    const own = planCardFx(landing, { entry, view, env, D }, table);
    expect(own).toEqual({ cues: ENTRANCES.titanBloom({ D, tgt, intensity: env.intensity }), row: "replace" });
    // The shared rays, gold burst, rings and shake yield to it; with no entrance of its own they play.
    expect(entranceCues(entry, env, D, table)).toEqual([]);
    expect(entranceCues(entry, env, D).map((cue) => cue.kind).sort()).toEqual(["burst", "rays", "ring", "ring", "shake"]);
    // Set face down (its fresh id's `formerId`, R227), it lands with no entrance, and the shared one stays.
    const set: GameEvent = { ...landing, instanceId: "f2", formerId: "f1" };
    expect(planCardFx(set, { entry: { ...entry, events: [set] }, view, env, D }, table)).toBeNull();
    // A Unit's row is no Field Spell's landing.
    expect(planCardFx({ ...landing, row: "units", lane: 3 }, { entry, view, env, D }, table)).toBeNull();
    // R202: the hidden sentinel keys nothing.
    expect(planCardFx({ ...landing, defId: "hidden", instanceId: "hidden" }, { entry, view, env, D }, table)).toBeNull();
  });

  it("R3407000 a MAGI's own entrance and Imitated Instrumentality's boot both draw on the summon they both claim", () => {
    const view = fullBoardView();
    const landing: GameEvent = { type: "summoned", player: "p1", instanceId: "m1", defId: "stinkyscape-018-1", row: "backrow", lane: 2 };
    const events: GameEvent[] = [played("p1", "ii", "stinkyscape-018"), landing];
    const entry: AnimationEntry = { ...planEntries(events, withEvents(view, events), false)[0]!, events };
    const D = entry.durationMs;
    const env = envOf();
    env.memory.remember([events[0]!]);
    const both = planCardFx(landing, { entry, view, env, D });
    // The entrance alone (no play resolving) and the boot alone (a table whose MELCHIOR has no entrance).
    const entrance = planCardFx(landing, { entry, view, env: envOf(), D });
    const table = new Map(FX_CARD_TABLE);
    table.set("stinkyscape-018-1", { kind: "hooks", hooks: {} });
    const boot = planCardFx(landing, { entry, view, env, D }, table);
    expect(entrance?.cues.length).toBeGreaterThan(0);
    expect(boot?.cues.length).toBeGreaterThan(0);
    expect(both).toEqual({ cues: [...(entrance?.cues ?? []), ...(boot?.cues ?? [])], row: "replace" });
  });

  it("R3407000 every other signature with an entrance and a recipe lands as before: its entrance alone, the recipe claiming nothing of its own summon", () => {
    const view = fullBoardView();
    const others = [...FX_CARD_TABLE].filter(([defId, entry]) => entry.kind === "bespoke" && entry.entrance !== undefined && !(STINKYSCAPE_TWO_SIGNATURES as readonly string[]).includes(defId));
    // R3389001: Classic's Field Spells (Palantir, InfiniScepter, Second Wind, In Too Deep) and Units among them.
    expect(others.map(([defId]) => defId)).toEqual(expect.arrayContaining(["classic-004", "classic-007", "classic-028", "classic-090"]));
    for (const [defId] of others) {
      const row = CATALOG[defId]?.type === "Unit" ? "units" : "backrow";
      const landing: GameEvent = { type: "summoned", player: "p1", instanceId: "own", defId, row, lane: 2 };
      const events: GameEvent[] = [played("p1", "own", defId), landing];
      const entry: AnimationEntry = { ...planEntries(events, withEvents(view, events), false)[0]!, events };
      const D = entry.durationMs;
      const env = envOf();
      env.memory.remember([events[0]!]);
      const alone = planCardFx(landing, { entry, view, env: envOf(), D });
      expect(alone?.cues.length, defId).toBeGreaterThan(0);
      expect(planCardFx(landing, { entry, view, env, D }), defId).toEqual(alone);
    }
  });

  it("R200 every cue the cast on draw and the per-card recipes plan starts inside its entry and ends within FX_MAX_TAIL_MS of its end, for short and long entries", () => {
    for (const D of [120, 200, 400, 732, 1400]) {
      const results = [
        ...planAll(hinderStream("p2", "p1"), fullBoardView(), envOf({ next: afterHinder(fullBoardView(), "you", 2) }), D),
        ...planAll(
          [drawn("p2", "c27", BLOOD), played("p2", "c27", BLOOD), { type: "radiantSet", instanceId: "hidden", defId: "hidden", zone: { z: "hand", player: "p2" } }, { type: "healthLost", player: "p2", amount: 7 }],
          fullBoardView(),
          envOf(),
          D,
        ),
        ...planAll(wallsStream(3), fullBoardView(), envOf(), D),
      ];
      const cues = results.flatMap((r) => r.cues);
      expect(wallsOf(cues)).toHaveLength(1);
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
    const results = [
      ...planAll(hinderStream("p2", "p1"), fullBoardView(), envOf({ intensity: 0, next: afterHinder(fullBoardView(), "you", 2) })),
      ...planAll(wallsStream(2), fullBoardView(), envOf({ intensity: 0 })),
    ];
    expect(results.flatMap((r) => r.cues)).toEqual([]);
  });
});

/* The real engine: both seats' redacted windows */

const DECK_A = [
  "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-008", "core-009", "core-010", "core-011",
  "core-012", "core-013", "core-014", "core-015", "core-016", "core-017", "core-018", "core-019", "core-020", "core-022",
  "core-023", "core-024", "core-025", "core-026", "core-028",
];
const DECK_B = [
  "core-021", "core-027", "core-030", "core-032", "core-033", "core-034", "core-035", "core-036", "core-037", "core-038",
  "core-040", "core-042", "core-043", "core-044", "core-045", "core-046", "core-047", "core-048", "core-051", "core-052",
  "core-053", "core-054", "core-055", "core-056", "core-057",
];

type Seats = { p1: PlayerView; p2: PlayerView };

/** The first seed whose opening turns cast `defId` on draw for p2 and leave both seats as `settled` wants. */
function castOnDrawGame(defId: string, settled: (seats: Seats) => boolean): Seats {
  const seats = castOnDrawViews("r502", [DECK_A, DECK_B], defId, settled);
  if (seats === null) throw new Error(`no seed casts ${defId} on draw in its opening turns`);
  return seats;
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

describe("R3389000 a signature's recipe outside its own play", () => {
  /** Each event of `events` the card recipes claim, planned as the layer plans them: remember, then plan. */
  function claims(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): { event: GameEvent; cues: FxCue[]; row: string }[] {
    return planEntries(events, withEvents(view, [...events]), false).flatMap((entry) => {
      env.memory.remember(entry.events);
      return entry.events.flatMap((event) => {
        const claim = planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs });
        return claim === null ? [] : [{ event, ...claim }];
      });
    });
  }
  const play = (player: PlayerId, instanceId: string, defId: string, costPaid = 2): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid });

  it("R3389000 a signature's recipe claims nested casts and events naming it; a hidden source keys none", () => {
    const view = fullBoardView();
    // The memory lists every open play, outermost first, each a copy.
    const env = envOf();
    env.memory.remember([play("p1", "tyr", "classic-056", 4), play("p1", "c1", "core-016", 0)]);
    expect(env.memory.plays().map((open) => open.instanceId)).toEqual(["tyr", "c1"]);
    expect(env.memory.plays().at(-1)).toEqual(env.memory.resolving());
    (env.memory.plays()[0]?.seen as Record<string, number>).cardPlayed = 99;
    expect(env.memory.plays()[0]?.seen.cardPlayed).toBe(1);

    // (a) A cast Spell Tyrant's play makes is its recipe's: the cast's own entry draws nothing of it.
    const cast = claims([play("p1", "tyr", "classic-056", 4), play("p1", "c1", "core-016", 0), { type: "exiled", instanceId: "c1", defId: "core-016", owner: "p1" }], view);
    expect(cast.map((claim) => [claim.event.type, claim.row])).toEqual([
      ["cardPlayed", "keep"],
      ["exiled", "replace"],
    ]);
    // Around a hidden play, or a play whose card has no signature recipe (Recycle's is no signature), nothing claims it.
    expect(claims([play("p2", "hidden", "hidden"), play("p2", "c2", "core-016", 0), { type: "exiled", instanceId: "c2", defId: "core-016", owner: "p2" }], view)).toEqual([]);
    expect(FX_CARD_TABLE.get("classic-030")).toEqual({ kind: "bespoke", key: "recycleLoop" });
    expect(claims([play("p1", "rec", "classic-030"), play("p1", "c3", "core-016", 0), { type: "shuffledIn", player: "p1", instanceId: "s1", defId: "core-003", position: 0 }], view)).toEqual([]);

    // (b) An event naming its card as the source: a redirect's, a counter's, an Activate's, a quest's.
    expect(sourceIdOf({ type: "redirected", what: "target", fromId: "a", toId: "j", byInstanceId: "j" })).toBe("j");
    expect(sourceIdOf({ type: "countered", player: "p2", instanceId: "s", defId: "core-016", byInstanceId: null, to: "graveyard" })).toBeNull();
    expect(sourceIdOf({ type: "activated", player: "p1", instanceId: "rod", defId: "classic-007", ability: "activate" })).toBe("rod");
    expect(sourceIdOf({ type: "questCompleted", player: "p1", instanceId: "deep", quest: "q1" })).toBe("deep");
    expect(sourceIdOf({ type: "damage", sourceId: "x", targetId: "y", amount: 1, combat: false })).toBeNull();
    // Found face up on the board, or in a readable landing the memory holds, as a hook of its own.
    const deep = fullBoardView();
    const field = deep.you.backrow[0];
    if (field === null || field === undefined || field.faceDown) throw new Error("a face-up Field Spell");
    deep.you.backrow[0] = { ...field, instanceId: "deep", defId: "classic-090" };
    expect(signaturePlayOf("deep", deep, createFxMemory())).toEqual({ player: "p1", instanceId: "deep", defId: "classic-090", castOnDraw: false, step: 0, seen: {}, hook: "trigger" });
    const quest = claims([{ type: "questProgressed", player: "p1", instanceId: "deep", quest: "q1", progress: 1, goal: 3 }], deep);
    expect(quest.map((claim) => claim.event.type)).toEqual(["questProgressed"]);
    const remembered = createFxMemory();
    remembered.remember([{ type: "summoned", player: "p2", instanceId: "joro", defId: "classic-033", row: "units", lane: 2 }]);
    expect(signaturePlayOf("joro", view, remembered)?.defId).toBe("classic-033");
    const aim = view.opponent.units[0]?.instanceId ?? "";
    const web = claims([play("p1", "hit", "core-016"), { type: "summoned", player: "p2", instanceId: "joro", defId: "classic-033", row: "units", lane: 3 }, { type: "redirected", what: "target", fromId: aim, toId: "joro", byInstanceId: "joro" }], view);
    expect(web.map((claim) => claim.event.type)).toEqual(["summoned", "redirected"]);

    // R202: a hidden source, a face-down card and a sentinel landing key none.
    expect(signaturePlayOf("hidden", view, remembered)).toBeUndefined();
    const sentinel = createFxMemory();
    sentinel.remember([{ type: "trapFired", instanceId: "tax", defId: "hidden", controller: "p2", row: "backrow", lane: 2 }]);
    expect(signaturePlayOf("tax", view, sentinel)).toBeUndefined();
    expect(claims([{ type: "redirected", what: "target", fromId: aim, toId: "hidden", byInstanceId: "hidden" }], view)).toEqual([]);
    expect(claims([{ type: "activated", player: "p2", instanceId: "nobody", defId: "hidden", ability: "activate" }], view)).toEqual([]);
    expect(JSON.stringify([...cast, ...quest, ...web].map((claim) => claim.cues))).not.toMatch(/classic-\d{3}|hidden/);

    // (c) A whole pile's one entry: Second Wind's backs fly beside the wave; another card's play draws nothing there.
    const wind = createFxMemory();
    wind.remember([play("p1", "wind", "classic-028", 0)]);
    const exiles = [0, 1, 2].map((k): GameEvent => ({ type: "exiled", instanceId: `deck-${String(k)}`, defId: "core-003", owner: "p1" }));
    const zone: AnimationEntry = { events: exiles, type: "exiled", durationMs: 600, frames: new Map(), view, zone: { side: "you", pile: "library", count: 3, events: 3 } };
    wind.remember(exiles);
    const backs = planSignatureZone(zone, view, envOf({ memory: wind }));
    expect(backs.filter((cue) => cue.kind === "ghost")).toHaveLength(3);
    const other = createFxMemory();
    other.remember([play("p1", "c4", "core-016"), ...exiles]);
    expect(planSignatureZone(zone, view, envOf({ memory: other }))).toEqual([]);
    expect(planSignatureZone({ ...zone, zone: undefined }, view, envOf({ memory: wind }))).toEqual([]);
  });
});
