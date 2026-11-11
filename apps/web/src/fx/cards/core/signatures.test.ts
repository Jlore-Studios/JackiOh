// R3402001: Core's ten signatures of issue #3402 (fx/cards/core/), each planned over its /dev/fx gallery
// play the way FxLayer plans it (`planFx`: its entrance, its recipe and its rows), action by action as a
// game deals them (a Trap's firing, a trigger and an Activate each an action of its own), each asserted
// beat by beat against its concept. R202: the same plays by the other seat, with what R97 hides from the
// viewer hidden, plan no cue that names a card or anchors on a hand card, and a Trap set face down plans
// nothing until it fires.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid, entranceEntries, planEntries, setPieceEntries, takeoverClaim, type AnimationEntry, type EntranceResolver } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import {
  FX_CENTER,
  FX_CLAMP_TRAUMA,
  FX_CRUCIBLE_TRAUMA,
  FX_GRADE_TRAUMA,
  FX_INTENSITY_SCALE,
  FX_PAWN_STRINGS_MAX,
  FX_PAWN_TRAUMA,
  FX_POCKET_TRAUMA,
  FX_REELS_MS,
  FX_REELS_TRAUMA,
  FX_SIGNATURE_BURSTS_MAX,
  FX_TAKEOVER_TRAUMA,
} from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryEntry, galleryFieldView, galleryPlayId, galleryPlayOf } from "../../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE, isSignature } from "../tables.ts";
import { SILAS_SIDE_TINTS } from "./sillySilas.ts";

const I = FX_INTENSITY_SCALE.normal;
const HIDDEN = "hidden";
/** The ten, in catalog order: #52 Silly Silas to #99 Craft a Card. */
const CORE_SIGNATURES = ["core-052", "core-083", "core-085", "core-087", "core-093", "core-095", "core-096", "core-097", "core-098", "core-099"] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const CENTRE = tid(testid.board, FX_CENTER);

function envOf(): FxPlanEnv {
  return { intensity: I, card: () => undefined, memory: createFxMemory() };
}

/** As the board resolves an entrance: the card's type and rarity, a token's the one it prints (B2.5). */
const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

/** A gallery play's events split into the actions a game deals them in (tiers.test.ts' `actionsOf`). */
function actionsOf(events: readonly GameEvent[]): GameEvent[][] {
  const actions: GameEvent[][] = [];
  for (const event of events) {
    const last = actions[actions.length - 1];
    const marked = event.type === "hookResolving" && event.hook === "activate" && last?.[0]?.type === "activated" && last[0].instanceId === event.sourceId;
    const later = event.type === "trapFired" || event.type === "activated" || (event.type === "hookResolving" && event.hook !== "cry" && !marked);
    if (last === undefined || (later && last.length > 0)) actions.push([event]);
    else last.push(event);
  }
  return actions;
}

type Planned = { entry: AnimationEntry; cues: FxCue[] };

/** `events` over `view`, entered and held as the queue holds them at speed 1 with Big moments full, then planned entry by entry. */
function planOut(events: readonly GameEvent[], view: PlayerView): Planned[] {
  const env = envOf();
  const seen = new Set<string>();
  const out: Planned[] = [];
  for (const action of actionsOf(events)) {
    const entered = entranceEntries(planEntries(action, view, false, undefined, hasSetPiece), resolve, { seen });
    for (const entry of setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 })) {
      env.memory.remember(entry.events);
      out.push({ entry, cues: planFx(entry, entry.view, env) });
    }
  }
  return out;
}

/**
 * The view `defId` plays over: its gallery board, a Field Spell standing face up in the viewer's first
 * backrow zone under the play's own id, as it stands in a game once it lands (the gallery's keeps the
 * fixture's id, so there its later beats fall back to its controller's hero).
 */
function fieldOf(defId: string, board: PlayerView, id: string = galleryPlayId(1)): PlayerView {
  const view = galleryFieldView(defId, board);
  const first = view.you.backrow[0];
  if (CATALOG[defId]?.type !== "Field Spell" || first === null || first === undefined || first.faceDown) return view;
  return { ...view, you: { ...view.you, backrow: [{ ...first, instanceId: id }, ...view.you.backrow.slice(1)] } };
}

/** `defId`'s gallery play by the viewer, over its gallery board, and its events. */
function galleryPlay(defId: string): { planned: Planned[]; events: GameEvent[]; board: PlayerView } {
  const entry = FX_CARD_TABLE.get(defId);
  const play = entry === undefined ? null : galleryPlayOf(defId, entry);
  if (play === null) throw new Error(`${defId} has no gallery play`);
  const board = galleryBoardOf(defId, fullBoardView());
  const events = galleryCardEvents(defId, play, board, 1);
  return { planned: planOut(events, withEvents(fieldOf(defId, board), events)), events, board };
}

