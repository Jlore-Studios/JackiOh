// R3338000: the Terminus cards' set pieces as the effects layer plans them (fx/terminus/), the way FxLayer
// plans every entry (remember, then plan): each held inside its entry and gone within FX_MAX_TAIL_MS of
// its end (R200) at every hold Big moments and the speed give it, Dice of Doom's three outcomes drawn
// each its own way, no cue naming a hidden card from the other seat (R202), the follow-ups the turns
// they owe open, and the Terminus pool the catalog's own.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  SET_PIECE_HOLD_CAP_MS,
  animTestid,
  planEntries,
  setPieceEntries,
  type AnimationEntry,
  type SetPieceClaimer,
} from "../../game/animations.ts";
import { testid } from "../../game/contract.ts";
import { createPlayTracker } from "../../game/runs.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { frac } from "../build.ts";
import { FX_CARD_TABLE } from "../cards/tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_TERMINUS_COUNT_AT } from "../constants.ts";
import { planFx } from "../cues.ts";
import type { FxBigMoments } from "../settings.ts";
import { createFxMemory } from "../memory.ts";
import { doomFixture, galleryCardEvents, type DoomRoll } from "../motifs/gallery.ts";
import {
  DOOM_MISS_HOLD_MS,
  SET_PIECE_KEYS,
  SET_PIECES,
  hasSetPiece,
  planSetPiece,
  signatureClaim,
  signatureRunOf,
  wipeClaim,
  type SetPieceKey,
} from "../setPieces.ts";
import type { FxCue, FxMemory, FxPlanEnv, FxRollCue, FxSetPieceCue } from "../types.ts";
import { TERMINUS_TUNING } from "./build.ts";
import { BIG_CRUNCH, DICE_OF_DOOM, EVENT_HORIZON, LAST_WORD, OLIVIAN_EXPANSE, TERMINUS_IDS } from "./ids.ts";
import { TERMINUS_PLANNERS, planTerminus, planTerminusStill } from "./index.ts";

const envOf = (memory: FxMemory = createFxMemory()): FxPlanEnv => ({ intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory });
const claimOf: SetPieceClaimer = (entry) => wipeClaim(entry) ?? signatureClaim(entry);

/** Every Terminus set piece: the seven signatures and the two Terminus board clears. */
const TERMINUS_KEYS: readonly SetPieceKey[] = [...Object.keys(TERMINUS_PLANNERS), "voidFold", "farewell"] as SetPieceKey[];

/** The card whose table entry names `key`. */
function cardOf(key: SetPieceKey): string {
  const found = [...FX_CARD_TABLE].find(([, entry]) => entry.kind === "setPiece" && entry.key === key);
  if (found === undefined) throw new Error(`no card names ${key}`);
  return found[0];
}

type Plan = { opts?: { bigMoments?: FxBigMoments; speed?: number; reduced?: boolean } };

