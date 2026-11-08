// R3389001: Classic's eleven signatures (#3389), card by card: each entry is its signature; over its
// gallery play each draws its entrance or its own cast, its text's beat and its aura; the beats a real
// game plays outside the card's own play (R3389000's claims: a cast its play makes, an event naming it)
// draw too; from the other seat no beat shows a hidden card (R202); the effects off and a zero-length
// entry plan nothing, and every delay scales with the entry (R200, R201). Planned as the board's
// effects layer plans them: remember, then plan.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { entranceEntries, planEntries, setPieceEntries, takeoverClaim, type AnimationEntry, type EntranceResolver } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { faceDownBackrow, fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { planAuras } from "../../aura/plan.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { ENTRANCES } from "../../entrances.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import { entranceCues } from "../../takeover.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { entryAura, FX_CARD_TABLE, type FxCardEntry } from "../tables.ts";
import { shrinePoint } from "./inTooDeep.ts";

/** Classic's Legendaries and Mythics, in catalog order. */
const CLASSIC_SIGNATURES = ["classic-004", "classic-007", "classic-009", "classic-028", "classic-033", "classic-044", "classic-045", "classic-056", "classic-080", "classic-085", "classic-090"] as const;

/** No cue names a card or the sentinel (R202). */
const NAMES_A_CARD = /classic-\d{3}|hidden/;

const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

function bespokeOf(defId: string): Extract<FxCardEntry, { kind: "bespoke" }> {
  const entry = FX_CARD_TABLE.get(defId);
  if (entry?.kind !== "bespoke") throw new Error(`${defId} is no bespoke entry`);
  return entry;
}

/** A cue's end, as tiers.test.ts counts it. */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shot":
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

type Planned = { entry: AnimationEntry; cues: FxCue[]; claims: { event: GameEvent; cues: FxCue[]; row: "keep" | "replace" }[] };

/** Every entry of `events`, entered and held as the queue holds them, planned in order: remember, then plan. */
function planStream(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): Planned[] {
  const entered = entranceEntries(planEntries(events, withEvents(view, [...events]), false, undefined, hasSetPiece), resolve, { seen: new Set() });
  const held = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
  return held.map((entry) => {
    env.memory.remember(entry.events);
    const claims = entry.events.flatMap((event) => {
      const claim = planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs });
      return claim === null ? [] : [{ event, ...claim }];
    });
    return { entry, cues: [...entranceCues(entry, env, entry.durationMs), ...planFx(entry, entry.view, env)], claims };
  });
}

/** `defId`'s gallery play, over its own board, as the gallery plays it. */
function galleryStream(defId: string): { events: GameEvent[]; view: PlayerView; board: PlayerView } {
  const entry = bespokeOf(defId);
  const play = galleryPlayOf(defId, entry);
  if (play === null) throw new Error(`${defId} has no gallery play`);
  const board = galleryBoardOf(defId, fullBoardView());
  return { events: galleryCardEvents(defId, play, board, 1), view: galleryFieldView(defId, board), board };
}

const played = (player: PlayerId, instanceId: string, defId: string, costPaid = 2): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid });
const resolved = (player: PlayerId, instanceId: string, defId: string, costPaid = 2): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid });
const landed = (player: PlayerId, instanceId: string, defId: string, row: "units" | "backrow", lane: number): GameEvent => ({ type: "summoned", player, instanceId, defId, row, lane });

/** A board with every zone open but a Field Spell of `player`'s face up in its first backrow zone. */
function fieldBoard(player: PlayerId, defId: string, instanceId: string): PlayerView {
  const view = fullBoardView();
  const side = player === view.viewer ? "you" : "opponent";
  const seat = view[side];
  return { ...view, [side]: { ...seat, backrow: [{ ...(seat.backrow[2] ?? seat.backrow[0]), instanceId, defId, type: "Field Spell", faceDown: false, owner: player, controller: player }, ...seat.backrow.slice(1)] } };
}

/** Every recipe claim of `planned`'s entries, flattened. */
const claimsOf = (planned: readonly Planned[]) => planned.flatMap((p) => p.claims);