/** The cues of the entry whose events hold one of `type`. */
function at(planned: readonly Planned[], type: GameEvent["type"]): FxCue[] {
  return planned.filter((p) => p.entry.events.some((e) => e.type === type)).flatMap((p) => p.cues);
}

const all = (planned: readonly Planned[]): FxCue[] => planned.flatMap((p) => p.cues);
const kinds = (cues: readonly FxCue[], kind: FxCue["kind"]): FxCue[] => cues.filter((cue) => cue.kind === kind);
const shakes = (cues: readonly FxCue[]): number[] => cues.flatMap((cue) => (cue.kind === "shake" ? [cue.trauma] : []));
const anchored = (cues: readonly FxCue[], anchor: FxAnchor): FxCue[] => cues.filter((cue) => "at" in cue && JSON.stringify(cue.at) === JSON.stringify(anchor));

describe("R3402001 Core's signatures", () => {
  it("R3402001 the ten are signatures, each planning over its gallery play", () => {
    for (const defId of CORE_SIGNATURES) {
      expect(isSignature(FX_CARD_TABLE.get(defId)), defId).toBe(true);
      expect(all(galleryPlay(defId).planned).length, defId).toBeGreaterThan(0);
    }
  });

  it("R3402001 Silly Silas lands in its spotlight and its Cry shuffles the field", () => {
    const { planned, board } = galleryPlay("core-052");
    const landing = at(planned, "summoned");
    // He pirouettes down from the top of the screen, cracks a crater into his zone and tips his hat.
    expect(landing.find((cue) => cue.kind === "shot")).toMatchObject({ from: { kind: "viewport", at: { x: 0.5, y: 0 } }, shot: { motion: "arc", turn: "spin" } });
    expect(kinds(landing, "crack").length).toBe(1);
    expect(shakes(landing)).toContain(Math.min(1, ENTRANCE_TRAUMA.silasSpotlight * I));
    // On `rotated` a mirrorball ring and confetti spin at the centre, a sheen on each occupied unit zone, capped.
    const spin = at(planned, "rotated");
    expect(anchored(spin, CENTRE).map((cue) => (cue.kind === "ring" || cue.kind === "burst" ? cue.preset : cue.kind))).toEqual(["prismatic", "confetti"]);
    const occupied = [...board.you.units, ...board.opponent.units].filter((unit) => unit !== null).length;
    expect(kinds(spin, "sheen").length).toBe(Math.min(occupied, FX_SIGNATURE_BURSTS_MAX));
    // Each card crossing sides flips its border to its new owner's colour on `controlChanged`.
    const crossed = at(planned, "controlChanged").filter((cue) => cue.kind === "brand");
    expect(crossed).toEqual([{ kind: "brand", at: tid(testid.zone("you", "units", 1)), tint: SILAS_SIDE_TINTS.you, delayMs: 0, durationMs: expect.any(Number) as number }]);
  });

  it("R3402001 Transmogulate's crucible tips over the caster's hero and its wave washes the piles", () => {
    const { planned } = galleryPlay("core-083");
    const hero = tid(testid.hero("you"));
    const cast = at(planned, "cardPlayed");
    expect(anchored(cast, hero).find((cue) => cue.kind === "rays")).toMatchObject({ tone: "legendary" });
    expect(anchored(cast, hero).some((cue) => cue.kind === "burst" && cue.preset === "prismatic")).toBe(true);
    expect(shakes(cast)).toContain(Math.min(1, FX_CRUCIBLE_TRAUMA * I));
    // R3402000: its own cast is its landing; no shared Legendary rays.
    expect(cast.filter((cue) => cue.kind === "rays")).toHaveLength(1);
    const wave = at(planned, "transformed");
    for (const pile of [animTestid.library("you"), animTestid.hand("you")]) expect(anchored(wave, tid(pile)).map((cue) => cue.kind)).toEqual(["ring", "sheen"]);
    expect(wave.some((cue) => cue.kind === "sheen" && "at" in cue && cue.at.kind === "testid" && cue.at.testid.startsWith("hand-card-"))).toBe(true);
  });

  it("R3402001 Transmogulate Embiggened re-forms each Unit in a ripple", () => {
    const view = fullBoardView();
    const mine = view.you.units[0];
    if (mine === null || mine === undefined) throw new Error("no unit");
    const events: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "fx-t", defId: "core-083", costPaid: 4 },
      { type: "transformed", instanceId: mine.instanceId, fromDefId: mine.defId, toDefId: "core-052", newInstanceId: mine.instanceId },
    ];
    const ripple = anchored(at(planOut(events, withEvents(view, events)), "transformed"), tid(testid.card(mine.instanceId)));
    expect(ripple.map((cue) => (cue.kind === "ring" ? `ring ${cue.preset}` : cue.kind))).toEqual(["ring prismatic", "sheen"]);
  });

  it("R3402001 Unlicensed Experimentation bursts open at its firing and welds the seized permanent", () => {
    const { planned, events } = galleryPlay("core-085");
    const fired = events.find((e): e is Extract<GameEvent, { type: "trapFired" }> => e.type === "trapFired");
    if (fired === undefined) throw new Error("no firing");
    const hatch = tid(testid.zone("you", fired.row, fired.lane));
    const firing = at(planned, "trapFired");
    // Its own beats come first; the firing's own row (the trap's reveal) still plays after them.
    expect(anchored(firing, hatch).map((cue) => (cue.kind === "burst" ? cue.preset : cue.kind)).slice(0, 4)).toEqual(["fracture", "poison", "smoke", "spark"]);
    // R3402000: its own firing replaces the shared Legendary rays.
    expect(firing.some((cue) => cue.kind === "rays")).toBe(false);
    const fused = events.find((e): e is Extract<GameEvent, { type: "fused" }> => e.type === "fused");
    if (fused === undefined) throw new Error("no fuse");
    const weld = at(planned, "fused");
    expect(anchored(weld, tid(testid.card(fused.instanceIds[0] ?? ""))).map((cue) => cue.kind === "ring" && cue.preset)).toEqual(["poison"]);
    expect(anchored(weld, tid(testid.card(fused.resultInstanceId))).map((cue) => (cue.kind === "ring" || cue.kind === "burst" ? `${cue.kind} ${cue.preset}` : cue.kind))).toEqual([
      "ring spark",
      "crack",
      "sheen",
      "burst spark",
      "burst poison",
    ]);
    expect(shakes(weld)).toContain(Math.min(1, FX_CLAMP_TRAUMA * I));
  });

  it("R3402001 Pocket Chaos opens its pocket and trades each pair through it", () => {
    const { planned } = galleryPlay("core-087");
    const cast = at(planned, "cardPlayed");
    expect(anchored(cast, CENTRE).some((cue) => cue.kind === "burst" && cue.preset === "confetti")).toBe(true);
    // Both heroes' light tugged into it.
    expect(cast.filter((cue) => cue.kind === "shot").map((cue) => cue.kind === "shot" && [cue.from, cue.to])).toEqual([
      [tid(testid.hero("you")), CENTRE],
      [tid(testid.hero("opponent")), CENTRE],
    ]);
    expect(shakes(cast)).toContain(Math.min(1, FX_POCKET_TRAUMA * I));
    // The other player's card drops into their hand pile as a card back.
    expect(at(planned, "addedToHand").filter((cue) => cue.kind === "ghost")).toEqual([{ kind: "ghost", from: CENTRE, to: tid(animTestid.hand("opponent")), delayMs: 0, durationMs: expect.any(Number) as number }]);
    // Each of the three pairs trades places: card backs crossing for the boards and Deck piles, rings and sheens for the heroes' health.
    const ends = { board: [testid.zone("you", "units", 3), testid.zone("opponent", "units", 3)], library: [animTestid.library("you"), animTestid.library("opponent")], health: [testid.hero("you"), testid.hero("opponent")] } as const;
    for (const what of ["board", "library", "health"] as const) {
      const events: GameEvent[] = [{ type: "cardPlayed", player: "p1", instanceId: "fx-p", defId: "core-087", costPaid: 3 }, { type: "swapped", what }];
      const swap = at(planOut(events, withEvents(fullBoardView(), events)), "swapped");
      const [a, b] = ends[what];
      if (what === "health") {
        for (const end of [a, b]) expect(anchored(swap, tid(end)).map((cue) => cue.kind), what).toEqual(["ring", "sheen"]);
      } else {
        expect(swap.filter((cue) => cue.kind === "ghost").map((cue) => cue.kind === "ghost" && [cue.from, cue.to]), what).toEqual([
          [tid(a), tid(b)],
          [tid(b), tid(a)],
        ]);
      }
      expect(shakes(swap), what).toContain(Math.min(1, FX_POCKET_TRAUMA * I));
    }
  });

  it("R3402001 Combo-Index raises its stadium and lights a grade on each trigger", () => {
    const { planned } = galleryPlay("core-093");
    const stadium = tid(testid.card(galleryPlayId(1)));
    const landing = at(planned, "summoned");
    expect(anchored(landing, tid(testid.zone("you", "backrow", 1))).find((cue) => cue.kind === "rays")).toMatchObject({ tone: "radiant" });
    expect(landing.filter((cue) => cue.kind === "ring" && cue.preset === "frost")).toHaveLength(3);
    expect(shakes(landing)).toContain(Math.min(1, ENTRANCE_TRAUMA.comboStadium * I));
    // Its trigger is an action of its own: as the grade rises (the engine's `counterChanged` of it,
    // combo_index.rs) the next ring lights in storm blue, and its grade's card flies into the hand pile.
    const grade = at(planned, "counterChanged");
    // Its rows still play beside it (`row: "keep"`).
    expect(anchored(grade, stadium).map((cue) => (cue.kind === "ring" || cue.kind === "burst" ? `${cue.kind} ${cue.preset}` : cue.kind))).toEqual(expect.arrayContaining(["ring frost", "burst sparkle"]));
    expect(shakes(grade)).toContain(Math.min(1, FX_GRADE_TRAUMA * I));
    expect(at(planned, "addedToHand").filter((cue) => cue.kind === "ghost")).toEqual([{ kind: "ghost", from: stadium, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: expect.any(Number) as number }]);
    // Each other grade draws its own beat: tags, a burn at the other pile, a spark, a beam.
    const id = "fx-stadium";
    const view = fieldOf("core-093", fullBoardView(), id);
    const foe = view.opponent.units[0]?.instanceId ?? "";
    const grades: GameEvent[] = [
      { type: "hookResolving", sourceId: id, defId: "core-093", controller: "p1", hook: "endOfTurn" },
      { type: "costChanged", instanceId: "hc1", cost: 0 },
      { type: "exiled", instanceId: HIDDEN, defId: HIDDEN, owner: "p2" },
      { type: "radiantSet", instanceId: HIDDEN, defId: HIDDEN, zone: { z: "hand", player: "p1" } },
      { type: "damage", sourceId: id, targetId: foe, amount: 2, combat: false },
    ];
    const rest = all(planOut(grades, withEvents(view, grades)));
    expect(anchored(rest, tid(animTestid.hand("you"))).some((cue) => cue.kind === "burst" && cue.preset === "gold")).toBe(true);
    expect(anchored(rest, tid(animTestid.hand("opponent"))).some((cue) => cue.kind === "burst" && cue.preset === "fire")).toBe(true);
    expect(anchored(rest, tid(testid.card(id))).some((cue) => cue.kind === "burst" && cue.preset === "spark")).toBe(true);
    expect(rest.find((cue) => cue.kind === "shot")).toMatchObject({ shot: { motion: "beam" }, from: tid(testid.card(id)), to: tid(testid.card(foe)) });
    // Only its own grade's rise lights a ring: not another card's grade in its trigger, nor another counter
    // of its own, nor the grade E its play writes as it arrives, which is no trigger's.
    const marker: GameEvent = { type: "hookResolving", sourceId: id, defId: "core-093", controller: "p1", hook: "endOfTurn" };
    const ringsOf = (events: GameEvent[]): FxCue[] =>
      anchored(all(planOut(events, withEvents(view, events))), tid(testid.card(id))).filter((cue) => cue.kind === "ring" && cue.preset === "frost");
    expect(ringsOf([marker, { type: "counterChanged", instanceId: id, counter: "grade", value: 3 }])).toHaveLength(1);
    expect(ringsOf([marker, { type: "counterChanged", instanceId: foe, counter: "grade", value: 3 }])).toEqual([]);
    expect(ringsOf([marker, { type: "counterChanged", instanceId: id, counter: "plague", value: 1 }])).toEqual([]);
    expect(ringsOf([{ type: "cardPlayed", player: "p1", instanceId: id, defId: "core-093", costPaid: 3 }, { type: "counterChanged", instanceId: id, counter: "grade", value: 1 }])).toEqual([]);
  });

  it("R3402001 Call to Chaos spins its reels and slams them to the roll", () => {
    const { planned } = galleryPlay("core-095");
    const cast = at(planned, "cardPlayed");
    expect(cast.find((cue) => cue.kind === "rays")).toMatchObject({ tone: "legendary", at: CENTRE });
    const reels = cast.filter((cue) => cue.kind === "burst" && cue.preset === "spark");
    expect(reels.map((cue) => "at" in cue && cue.at.kind === "testid" && cue.at.at?.x)).toEqual([0.4, 0.5, 0.6]);
    expect(reels.map((cue) => cue.delayMs)).toEqual([...reels.map((cue) => cue.delayMs)].sort((a, b) => a - b));
    // Its roll's reveal stands FX_REELS_MS, shown to both seats, with a gold ring and the jackpot's shake.
    const roll = at(planned, "chaosRolled");
    expect(roll.find((cue) => cue.kind === "chaos")).toMatchObject({ durationMs: FX_REELS_MS });
    expect(roll.some((cue) => cue.kind === "ring" && cue.preset === "gold")).toBe(true);
    expect(shakes(roll)).toContain(Math.min(1, FX_REELS_TRAUMA * I));
  });

  it("R3402001 My Pawn takes over at its firing, strings every enemy piece and shatters the attack", () => {
    const { planned, board } = galleryPlay("core-096");
    const firing = at(planned, "trapFired");
    expect(firing.find((cue) => cue.kind === "takeover")).toMatchObject({ look: "pawn" });
    const strings = firing.filter((cue) => cue.kind === "shot" && cue.from.kind === "viewport");
    const pieces = board.opponent.units.filter((unit) => unit !== null).length + 1;
    expect(strings.length).toBe(Math.min(pieces, FX_PAWN_STRINGS_MAX));
    expect(strings.some((cue) => cue.kind === "shot" && JSON.stringify(cue.to) === JSON.stringify(tid(testid.hero("opponent"))))).toBe(true);
    expect(anchored(firing, tid(testid.hero("opponent"))).some((cue) => cue.kind === "sheen")).toBe(true);
    // R3400000: its own cast keeps its takeover alone, without the shared takeover's shake; its strings stand in.
    expect(shakes(firing)).not.toContain(Math.min(1, FX_TAKEOVER_TRAUMA * I));
    const foe = board.opponent.units[0]?.instanceId ?? "";
    const shatter = anchored(at(planned, "attackCancelled"), tid(testid.card(foe)));
    expect(shatter.map((cue) => (cue.kind === "ring" || cue.kind === "burst" ? `${cue.kind} ${cue.preset}` : cue.kind)).slice(0, 3)).toEqual(["crack", "ring frost", "burst shard"]);
    expect(shakes(at(planned, "attackCancelled"))).toContain(Math.min(1, FX_PAWN_TRAUMA * I));
  });

  it("R3402001 Zephrys uncoils over its silk and gusts the wish into the caster's hand", () => {
    const { planned } = galleryPlay("core-097");
    const cast = at(planned, "cardPlayed");
    expect(cast.find((cue) => cue.kind === "takeover")).toMatchObject({ look: "genie" });
    expect(anchored(cast, CENTRE).some((cue) => cue.kind === "burst" && cue.preset === "sparkle")).toBe(true);
    expect(anchored(cast, CENTRE).some((cue) => cue.kind === "ring" && cue.preset === "frost")).toBe(true);
    // One card back streaks on a gust from the board's far side into the caster's hand pile.
    expect(at(planned, "addedToHand").filter((cue) => cue.kind === "ghost")).toEqual([
      { kind: "ghost", from: tid(testid.hero("opponent")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: expect.any(Number) as number },
    ]);
  });

  it("R3402001 Heroic Power lands as a pedestal and launches each Activate's power", () => {
    const { planned } = galleryPlay("core-098");
    const zone = tid(testid.zone("you", "backrow", 1));
    const pedestal = tid(testid.card(galleryPlayId(1)));
    const landing = at(planned, "summoned");
    expect(landing.find((cue) => cue.kind === "takeover")).toMatchObject({ look: "rotunda" });
    expect(anchored(landing, zone).find((cue) => cue.kind === "rays")).toMatchObject({ tone: "holy" });
    expect(anchored(landing, zone).some((cue) => cue.kind === "burst" && cue.preset === "holy")).toBe(true);
    expect(anchored(landing, tid(testid.zone("you", "backrow", 1), { x: 0.5, y: 0.2 })).some((cue) => cue.kind === "burst" && cue.preset === "sparkle")).toBe(true);
    // Each Activate flares the pedestal and launches its power: the gallery's draws a card back into the hand pile.
    const flare = at(planned, "activated");
    expect(anchored(flare, pedestal).map((cue) => (cue.kind === "ring" || cue.kind === "burst" ? `${cue.kind} ${cue.preset}` : cue.kind)).slice(0, 2)).toEqual(["ring holy", "burst holy"]);
    // Beside the draw's own flight from the Deck pile.
    expect(at(planned, "drawn").filter((cue) => cue.kind === "ghost")).toContainEqual({ kind: "ghost", from: pedestal, to: tid(animTestid.hand("you")), delayMs: 0, durationMs: expect.any(Number) as number });
    // A dart to a `damage`'s target, a sprinter to a summoned Unit's zone, a card back into the hand pile on a draw.
    const id = "fx-pedestal";
    const view = fieldOf("core-098", fullBoardView(), id);
    view.you.units[1] = null;
    // In the engine's order: `activated` first, then the hook's marker (activate.rs), as a game deals them.
    const powers: GameEvent[] = [
      { type: "activated", player: "p1", instanceId: id, defId: "core-098", ability: "power" },
      { type: "hookResolving", sourceId: id, defId: "core-098", controller: "p1", hook: "activate" },
      { type: "damage", sourceId: id, targetId: "hero-p2", amount: 2, combat: false },
      { type: "summoned", player: "p1", instanceId: "fx-sprinter", defId: "core-003", row: "units", lane: 2 },
      { type: "drawn", player: "p1", instanceId: "fx-drawn", defId: "core-003" },
    ];
    const rest = all(planOut(powers, withEvents(view, powers)));
    expect(rest.filter((cue) => cue.kind === "ring" && cue.preset === "holy" && JSON.stringify(cue.at) === JSON.stringify(tid(testid.card(id))))).toHaveLength(1);
    // Its `activated` alone, no hook of it open yet (the engine's marker comes after it), still flares the
    // pedestal, in place of the shared arcane flare; another card's Activate flares nothing of it.
    const alone: GameEvent[] = [powers[0] as GameEvent];
    const flared = all(planOut(alone, withEvents(view, alone)));
    expect(flared.filter((cue) => cue.kind === "ring").map((cue) => cue.kind === "ring" && `${cue.preset} ${JSON.stringify(cue.at)}`)).toEqual([`holy ${JSON.stringify(tid(testid.card(id)))}`]);
    const other: GameEvent[] = [{ type: "activated", player: "p1", instanceId: "fx-other", defId: "core-004", ability: "power" }];
    expect(all(planOut(other, withEvents(view, other))).some((cue) => cue.kind === "ring" && cue.preset === "holy")).toBe(false);
    expect(rest.find((cue) => cue.kind === "shot" && cue.preset === "fire")).toMatchObject({ from: tid(testid.card(id)), to: tid(testid.hero("opponent")) });
    expect(rest.some((cue) => cue.kind === "shot" && cue.preset === "dust" && JSON.stringify(cue.from) === JSON.stringify(tid(testid.card(id))))).toBe(true);
    expect(rest.some((cue) => cue.kind === "ghost" && JSON.stringify(cue.to) === JSON.stringify(tid(animTestid.hand("you"))))).toBe(true);
  });

  it("R3402001 Craft a Card forges its two Units", () => {
    const { planned } = galleryPlay("core-099");
    const cast = at(planned, "cardPlayed");
    expect(cast.find((cue) => cue.kind === "takeover")).toMatchObject({ look: "crystalForge" });
    // The two halves ride the rails in from the board's edges as card backs and meet at the anvil.
    expect(cast.filter((cue) => cue.kind === "ghost").map((cue) => cue.kind === "ghost" && cue.to)).toEqual([CENTRE, CENTRE]);
    expect(anchored(cast, CENTRE).some((cue) => cue.kind === "burst" && cue.preset === "frost")).toBe(true);
    // The hand pile rings as the card is added, and on `fused` the card flies from the forge into it.
    const hand = tid(animTestid.hand("you"));
    expect(anchored(at(planned, "addedToHand"), hand).map((cue) => cue.kind)).toContain("ring");
    expect(at(planned, "fused").filter((cue) => cue.kind === "ghost")).toEqual([{ kind: "ghost", from: CENTRE, to: hand, delayMs: 0, durationMs: expect.any(Number) as number }]);
  });

  it("R3402001 the follow-up beats one entry draws are capped at FX_SIGNATURE_BURSTS_MAX", () => {
    // A run the queue plays as one entry (a rotation handing many cards over at once) brands only the first few.
    const view = fullBoardView();
    const units = [...view.you.units, ...view.opponent.units].filter((unit) => unit !== null);
    expect(units.length).toBeGreaterThan(FX_SIGNATURE_BURSTS_MAX);
    const changes: GameEvent[] = units.map((unit, k): GameEvent => ({ type: "controlChanged", instanceId: unit.instanceId, controller: k % 2 === 0 ? "p1" : "p2", row: "units", lane: (k % 5) + 1 }));
    const env = envOf();
    env.memory.remember([{ type: "cardPlayed", player: "p1", instanceId: "fx-s", defId: "core-052", costPaid: 5 }]);
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const entry = { ...galleryEntry(changes, view), durationMs: 400 };
    const brands = changes.flatMap((event) => CARD_RECIPES.mirrorballShuffle(event, { entry, view, env, D: 400 }, play)?.cues ?? []).filter((cue) => cue.kind === "brand");
    expect(brands).toHaveLength(FX_SIGNATURE_BURSTS_MAX);
  });
});