/** `events` planned as the queue plans one action on `view` (with a set piece's claim), each entry's cues as FxLayer plans them. */
function play(events: readonly GameEvent[], view: PlayerView, { opts = {} }: Plan = {}, memory: FxMemory = createFxMemory()): { entry: AnimationEntry; cues: FxCue[] }[] {
  const reduced = opts.reduced ?? false;
  const planned = planEntries(events, withEvents(view, [...events]), reduced, createPlayTracker(), hasSetPiece, signatureRunOf);
  const scaled = planned.map((entry) => ({ ...entry, durationMs: Math.round(entry.durationMs / (opts.speed ?? 1)) }));
  const entries = setPieceEntries(scaled, claimOf, { reduced, bigMoments: opts.bigMoments ?? "full", speed: opts.speed ?? 1 });
  const env = envOf(memory);
  return entries.map((entry) => {
    memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

/** The gallery's play of `key`'s card, as the viewer's own. */
function galleryPlay(key: SetPieceKey, view: PlayerView): GameEvent[] {
  return galleryCardEvents(cardOf(key), { kind: "setPiece", key }, view, 1);
}

/** The entry `key`'s set piece is: its signature's or its wipe's. */
function pieceEntry(results: ReturnType<typeof play>): { entry: AnimationEntry; cues: FxCue[] } {
  const found = results.find(({ entry }) => entry.signature !== undefined || entry.wipe !== undefined);
  if (found === undefined) throw new Error("no set piece entry");
  return found;
}

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const pieceOf = (cues: readonly FxCue[]): FxSetPieceCue | undefined => cues.find((c): c is FxSetPieceCue => c.kind === "setPiece");

/** A seat's view of `events` from the other side: every card added to the caster's hand hidden (R97). */
function fromTheOtherSeat(events: readonly GameEvent[]): GameEvent[] {
  return events.map((event) => (event.type === "addedToHand" ? { ...event, instanceId: "hidden", defId: "hidden" } : event));
}

/** `card` played by `player` with `under` resolving beneath it. */
function played(player: PlayerId, instanceId: string, defId: string, under: readonly GameEvent[], cost = 4): GameEvent[] {
  return [
    { type: "cardPlayed", player, instanceId, defId, costPaid: cost },
    ...under,
    { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: cost },
  ];
}

describe("R3338000 the Terminus set pieces", () => {
  it("R3338000 every Terminus card's set piece is its own, and only the board clears play as wipes", () => {
    for (const key of TERMINUS_KEYS) {
      const defId = cardOf(key);
      expect(hasSetPiece(defId), key).toBe(key === "voidFold" || key === "farewell");
      expect(signatureRunOf(defId) === null, key).toBe(key === "voidFold" || key === "farewell");
    }
    expect(new Set(TERMINUS_KEYS.map(cardOf))).toEqual(new Set([...TERMINUS_IDS, OLIVIAN_EXPANSE, DICE_OF_DOOM]));
  });

  it("R3338000 R200 every cue ends within D + FX_MAX_TAIL_MS, held full, short, off, at every speed", () => {
    const view = fullBoardView();
    for (const key of TERMINUS_KEYS) {
      for (const bigMoments of ["full", "short", "off"] as const) {
        for (const speed of [0.5, 1, 2]) {
          const results = play(galleryPlay(key, view), view, { opts: { bigMoments, speed } });
          const { entry, cues } = pieceEntry(results);
          const where = `${key} ${bigMoments} ×${String(speed)}`;
          expect(entry.setPiece?.holdMs ?? 0, where).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
          if (bigMoments === "off") expect(entry.setPiece, where).toBeUndefined();
          else expect(entry.setPiece?.key, where).toBe(key);
          expect(pieceOf(cues)?.shape, where).toBe(SET_PIECES[key].shape);
          for (const result of results) {
            const D = result.entry.durationMs;
            for (const cue of result.cues) {
              const label = `${where}: ${cue.kind}`;
              expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
              expect(cue.delayMs, label).toBeLessThanOrEqual(D);
              if (cue.kind === "pull" || cue.kind === "conceal") continue;
              if (cue.kind === "shot" || cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
              expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
            }
          }
        }
      }
    }
  });

  it("R3338000 Reduce motion holds nothing and shows one still frame of each", () => {
    const view = fullBoardView();
    for (const key of TERMINUS_KEYS) {
      const results = play(galleryPlay(key, view), view, { opts: { reduced: true } });
      const { entry } = pieceEntry(results);
      expect(entry.durationMs, key).toBe(0);
      expect(entry.setPiece, key).toBeUndefined();
      // An entry of no duration plays no set piece (FxLayer sends it to `still`, never to planFx).
      expect(planTerminus(entry, view, envOf()), key).toBeNull();
      expect(planSetPiece(entry, view, envOf()), key).toBeNull();
      const still = planTerminusStill(entry, view, envOf());
      expect(still, key).toHaveLength(1);
      expect(still[0], key).toMatchObject({ kind: "setPiece", shape: SET_PIECES[key].shape, still: true, holdMs: 0, delayMs: 0 });
      expect(planTerminusStill(entry, view, { intensity: 0 }), key).toEqual([]);
    }
    // The other board clears have no still frame of their own.
    const nether = play(galleryCardEvents(cardOf("nether"), { kind: "setPiece", key: "nether" }, view, 1), view, { opts: { reduced: true } });
    expect(planTerminusStill(pieceEntry(nether).entry, view, envOf())).toEqual([]);
  });

  it.each(["jackpot", "miss", "embiggened"] as const)("R3338000 the Dice jackpot, miss and Embiggened: %s", (roll: DoomRoll) => {
    const view = fullBoardView();
    const events = played("p1", "dd", DICE_OF_DOOM, doomFixture(view, "dd", "p1", roll), 0);
    const { entry, cues } = pieceEntry(play(events, view));
    const H = roll === "jackpot" ? SET_PIECES.diceOfDoom.holdMs : DOOM_MISS_HOLD_MS;
    expect(entry.setPiece).toEqual({ key: "diceOfDoom", holdMs: H });
    // Every event of the run plays inside the one entry, its own rows drawn by the set piece.
    expect(entry.events.map((event) => event.type)).toEqual(doomFixture(view, "dd", "p1", roll).slice(1).map((event) => event.type));
    const panel = cues.find((cue): cue is FxRollCue => cue.kind === "roll");
    expect(panel?.look).toBe("doom");
    expect(cues.filter((cue) => cue.kind === "roll")).toHaveLength(1);
    for (const item of panel?.items ?? []) expect(item.landMs).toBe(frac(TERMINUS_TUNING.doomLandAt, H));
    const ghosts = cues.filter((cue) => cue.kind === "ghost");
    const crack = cues.find((cue) => cue.kind === "crack");
    const glint = cues.find((cue) => cue.kind === "ring" && cue.preset === "gold");
    if (roll === "jackpot") {
      expect(pieceOf(cues)?.beats).toHaveLength(TERMINUS_IDS.length);
      expect(ghosts).toHaveLength(TERMINUS_IDS.length);
      for (const ghost of ghosts) expect(ghost).toMatchObject({ to: { kind: "testid", testid: animTestid.hand("you") } });
      for (const ghost of ghosts) expect(ghost.delayMs).toBeGreaterThanOrEqual(H);
      expect(crack).toBeUndefined();
    } else {
      expect(pieceOf(cues)?.beats).toBeUndefined();
      expect(ghosts).toEqual([]);
      expect(crack?.delayMs).toBe(frac(TERMINUS_TUNING.crumbleAt, H));
    }
    expect(glint === undefined, roll).toBe(roll !== "embiggened");
  });

  it("R3338000 R202 from the opponent's seat: Olivian's Discover and Dice's 67 name no card, and land in the opponent's hand", () => {
    const view = fullBoardView();
    const names = TERMINUS_IDS.map((id) => CATALOG[id]?.name ?? id);
    const discover: GameEvent[] = played("p2", "olv", OLIVIAN_EXPANSE, [
      { type: "hookResolving", sourceId: "olv", defId: OLIVIAN_EXPANSE, controller: "p2", hook: "cry" },
      { type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" },
    ]);
    const doom = played("p2", "dd", DICE_OF_DOOM, fromTheOtherSeat(doomFixture(view, "dd", "p2", "jackpot")), 0);
    for (const [label, events, count] of [
      ["Olivian Expanse", discover, 1],
      ["Dice of Doom", doom, TERMINUS_IDS.length],
    ] as const) {
      const results = play(events, view);
      const { entry, cues } = pieceEntry(results);
      expect(entry.setPiece?.key, label).toBe(label === "Olivian Expanse" ? "terminusRift" : "diceOfDoom");
      const text = JSON.stringify(results.map((result) => result.cues));
      for (const id of TERMINUS_IDS) expect(text, `${label} ${id}`).not.toContain(id);
      for (const name of names) expect(text, `${label} ${name}`).not.toContain(name);
      const ghosts = cues.filter((cue) => cue.kind === "ghost");
      expect(ghosts, label).toHaveLength(count);
      for (const ghost of ghosts) expect(ghost, label).toMatchObject({ to: { kind: "testid", testid: animTestid.hand("opponent") } });
    }
    // The rift's stars are the pool, every Terminus card, whichever one the pick was.
    const rift = pieceOf(pieceEntry(play(discover, view)).cues);
    expect(rift?.beats).toHaveLength(TERMINUS_IDS.length);
    // A hidden caster keys none: the seat cannot read whose Discover it was.
    const veiled = discover.map((event) => ("defId" in event && event.defId === OLIVIAN_EXPANSE ? { ...event, defId: "hidden" } : event)) as GameEvent[];
    expect(play(veiled, view).some(({ entry }) => entry.signature !== undefined)).toBe(false);
  });

  it("R3338000 the follow-ups: Event Horizon's turn snaps back, Last Word's lights Charge and counts down, The Big Crunch's point holds", () => {
    const view = fullBoardView();
    const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => cue.kind);
    // Event Horizon owes its caster the next turn: that turn's opening snaps the board back.
    const memory = createFxMemory();
    const horizon = play(
      [
        ...played("p1", "eh", EVENT_HORIZON, [
          { type: "hookResolving", sourceId: "eh", defId: EVENT_HORIZON, controller: "p1", hook: "cry" },
          { type: "modifierChanged", player: "p1", modifierId: "extraTurns", added: true },
        ]),
        { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 },
        { type: "turnStarted", player: "p1", turn: 4, extra: true },
      ] as GameEvent[],
      view,
      {},
      memory,
    );
    const opened = horizon.find(({ entry }) => entry.type === "turnStarted");
    expect(kinds(opened?.cues ?? [])).toEqual(expect.arrayContaining(["ring", "burst", "shake", "banner"]));
    // The turn after it owes nothing.
    const after = play([{ type: "turnStarted", player: "p2", turn: 5 }] as GameEvent[], view, {}, memory);
    expect(kinds(after[0]?.cues ?? [])).not.toContain("shake");

    // Last Word fires on the opponent's turn, ending it (its TurnEnds modifier is theirs) and owing its controller Charge.
    const word = play(
      [
        { type: "trapFired", instanceId: "lw", defId: LAST_WORD, controller: "p1", row: "backrow", lane: 1 },
        { type: "enteredGraveyard", instanceId: "lw", defId: LAST_WORD, owner: "p1" },
        { type: "modifierChanged", player: "p2", modifierId: "m26", added: true },
        { type: "modifierChanged", player: "p1", modifierId: "m27", added: true },
        { type: "turnCutShort", player: "p2", byInstanceId: "lw" },
        { type: "turnEnded", player: "p2", turn: 2, unspentMana: 1 },
        { type: "turnStarted", player: "p1", turn: 3 },
      ] as GameEvent[],
      view,
    );
    const signed = pieceEntry(word);
    expect(signed.entry.events.map((event) => event.type)).toEqual(["trapFired", "enteredGraveyard", "modifierChanged", "modifierChanged"]);
    const turn = word.find(({ entry }) => entry.type === "turnStarted");
    const units = view.you.units.flatMap((unit) => (unit === null ? [] : [unit]));
    const callouts = (turn?.cues ?? []).filter((cue) => cue.kind === "callout");
    expect(callouts).toHaveLength(units.length);
    for (const callout of callouts) expect(callout).toMatchObject({ keyword: "Charge" });
    const counts = (turn?.cues ?? []).filter((cue) => cue.kind === "zone");
    expect(counts.map((cue) => (cue.kind === "zone" ? cue.text : ""))).toEqual(FX_TERMINUS_COUNT_AT.map((_, k) => String(FX_TERMINUS_COUNT_AT.length - k)));
    for (const count of counts) expect(count).toMatchObject({ at: { kind: "testid", testid: testid.hero("you") } });

    // The Big Crunch: its claimed first cast collapses the board; each later entry of its play keeps the point.
    const crunch = play(
      played("p1", "bc", BIG_CRUNCH, [
        { type: "hookResolving", sourceId: "bc", defId: BIG_CRUNCH, controller: "p1", hook: "cry" },
        ...played("p1", "c1", "core-095", [], 0),
        ...played("p1", "c2", "classicplus-073", [], 0),
      ]),
      view,
    );
    // Its own play's entry, before the collapse, draws no point.
    const own = crunch.find(({ entry }) => entry.events.some((event) => event.type === "cardPlayed" && event.instanceId === "bc"));
    expect(pieceOf(own?.cues ?? [])).toBeUndefined();
    const casts = crunch.filter(({ entry }) => entry.cast?.by === BIG_CRUNCH);
    expect(casts).toHaveLength(2);
    const [first, second] = casts;
    expect(first?.entry.setPiece?.key).toBe("bigCrunch");
    expect(first?.entry.signature).toMatchObject({ by: BIG_CRUNCH, sourceId: "bc", player: "p1" });
    expect(second?.entry.signature).toBeUndefined();
    expect(pieceOf(first?.cues ?? [])).toMatchObject({ shape: "crunch", holdMs: first?.entry.setPiece?.holdMs });
    expect(pieceOf(second?.cues ?? [])).toMatchObject({ shape: "crunch", holdMs: 0 });
    for (const cast of casts) expect(kinds(cast.cues)).toContain("ghost");
    // Once its play resolves, the point is gone.
    const last = crunch.at(-1);
    expect(last?.entry.events.some((event) => event.type === "cardResolved" && event.instanceId === "bc")).toBe(true);
    expect(pieceOf(last?.cues ?? [])).toBeUndefined();
  });

  it("R3338000 TERMINUS_IDS are the catalog's Terminus cards, in its order", () => {
    const tagged = Object.keys(CATALOG).filter((id) => CATALOG[id]?.tags.includes("Terminus") === true);
    expect(tagged).toEqual([...TERMINUS_IDS]);
    expect(SET_PIECE_KEYS).toEqual(expect.arrayContaining([...TERMINUS_KEYS]));
  });
});