describe("R3389001 Classic's signatures", () => {
  it("R3389001 each card's entry is its signature", () => {
    const entries = Object.fromEntries(CLASSIC_SIGNATURES.map((defId) => [defId, FX_CARD_TABLE.get(defId)]));
    expect(entries).toEqual({
      "classic-004": { kind: "bespoke", key: "palantirGaze", entrance: "seeingOrb", aura: { look: "seeingIris", palette: "Tech/hologram", scope: "zone" }, signature: true },
      "classic-007": { kind: "bespoke", key: "sceptreLocket", entrance: "sceptrePlant", aura: { look: "sceptreLoop", scope: "zone" }, signature: true },
      "classic-009": { kind: "bespoke", key: "taxSeizure", signature: true },
      "classic-028": { kind: "bespoke", key: "secondGust", entrance: "galeBurst", aura: { look: "graveBreeze", palette: "Physical/wind", scope: "zone" }, signature: true },
      "classic-033": { kind: "bespoke", key: "webRedirect", entrance: "silkDrop", aura: { look: "silkWeb", scope: "tile" }, signature: true },
      "classic-044": { kind: "bespoke", key: "graveRise", signature: true },
      "classic-045": { kind: "bespoke", key: "titanLeaf", entrance: "titanBloom", aura: { look: "mossCreep", scope: "tile" }, signature: true },
      "classic-056": { kind: "bespoke", key: "tyrantCommand", entrance: "tyrantSigil", aura: { look: "crownSigil", scope: "tile" }, signature: true },
      "classic-080": { kind: "bespoke", key: "craterLock", entrance: "bigBoom", aura: { look: "craterSmoke", palette: "Fire", scope: "tile" }, signature: true },
      "classic-085": { kind: "bespoke", key: "crownSwap", entrance: "royalCloak", aura: { look: "cloakShimmer", scope: "tile" }, signature: true },
      "classic-090": { kind: "bespoke", key: "shrineDescent", entrance: "abyssSurface", aura: { look: "abyssWhirl", palette: "Dark/shadow", scope: "zone" }, takeover: "abyss", signature: true },
    });
    // The Legendaries and Mythics of Classic are exactly these.
    const classic = Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === "Classic" && ["Legendary", "Mythic"].includes(CATALOG[id]?.printedRarity ?? CATALOG[id]?.rarity ?? ""));
    expect(classic).toEqual([...CLASSIC_SIGNATURES]);
  });

  it("R3389001 each plays its entrance, text beat and aura over its gallery play", () => {
    for (const defId of CLASSIC_SIGNATURES) {
      const entry = bespokeOf(defId);
      const { events, view } = galleryStream(defId);
      const planned = planStream(events, view);
      const claims = claimsOf(planned);
      // Its first beat: its entrance as it lands, in place of the summon's row, or its own cast.
      if (entry.entrance !== undefined) {
        const landing = claims.find((claim) => claim.event.type === "summoned" && claim.event.defId === defId);
        expect(landing?.row, `${defId} lands`).toBe("replace");
        expect(landing?.cues.length ?? 0, `${defId} lands`).toBeGreaterThan(2);
      }
      // Its text's beat: its recipe claims at least one event of its fixture.
      const beat = claims.filter((claim) => !(claim.event.type === "summoned" && claim.event.defId === defId));
      expect(beat.length, `${defId} beat`).toBeGreaterThan(0);
      expect(beat.some((claim) => claim.cues.length > 0), `${defId} beat`).toBe(true);
      for (const fixture of RECIPE_FIXTURES[entry.key](view, "fx-gallery-card-1", view.viewer)) expect(events, defId).toContainEqual(fixture);
      // Its aura stands on its own zone, a Unit's on the tile it lands in (R3400000); a Spell and a Trap wear none.
      const aura = entryAura(entry);
      const auras = planAuras(view, new Map()).filter((spec) => spec.defId === defId);
      if (aura === undefined) expect(auras, defId).toEqual([]);
      else expect(auras.map((spec) => [spec.look, spec.scope]), defId).toEqual([[aura.look, CATALOG[defId]?.type === "Unit" ? "tile" : "zone"]]);
      // R202: no cue names the card.
      expect(JSON.stringify(planned.map((p) => p.cues)), defId).not.toMatch(NAMES_A_CARD);
    }
    // A Spell's and a Trap's own cast stands in for the shared Legendary rays.
    for (const defId of ["classic-009", "classic-044"]) {
      const { events, view } = galleryStream(defId);
      expect(planStream(events, view).flatMap((p) => p.cues).some((cue) => cue.kind === "rays"), defId).toBe(false);
    }
  });

  it("R3389001 the beats outside the play", () => {
    // Palantir's steal in a real game: the opponent's (2) Cost Spell is countered into Palantir's owner's hand, no play of Palantir's open.
    const palantir = fieldBoard("p1", "classic-004", "pal");
    const steal = claimsOf(planStream([played("p2", "spell", "core-016"), { type: "countered", player: "p2", instanceId: "spell", defId: "core-016", byInstanceId: "pal", to: "hand" }], palantir));
    expect(steal.map((claim) => [claim.event.type, claim.row])).toEqual([["countered", "replace"]]);
    expect(steal[0]?.cues.some((cue) => cue.kind === "ghost" && cue.from.kind === "testid" && cue.from.testid === "hand-opponent" && cue.to.kind === "testid" && cue.to.testid === "hand-you")).toBe(true);
    // A counter by another card claims nothing of Palantir's.
    expect(claimsOf(planStream([played("p2", "spell", "core-016"), { type: "countered", player: "p2", instanceId: "spell", defId: "core-016", byInstanceId: "other", to: "graveyard" }], palantir))).toEqual([]);

    // InfiniScepter's Activate on a later turn: its copy flies out to be cast.
    const scepter = fieldBoard("p1", "classic-007", "rod");
    const activate = claimsOf(planStream([{ type: "turnStarted", player: "p1", turn: 5 }, { type: "activated", player: "p1", instanceId: "rod", defId: "classic-007", ability: "cast-copy" }], scepter));
    expect(activate.map((claim) => [claim.event.type, claim.row])).toEqual([["activated", "replace"]]);
    expect(activate[0]?.cues.some((cue) => cue.kind === "ghost" && cue.to.kind === "viewport")).toBe(true);

    // Joro, from its owner's hand, takes the opponent's Spell's target: it lands, then the web snaps to it.
    const open = fullBoardView();
    open.you.units[2] = null;
    const aim = open.you.units[0]?.instanceId ?? "";
    const joro = planStream(
      [
        played("p2", "hit", "core-016"),
        landed("p1", "joro", "classic-033", "units", 3),
        { type: "redirected", what: "target", fromId: aim, toId: "joro", byInstanceId: "joro" },
        resolved("p2", "hit", "core-016"),
      ],
      open,
    );
    const web = claimsOf(joro).find((claim) => claim.event.type === "redirected");
    expect(web?.cues.some((cue) => cue.kind === "shot" && cue.to.kind === "testid" && cue.to.testid === testid.zone("you", "units", 3))).toBe(true);
    expect(claimsOf(joro).find((claim) => claim.event.type === "summoned")?.row).toBe("replace");

    // In Too Deep's quests on a later turn: a shrine lit lower down its spiral for each step, its shrine bursting as one completes.
    const deep = fieldBoard("p1", "classic-090", "deep");
    const quests = claimsOf(
      planStream(
        [
          { type: "turnStarted", player: "p1", turn: 6 },
          { type: "questProgressed", player: "p1", instanceId: "deep", quest: "q1", progress: 2, goal: 3 },
          { type: "questProgressed", player: "p1", instanceId: "deep", quest: "q1", progress: 0, goal: 3 },
          { type: "questCompleted", player: "p1", instanceId: "deep", quest: "q1" },
        ],
        deep,
      ),
    );
    expect(quests.map((claim) => claim.event.type)).toEqual(["questProgressed", "questCompleted"]);
    expect(quests[0]?.cues.some((cue) => cue.kind === "burst" && cue.at.kind === "testid" && cue.at.at?.y === shrinePoint(2, 3).y)).toBe(true);
    expect(shrinePoint(2, 3).y).toBeGreaterThan(shrinePoint(1, 3).y);

    // Spell Tyrant's Cry casts two Spells from its graveyard: each rises to its crown, then burns into exile in place of that row.
    const throne = fullBoardView();
    throne.you.units[2] = null;
    const tyrant = claimsOf(
      planStream(
        [
          played("p1", "tyr", "classic-056", 4),
          landed("p1", "tyr", "classic-056", "units", 3),
          { type: "promptOpened", player: "p1", kind: "pick" },
          ...["core-016", "core-021"].flatMap((defId, k): GameEvent[] => [
            played("p1", `cast-${String(k)}`, defId, 0),
            { type: "exiled", instanceId: `cast-${String(k)}`, defId, owner: "p1" },
            resolved("p1", `cast-${String(k)}`, defId, 0),
          ]),
          resolved("p1", "tyr", "classic-056", 4),
        ],
        throne,
      ),
    );
    const rises = tyrant.filter((claim) => claim.event.type === "cardPlayed");
    const ashes = tyrant.filter((claim) => claim.event.type === "exiled");
    expect(rises).toHaveLength(2);
    expect(ashes.map((claim) => claim.row)).toEqual(["replace", "replace"]);
    for (const rise of rises) expect(rise.cues.some((cue) => cue.kind === "ghost" && cue.from.kind === "testid" && cue.from.testid === "graveyard-you")).toBe(true);
    for (const ash of ashes) expect(ash.cues.some((cue) => cue.kind === "ghost" && cue.to.kind === "testid" && cue.to.testid === "exile-you")).toBe(true);
    // A cast another card makes is none of the Tyrant's.
    expect(claimsOf(planStream([played("p1", "c9", "core-016", 0), { type: "exiled", instanceId: "c9", defId: "core-016", owner: "p1" }], throne))).toEqual([]);

    // Second Wind's Cry over a whole deck and hand: its backs fly beside each pile's one wave.
    const wind = fieldBoard("p1", "classic-028", "wind");
    const deck = { ...wind, you: { ...wind.you, libraryCount: 3, hand: wind.you.hand } };
    const blown = planStream(
      [
        played("p1", "wind", "classic-028", 0),
        ...[0, 1, 2].map((k): GameEvent => ({ type: "exiled", instanceId: `deck-${String(k)}`, defId: "core-003", owner: "p1" })),
        resolved("p1", "wind", "classic-028", 0),
      ],
      withEvents(deck, []),
    );
    const zone = blown.find((p) => p.entry.zone !== undefined);
    if (zone !== undefined) expect(zone.cues.some((cue) => cue.kind === "ghost" && cue.to.kind === "testid" && cue.to.testid === "exile-you")).toBe(true);
    else expect(claimsOf(blown).some((claim) => claim.event.type === "exiled")).toBe(true);
  });

  it("R3389001 R202 from the opponent's seat no beat shows a hidden card", () => {
    // The viewer is p1; p2 plays each card, its hand, its deck and its face-down backrow hidden from p1.
    const open = (): PlayerView => {
      const view = fullBoardView();
      view.opponent.units[2] = null;
      return view;
    };
    const streams: Record<(typeof CLASSIC_SIGNATURES)[number], { view: PlayerView; events: GameEvent[] }> = {
      "classic-004": {
        view: fieldBoard("p2", "classic-004", "pal"),
        events: [
          played("p1", "mine", "core-016"),
          { type: "hookResolving", sourceId: "pal", defId: "classic-004", controller: "p2", hook: "whenever" },
          { type: "destroyed", instanceId: "pal", defId: "classic-004", owner: "p2", controller: "p2", attack: 0, maxHealth: 0, killerId: null },
          { type: "countered", player: "p1", instanceId: "mine", defId: "core-016", byInstanceId: "pal", to: "hand" },
          { type: "cardPlayed", player: "p2", instanceId: "mine", defId: "core-016", costPaid: 0 },
        ],
      },
      "classic-007": {
        view: open(),
        events: [
          played("p2", "rod", "classic-007", 1),
          landed("p2", "rod", "classic-007", "backrow", 3),
          // Its hand pick is p2's alone: p1's prompt event names no kind, and the card it picks moves face down until it lands in exile.
          { type: "promptOpened", player: "p2" },
          { type: "exiled", instanceId: "picked", defId: "core-016", owner: "p2" },
          resolved("p2", "rod", "classic-007", 1),
          { type: "activated", player: "p2", instanceId: "rod", defId: "classic-007", ability: "cast-copy" },
        ],
      },
      "classic-009": {
        view: { ...open(), opponent: { ...open().opponent, backrow: [faceDownBackrow, faceDownBackrow, faceDownBackrow, faceDownBackrow, faceDownBackrow] } },
        events: [
          // Set face down: p1 reads nothing of it.
          played("p2", "hidden", "hidden", 3),
          resolved("p2", "hidden", "hidden", 3),
          { type: "drawn", player: "p1", instanceId: "d1", defId: "core-055" },
          { type: "trapFired", instanceId: "tax", defId: "classic-009", controller: "p2", row: "backrow", lane: 2 },
          { type: "promptOpened", player: "p1", kind: "pick" },
          { type: "stolen", instanceId: "c1", defId: "core-002", from: "p1", to: "p2", zone: "hand" },
          { type: "stolen", instanceId: "c2", defId: "core-019", from: "p1", to: "p2", zone: "hand" },
        ],
      },
      "classic-028": {
        view: open(),
        events: [
          played("p2", "wind", "classic-028", 0),
          landed("p2", "wind", "classic-028", "backrow", 3),
          { type: "exiled", instanceId: "deck-1", defId: "core-003", owner: "p2" },
          { type: "discarded", instanceId: "hand-1", defId: "core-055", owner: "p2" },
          resolved("p2", "wind", "classic-028", 0),
        ],
      },
      "classic-033": {
        view: open(),
        events: [
          // Waiting in p2's hand it shows nothing; it acts as p1's Spell targets p2's unit.
          played("p1", "hit", "core-016"),
          landed("p2", "joro", "classic-033", "units", 3),
          { type: "redirected", what: "target", fromId: open().opponent.units[0]?.instanceId ?? "", toId: "joro", byInstanceId: "joro" },
          resolved("p1", "hit", "core-016"),
        ],
      },
      "classic-044": {
        view: open(),
        events: [played("p2", "gy", "classic-044", 4), landed("p2", "risen", "core-003", "units", 3), { type: "exiled", instanceId: "gy", defId: "classic-044", owner: "p2" }, resolved("p2", "gy", "classic-044", 4)],
      },
      "classic-045": {
        view: open(),
        events: [
          played("p2", "titan", "classic-045"),
          landed("p2", "titan", "classic-045", "units", 3),
          // Its draw is p2's: p1 reads the sentinel.
          { type: "drawn", player: "p2", instanceId: "hidden", defId: "hidden" },
          { type: "healed", targetId: "hero-p2", amount: 3 },
          resolved("p2", "titan", "classic-045"),
        ],
      },
      "classic-056": {
        view: open(),
        events: [
          played("p2", "tyr", "classic-056", 4),
          landed("p2", "tyr", "classic-056", "units", 3),
          { type: "promptOpened", player: "p2" },
          played("p2", "cast", "core-016", 0),
          { type: "exiled", instanceId: "cast", defId: "core-016", owner: "p2" },
          resolved("p2", "cast", "core-016", 0),
          resolved("p2", "tyr", "classic-056", 4),
        ],
      },
      "classic-080": {
        view: open(),
        events: [played("p2", "max", "classic-080", 4), landed("p2", "max", "classic-080", "units", 3), { type: "locked", player: "p2", row: "units", lane: 3 }, resolved("p2", "max", "classic-080", 4)],
      },
      "classic-085": {
        view: open(),
        events: [
          played("p2", "king", "classic-085", 4),
          landed("p2", "king", "classic-085", "units", 3),
          { type: "swapped", what: "library" },
          landed("p2", "recruit", "core-003", "units", 2),
          resolved("p2", "king", "classic-085", 4),
        ],
      },
      "classic-090": {
        view: open(),
        events: [
          played("p2", "deep", "classic-090", 1),
          landed("p2", "deep", "classic-090", "backrow", 3),
          resolved("p2", "deep", "classic-090", 1),
          { type: "questProgressed", player: "p2", instanceId: "deep", quest: "q1", progress: 1, goal: 3 },
          { type: "questCompleted", player: "p2", instanceId: "deep", quest: "q1" },
          // Its rewards are p2's prompt: p1's names no kind, and no beat draws them.
          { type: "promptOpened", player: "p2" },
        ],
      },
    };
    for (const defId of CLASSIC_SIGNATURES) {
      const { view, events } = streams[defId];
      for (const event of events) if (event.type === "promptOpened" && event.player === "p2") expect(event.kind, defId).toBeUndefined();
      const planned = planStream(events, view);
      expect(claimsOf(planned).length, `${defId} draws`).toBeGreaterThan(0);
      expect(JSON.stringify(planned.map((p) => p.cues)), defId).not.toMatch(NAMES_A_CARD);
      expect(JSON.stringify(claimsOf(planned).map((claim) => claim.cues)), defId).not.toMatch(NAMES_A_CARD);
      // Nothing it moves shows a face: a card it moves is a card back.
      expect(JSON.stringify(planned.map((p) => p.cues)), defId).not.toMatch(/core-0\d\d/);
    }
    // Joro and Income Tax stay hidden until they act: a face-down card and a hidden play key nothing, and wear no aura.
    const hiddenTrap = streams["classic-009"];
    const set = planStream(hiddenTrap.events.slice(0, 3), hiddenTrap.view);
    expect(claimsOf(set)).toEqual([]);
    expect(planAuras(hiddenTrap.view, new Map())).toEqual(planAuras(open(), new Map()).filter((spec) => spec.side === "you"));
    // A hidden play or firing keys nothing of any signature's recipe.
    for (const defId of CLASSIC_SIGNATURES) {
      const fixture = RECIPE_FIXTURES[bespokeOf(defId).key](open(), "hidden", "p2").filter((event) => !(event.type === "trapFired" || event.type === "hookResolving"));
      const hidden = planStream([played("p2", "hidden", "hidden"), { type: "trapFired", instanceId: "hidden", defId: "hidden", controller: "p2", row: "backrow", lane: 2 }, ...fixture], open());
      expect(claimsOf(hidden), defId).toEqual([]);
    }
  });

  it("R3389001 intensity 0 and a zero-length entry plan nothing; delays scale with D", () => {
    for (const defId of CLASSIC_SIGNATURES) {
      const entry = bespokeOf(defId);
      const { events, view } = galleryStream(defId);
      // The effects off: the layer plans nothing, and no recipe draws.
      expect(planStream(events, view, envOf({ intensity: 0 })).flatMap((p) => p.cues), defId).toEqual([]);
      expect(claimsOf(planStream(events, view, envOf({ intensity: 0 }))).filter((claim) => claim.cues.length > 0 && !(claim.event.type === "summoned" && claim.event.defId === defId)), defId).toEqual([]);
      // Every recipe claim and entrance, at each D: delays inside the entry, every end within its tail, each delay its share of D.
      const shares = new Map<string, number[]>();
      for (const D of [250, 400, 800, 1400]) {
        const env = envOf();
        for (const planned of planStream(events, view, env)) {
          const at = { ...planned.entry, durationMs: D };
          for (const event of planned.entry.events) {
            // Its entrance is held to D below, landing at FX_SLAM_AT of it.
            if (event.type === "summoned" && event.defId === defId) continue;
            const claim = planCardFx(event, { entry: at, view: at.view, env, D });
            if (claim === null) continue;
            // A zero-length entry (Reduce Motion) draws no recipe.
            expect(planCardFx(event, { entry: { ...at, durationMs: 0 }, view: at.view, env, D: 0 }), `${defId} ${event.type} D=0`).toBeNull();
            claim.cues.forEach((cue, k) => {
              const label = `${defId} ${event.type} ${cue.kind} D=${String(D)}`;
              expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
              expect(cue.delayMs, label).toBeLessThanOrEqual(D);
              if (cue.kind === "shot" || cue.kind === "projectile") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
              expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
              const key = `${event.type} ${String(k)}`;
              shares.set(key, [...(shares.get(key) ?? []), cue.delayMs / D]);
            });
          }
        }
      }
      for (const [key, seen] of shares) {
        const [first] = seen;
        for (const share of seen) expect(Math.abs(share - (first ?? 0)), `${defId} ${key}`).toBeLessThanOrEqual(0.01);
      }
      // Its entrance, at each D, lands at FX_SLAM_AT of it.
      if (entry.entrance !== undefined) {
        for (const D of [250, 400, 800, 1400]) {
          const cues = ENTRANCES[entry.entrance]({ D, tgt: testid.zone("you", "units", 3), intensity: FX_INTENSITY_SCALE.normal, landMs: frac(FX_SLAM_AT, D) });
          for (const cue of cues) {
            expect(cue.delayMs, `${defId} ${cue.kind}`).toBeLessThanOrEqual(D);
            expect(endOf(cue), `${defId} ${cue.kind}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          }
          expect(cues.some((cue) => cue.kind === "shake" && cue.delayMs === frac(FX_SLAM_AT, D)), `${defId} D=${String(D)}`).toBe(true);
        }
      }
      // Each recipe answers only its own events: a stray damage claims nothing.
      expect(CARD_RECIPES[entry.key]({ type: "damage", sourceId: null, targetId: "hero-p2", amount: 1, combat: false }, { entry: planStream(events, view)[0]!.entry, view, env: envOf(), D: 400 }, { player: "p1", instanceId: "x", defId, castOnDraw: false, step: 0, seen: {} }), defId).toBeNull();
    }
  });

  it("R3389001 In Too Deep and BOOM! Big Max plan the most particles", () => {
    const particles = CLASSIC_SIGNATURES.map((defId) => {
      const { events, view } = galleryStream(defId);
      const count = planStream(events, view)
        .flatMap((p) => p.cues)
        .reduce((sum, cue) => sum + (cue.kind === "burst" ? cue.count : 0), 0);
      return [defId, count] as const;
    });
    const ranked = [...particles].sort((a, b) => b[1] - a[1]).map(([defId]) => defId);
    expect(new Set(ranked.slice(0, 2))).toEqual(new Set(["classic-090", "classic-080"]));
  });
});