/** R3402001: each signature Spell's own cast, told apart by the beat only it draws (the tests above). */
const OWN_CAST: Readonly<Record<string, (cues: readonly FxCue[]) => boolean>> = {
  "core-083": (cues) => anchored(cues, tid(testid.hero("you"))).some((cue) => cue.kind === "rays"),
  "core-087": (cues) => anchored(cues, CENTRE).some((cue) => cue.kind === "burst" && cue.preset === "confetti"),
  "core-095": (cues) => cues.filter((cue) => cue.kind === "burst" && cue.preset === "spark").length === 3,
  "core-097": (cues) => anchored(cues, CENTRE).some((cue) => cue.kind === "ring" && cue.preset === "frost"),
  "core-099": (cues) => cues.filter((cue) => cue.kind === "ghost" && JSON.stringify(cue.to) === JSON.stringify(CENTRE)).length === 2,
};

/** R3402001: each signature Spell's text beat, the one its gallery play's later events draw (the tests above). */
const TEXT_BEAT: Readonly<Record<string, (cues: readonly FxCue[]) => boolean>> = {
  "core-083": (cues) => anchored(cues, tid(animTestid.library("you"))).some((cue) => cue.kind === "ring"),
  "core-087": (cues) => cues.some((cue) => cue.kind === "ghost" && JSON.stringify(cue.to) === JSON.stringify(tid(animTestid.hand("opponent")))),
  "core-095": (cues) => cues.some((cue) => cue.kind === "ring" && cue.preset === "gold"),
  "core-097": (cues) => cues.some((cue) => cue.kind === "ghost" && JSON.stringify(cue.from) === JSON.stringify(tid(testid.hero("opponent")))),
  "core-099": (cues) => cues.some((cue) => cue.kind === "ghost" && JSON.stringify(cue.from) === JSON.stringify(CENTRE) && JSON.stringify(cue.to) === JSON.stringify(tid(animTestid.hand("you")))),
};

describe("R3402001 R2722002 a Cloaked signature Spell of the caster's own", () => {
  it("R3402001 R2722002 sets face down with no cast of its own, and casts it once at its reveal, its text's beats with it", () => {
    const CLOAKED = "cloaked-1";
    for (const [defId, ownCast] of Object.entries(OWN_CAST)) {
      const entry = FX_CARD_TABLE.get(defId);
      const play = entry === undefined ? null : galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      const board = galleryBoardOf(defId, fullBoardView());
      const events = galleryCardEvents(defId, play, board, 1);
      const cast = all(planOut(events, withEvents(board, events)));
      expect(ownCast(cast), `${defId} cast`).toBe(true);
      const textBeat = TEXT_BEAT[defId];
      if (textBeat === undefined) throw new Error(`${defId} has no text beat`);
      expect(textBeat(cast), `${defId} text`).toBe(true);
      // R227: its set, on its controller's seat, a fresh id whose `formerId` is the hand card's.
      const set: GameEvent[] = [
        { type: "cardPlayed", player: "p1", instanceId: CLOAKED, defId, costPaid: 1, formerId: galleryPlayId(1) },
        { type: "cardResolved", player: "p1", instanceId: CLOAKED, defId, permanent: true, costPaid: 1 },
      ];
      const setCues = all(planOut(set, withEvents(board, set)));
      expect(ownCast(setCues), `${defId} set`).toBe(false);
      expect(setCues.filter((cue) => cue.kind === "shake" || cue.kind === "rays" || cue.kind === "takeover"), `${defId} set`).toEqual([]);
      // R783006: its Activate: Reveal, then the firing that casts it, then, as the engine runs it
      // (cry_trigger.rs `reveal_set_spell`), its Cry's own marker and the cast's own beats.
      const rest = JSON.parse(JSON.stringify(events.filter((event) => event.type !== "cardPlayed" && event.type !== "hookResolving")).replaceAll(`"${galleryPlayId(1)}"`, `"${CLOAKED}"`)) as GameEvent[];
      const reveal: GameEvent[] = [
        { type: "activated", player: "p1", instanceId: CLOAKED, defId, ability: "reveal" },
        { type: "trapFired", instanceId: CLOAKED, defId, controller: "p1", row: "backrow", lane: 2 },
        { type: "hookResolving", sourceId: CLOAKED, defId, controller: "p1", hook: "cry" },
        ...rest,
      ];
      const planned = planOut(reveal, withEvents(board, reveal));
      expect(planned.filter((p) => ownCast(p.cues)).map((p) => p.entry.events[0]?.type), `${defId} reveal`).toEqual(["trapFired"]);
      // Its Cry's marker leaves the firing's `cast` open, so its text's beats play at the reveal too.
      expect(textBeat(all(planned)), `${defId} reveal text`).toBe(true);
    }
  });
});

/** Every player field of `event` turned to the other seat's, so the viewer's own play becomes the other player's. */
function byOther(event: GameEvent): GameEvent {
  const swap = (player: PlayerId): PlayerId => (player === "p1" ? "p2" : "p1");
  const out: Record<string, unknown> = { ...event };
  for (const field of ["player", "controller", "owner"] as const) {
    const value = out[field];
    if (value === "p1" || value === "p2") out[field] = swap(value);
  }
  if (typeof out.targetId === "string") out.targetId = out.targetId.replace(/^hero-(p[12])$/, (_, p: PlayerId) => `hero-${swap(p)}`);
  return out as GameEvent;
}

/** `event` as R97 sends it to p1 when p2 plays it: the cards p2 gets, the transforms and cost changes in p2's hand, hidden. */
function redacted(event: GameEvent): GameEvent {
  switch (event.type) {
    case "addedToHand":
    case "drawn":
      return event.player === "p2" ? { ...event, instanceId: HIDDEN, defId: HIDDEN } : event;
    case "transformed":
      return { ...event, instanceId: HIDDEN, fromDefId: HIDDEN, toDefId: HIDDEN, newInstanceId: HIDDEN, hiddenFrom: ["p1"] };
    case "costChanged":
      return { ...event, instanceId: HIDDEN, hiddenFrom: ["p1"] };
    case "fused":
      return { ...event, instanceIds: [HIDDEN, HIDDEN], resultInstanceId: HIDDEN, defId: HIDDEN };
    default:
      return event;
  }
}

describe("R3402001 R202 from the opponent's seat", () => {
  it("R3402001 from the opponent's seat no beat shows a hidden card", () => {
    // The other player plays each card with a Discover, a hand or a deck; p1 watches.
    const discover = new Set(["core-097", "core-098", "core-099"]);
    for (const defId of ["core-083", "core-087", "core-093", "core-095", "core-097", "core-098", "core-099"]) {
      const entry = FX_CARD_TABLE.get(defId);
      const play = entry === undefined ? null : galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      const view = fullBoardView();
      const theirs = galleryCardEvents(defId, play, view, 1).map(byOther).map(redacted);
      // Their Discover opens a prompt only they answer; their roll's draws and their grade's discounts land hidden.
      const cast = theirs.findIndex((event) => event.type === "cardPlayed");
      if (discover.has(defId)) theirs.splice(cast + 1, 0, { type: "promptOpened", player: "p2", kind: "discover" });
      if (defId === "core-095") theirs.splice(theirs.length - 1, 0, { type: "drawn", player: "p2", instanceId: HIDDEN, defId: HIDDEN }, { type: "drawn", player: "p2", instanceId: HIDDEN, defId: HIDDEN });
      if (defId === "core-093") theirs.splice(theirs.length - 1, 0, { type: "costChanged", instanceId: HIDDEN, cost: 0, hiddenFrom: ["p1"] });
      // A Field Spell of theirs lands face up in their first backrow zone.
      const board = CATALOG[defId]?.type === "Field Spell" ? { ...view, opponent: { ...view.opponent, backrow: [{ ...(view.you.backrow[0] ?? { faceDown: false }), defId, type: "Field Spell" }, ...view.opponent.backrow.slice(1)] } } : view;
      const cues = all(planOut(theirs, withEvents(board as PlayerView, theirs)));
      expect(cues.length, defId).toBeGreaterThan(0);
      const text = JSON.stringify(cues);
      expect(text, defId).not.toMatch(/core-\d{3}/);
      expect(text, defId).not.toMatch(/hand-card-/);
      expect(text, defId).not.toContain(`"${HIDDEN}"`);
    }
  });

  it("R3402001 a Trap of theirs set face down plans nothing, and once it fires names no card", () => {
    for (const defId of ["core-085", "core-096"]) {
      const view = fullBoardView();
      // R97: the set is a hidden card's, its id and its definition the sentinel's.
      const set: GameEvent[] = [
        { type: "cardPlayed", player: "p2", instanceId: HIDDEN, defId: HIDDEN, costPaid: 2 },
        { type: "cardResolved", player: "p2", instanceId: HIDDEN, defId: HIDDEN, permanent: true, costPaid: 2 },
      ];
      const planned = planOut(set, withEvents(view, set));
      // It plays what any hidden card's play does (a glint at their hand pile), and no signature beat of its own.
      const setCues = planned.flatMap((p) => p.cues);
      expect(setCues.every((cue) => cue.kind === "burst" && JSON.stringify(cue.at) === JSON.stringify(tid(animTestid.hand("opponent")))), defId).toBe(true);
      expect(planned.some((p) => p.entry.entrance !== undefined || p.entry.setPiece !== undefined), defId).toBe(false);
      // Its firing names it publicly, yet no cue carries its id.
      const entry = FX_CARD_TABLE.get(defId);
      const play = entry === undefined ? null : galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      const fired = galleryCardEvents(defId, play, view, 1)
        .filter((event) => event.type !== "cardPlayed" && event.type !== "cardResolved")
        .map(byOther);
      const cues = all(planOut(fired, withEvents(view, fired)));
      expect(cues.length, defId).toBeGreaterThan(0);
      expect(JSON.stringify(cues), defId).not.toMatch(/core-\d{3}/);
    }
  });
});
