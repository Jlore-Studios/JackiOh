// R3407000: Scornboi's signature (scornboi.ts), planned as FxLayer plans it: a crimson gate slams down over
// his zone and he strides through it with his short list; each Unit his aura exiles as he lands is picked out
// by a crimson beam and dragged into a void hatch, and on the first the Units he spares glint. His gallery play
// within a Legendary's ceiling, R200 at any length, nothing with the effects off; the beam reaches only a card
// the view shows face up and no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { testid } from "../../../game/contract.ts";
import { card, fullBoardView, unit } from "../../../test/fixtures.ts";
import { beatsOf, cuesOfRun, envOf, expectCuesR200, expectGalleryWithinCeiling, expectNoCard, planWithOffsets } from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_CRACK_TAIL_MS, FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardFxResult } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { scornSweepFixture } from "./scornboi.ts";

const SCORNBOI = "stinkyscape-115";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const played = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: "scorn", defId: SCORNBOI, costPaid: 3 });
const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "scorn", defId: SCORNBOI, row: "units", lane: 3 });

describe("R3407000 Scornboi's signature", () => {
  it("R3407000 his table entry is his signature", () => {
    expect(FX_CARD_TABLE.get(SCORNBOI)).toEqual({ kind: "bespoke", key: "scornSweep", entrance: "scornGate", aura: { look: "searchlight", palette: "Dark/blood", scope: "tile" }, signature: true });
  });

  it("R3407000 a crimson gate slams down over his zone, cracking its foot, and he strides through with his short list", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.scornGate({ D, tgt, intensity: 1, view: fullBoardView(), landing: landing("p1") });
    expect(kinds(cues)).toEqual(["shot", "ring", "crack", "burst", "sheen", "burst", "figure", "shake"]);
    expect(cues[0]).toMatchObject({ to: tid(tgt, { x: 0.5, y: 1 }) });
    expect(cues[2]).toMatchObject({ at: tid(tgt, { x: 0.5, y: 1 }), delayMs: slam });
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.scornGate, delayMs: slam });
  });

  it("R3407000 each Unit his aura exiles is beamed into a void hatch, and on the first the Units he spares glint", () => {
    const view = fullBoardView();
    const D = 350;
    const exiles = scornSweepFixture(view, "scorn", "p1");
    expect(exiles).toHaveLength(1);
    const [first] = beatsOf(exiles, view, D, [played("p1"), landing("p1")]);
    expect(first?.row).toBe("replace");
    const foe = exiles[0]?.type === "exiled" ? exiles[0].instanceId : "";
    expect(kinds(first?.cues).slice(0, 3)).toEqual(["shot", "ring", "burst"]);
    expect(first?.cues[0]).toMatchObject({ from: tid(testid.zone("you", "units", 3)), to: tid(testid.card(foe)) });
    const shown = [...view.you.units, ...view.opponent.units].filter((unit) => unit !== null && unit.defId !== "hidden" && unit.instanceId !== foe).length;
    expect(first?.cues.filter((cue) => cue.kind === "sheen")).toHaveLength(shown);
    // A second exile beams again, with no second glint; a card the board does not show draws nothing.
    const second = view.opponent.units.find((unit) => unit !== null && unit.instanceId !== foe && unit.defId !== "hidden");
    const [, again] = beatsOf([...exiles, { type: "exiled", instanceId: second?.instanceId ?? "", defId: second?.defId ?? "", owner: "p2" }], view, D, [played("p1"), landing("p1")]);
    expect(kinds(again?.cues)).toEqual(["shot", "ring", "burst"]);
    expect(beatsOf([{ type: "exiled", instanceId: "not-shown", defId: "core-002", owner: "p2" }], view, D, [played("p1")])).toEqual([null]);
    // Another card's exile is not his sweep.
    expect(beatsOf(exiles, view, D, [{ type: "cardPlayed", player: "p1", instanceId: "x", defId: "core-002", costPaid: 1 }])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "units", 3);
    for (const D of DS) {
      expectCuesR200(ENTRANCES.scornGate({ D, tgt, intensity: 1, view, landing: landing("p1") }), D, "scornGate", ENTRANCE_TRAUMA.scornGate);
      const sweep = beatsOf(scornSweepFixture(view, "scorn", "p1"), view, D, [played("p1"), landing("p1")]).flatMap((beat) => beat?.cues ?? []);
      expect(sweep.length).toBeGreaterThan(0);
      expectCuesR200(sweep, D, "scornSweep");
      expectNoCard(sweep, SCORNBOI);
    }
    expect(cuesOfRun(planWithOffsets([played("p1"), landing("p1"), ...scornSweepFixture(view, "scorn", "p1")], view, envOf({ intensity: FX_INTENSITY_SCALE.off })))).toEqual([]);
  });

  it("R3407000 his gallery play ends within a Legendary's ceiling", () => {
    expectGalleryWithinCeiling(SCORNBOI);
  });
});

const BLOOD = paletteLook("Dark/blood");
const exile = (unit: { instanceId: string; defId: string; owner: PlayerId }): GameEvent => ({ type: "exiled", instanceId: unit.instanceId, defId: unit.defId, owner: unit.owner });
const landingAt = (instanceId: string, lane: number): GameEvent => ({ type: "summoned", player: "p1", instanceId, defId: SCORNBOI, row: "units", lane });

/** The board with him standing in lane 3 of yours, and on the other side an empty lane 1 and a face-down Unit in lane 2. */
function standingBoard(): PlayerView {
  const view = fullBoardView();
  const him = unit("p1", { instanceId: "scorn", defId: SCORNBOI });
  return {
    ...view,
    you: { ...view.you, units: view.you.units.map((u, k) => (k === 2 ? him : u)) },
    opponent: { ...view.opponent, units: view.opponent.units.map((u, k) => (k === 0 ? null : k === 1 && u !== null ? { ...u, defId: "hidden" } : u)) },
  };
}

/** The card testids of `view`'s Units in lanes `lanes` of `side`. */
function cardsAt(view: PlayerView, side: "you" | "opponent", lanes: readonly number[]): FxAnchor[] {
  return lanes.map((lane) => tid(testid.card(view[side].units[lane - 1]?.instanceId ?? "")));
}

/** His sweep's answer for each of `events`, all in one entry of D after `before`, as FxLayer plans an entry of many events. */
function sweepInOneEntry(events: readonly GameEvent[], view: PlayerView, D: number, before: readonly GameEvent[], env: FxPlanEnv = envOf()): (CardFxResult | null)[] {
  env.memory.remember(before);
  env.memory.remember(events);
  const entry = { ...galleryEntry([...events], view), durationMs: D };
  return events.map((event) => planCardFx(event, { entry, view, env, D }));
}

const glintsOf = (result: CardFxResult | null | undefined): FxAnchor[] => (result?.cues ?? []).flatMap((cue) => (cue.kind === "sheen" ? [cue.at] : []));
const beamFrom = (result: CardFxResult | null | undefined): FxAnchor | undefined => {
  const shot = result?.cues[0];
  return shot?.kind === "shot" ? shot.from : undefined;
};

describe("R3407000 Scornboi's signature, beat by beat", () => {
  it("R3407000 the gate: a crimson bolt from above his zone to its foot, a ring, a crack to the tail, dust over the area, his coat's sheen and smoke, and his small list lingering past the entry on his side", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const [shot, ring, crack, dust, sheen, smoke, list] = ENTRANCES.scornGate({ D, tgt, intensity: 1, view: fullBoardView(), landing: landing("p1") });
    expect(shot).toMatchObject({ kind: "shot", shot: { motion: "bolt" }, to: tid(tgt, FOOT), colors: BLOOD?.colors });
    expect(shot?.kind === "shot" && shot.delayMs + shot.flightMs).toBe(slam);
    expect(shot?.delayMs).toBeGreaterThan(0);
    expect(ring).toMatchObject({ kind: "ring", at: tid(tgt), delayMs: slam });
    expect(crack).toEqual({ kind: "crack", at: tid(tgt, FOOT), delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS });
    expect(dust).toMatchObject({ kind: "burst", preset: "dust", spread: "area", at: tid(tgt, FOOT), delayMs: slam });
    expect(sheen).toMatchObject({ kind: "sheen", at: tid(tgt) });
    expect(sheen?.kind === "sheen" && sheen.delayMs + sheen.durationMs).toBe(D);
    expect(smoke).toMatchObject({ kind: "burst", preset: "smoke", spread: "point", at: tid(tgt), delayMs: sheen?.delayMs });
    expect(list).toMatchObject({ kind: "figure", figure: "scroll", size: "small", side: "you" });
    // His list hangs on past the entry, gone within the tail (R200).
    const end = list?.kind === "figure" ? list.delayMs + list.durationMs : 0;
    expect(end).toBeGreaterThan(D);
    expect(end).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
  });

  it("R3407000 his list faces the side he lands on; with no view or no landing it faces none", () => {
    const tgt = testid.zone("you", "units", 3);
    const view = fullBoardView();
    const listOf = (cues: readonly FxCue[]): FxCue | undefined => cues.find((cue) => cue.kind === "figure");
    expect(listOf(ENTRANCES.scornGate({ D: 400, tgt, intensity: 1, view, landing: landing("p2") }))).toMatchObject({ side: "opponent" });
    for (const plan of [{ view }, { landing: landing("p1") }, {}]) {
      const list = listOf(ENTRANCES.scornGate({ D: 400, tgt, intensity: 1, ...plan }));
      expect(list?.kind, JSON.stringify(Object.keys(plan))).toBe("figure");
      expect(list !== undefined && "side" in list, JSON.stringify(Object.keys(plan))).toBe(false);
    }
  });

  it("R3407000 his aura exiles the other side's first face-up Unit, an empty lane and a face-down card passed over, and none where it shows none", () => {
    const view = standingBoard();
    const foe = view.opponent.units[2];
    if (foe === null || foe === undefined) throw new Error("no foe");
    expect(scornSweepFixture(view, "scorn", "p1")).toEqual([exile(foe)]);
    // Played by the other seat, his aura reads the viewer's side.
    const mine = view.you.units[0];
    if (mine === null || mine === undefined) throw new Error("no unit");
    expect(scornSweepFixture(view, "scorn", "p2")).toEqual([exile(mine)]);
    const none: PlayerView = { ...view, opponent: { ...view.opponent, units: view.opponent.units.map((u) => (u === null ? null : { ...u, defId: "hidden" })) } };
    expect(scornSweepFixture(none, "scorn", "p1")).toEqual([]);
  });

  it("R3407000 R202 standing on the board, he beams from his own card; the spared that glint are the face-up Units but him and the exiled", () => {
    const view = standingBoard();
    const foe = view.opponent.units[2];
    if (foe === null || foe === undefined) throw new Error("no foe");
    const D = 350;
    const [first] = beatsOf([exile(foe)], view, D, [played("p1"), landing("p1")]);
    const target = tid(testid.card(foe.instanceId));
    const [beam, hatch, ring] = first?.cues ?? [];
    expect(beam).toMatchObject({ kind: "shot", shot: { motion: "beam" }, from: tid(testid.card("scorn")), to: target, colors: BLOOD?.colors });
    const land = beam?.kind === "shot" ? beam.delayMs + beam.flightMs : -1;
    expect(beam?.delayMs).toBeGreaterThan(0);
    expect(hatch).toMatchObject({ kind: "ring", preset: "void", at: target, delayMs: land });
    expect(ring).toMatchObject({ kind: "burst", preset: "void", spread: "ring", at: target, delayMs: land });
    // Lane 1 empty, lane 2 face down, lane 3 exiled; and on your side, him in lane 3.
    expect(glintsOf(first)).toEqual([...cardsAt(view, "you", [1, 2, 4, 5]), ...cardsAt(view, "opponent", [4, 5])]);
    // His own exile, or another event on a card he shows, draws none of his sweep.
    expect(beatsOf([exile({ instanceId: "scorn", defId: SCORNBOI, owner: "p1" })], view, D, [played("p1"), landing("p1")])).toEqual([null]);
    expect(beatsOf([{ type: "destroyed", instanceId: foe.instanceId, defId: foe.defId, owner: "p2", controller: "p2", attack: foe.attack, maxHealth: foe.maxHealth, killerId: null }], view, D, [played("p1"), landing("p1")])).toEqual([null]);
  });

  it("R3407000 off the board he beams from the zone his latest summoned named, else his hero; a card in a hand is never his anchor nor his target (R202)", () => {
    const view = fullBoardView();
    const foe = view.opponent.units[0];
    if (foe === null || foe === undefined) throw new Error("no foe");
    const from = (before: readonly GameEvent[], board: PlayerView = view): FxAnchor | undefined => beamFrom(beatsOf([exile(foe)], board, 350, before)[0]);
    const zone = (lane: number): FxAnchor => tid(testid.zone("you", "units", lane));
    expect(from([played("p1"), landingAt("scorn", 2), landingAt("scorn", 4)])).toEqual(zone(4));
    // Another card landing after him, or a later event of his own, is not where he stands.
    expect(from([played("p1"), landingAt("scorn", 3), landingAt("other", 5)])).toEqual(zone(3));
    expect(from([played("p1"), landingAt("scorn", 3), { type: "buffed", instanceId: "scorn", attack: 1, health: 1 }])).toEqual(zone(3));
    expect(from([played("p1")])).toEqual(tid(testid.hero("you")));
    // His card shown in the viewer's hand is no anchor; a card in a hand is no target.
    const inHand: PlayerView = { ...view, you: { ...view.you, hand: [card({ instanceId: "scorn", defId: SCORNBOI }), card({ instanceId: "held" })] } };
    expect(from([played("p1"), landingAt("scorn", 3)], inHand)).toEqual(zone(3));
    expect(beatsOf([exile({ instanceId: "held", defId: "core-001", owner: "p1" })], inHand, 350, [played("p1"), landing("p1")])).toEqual([null]);
  });

  it("R3407000 the spared are those the view after the action still shows", () => {
    const view = fullBoardView();
    const foe = view.opponent.units[0];
    const gone = view.you.units[1];
    if (foe === null || foe === undefined || gone === null || gone === undefined) throw new Error("no units");
    const next: PlayerView = { ...view, you: { ...view.you, units: view.you.units.map((u, k) => (k === 1 ? null : u)) } };
    const [first] = beatsOf([exile(foe)], view, 350, [played("p1"), landing("p1")], envOf({ next }));
    expect(glintsOf(first)).toEqual([...cardsAt(view, "you", [1, 3, 4, 5]), ...cardsAt(view, "opponent", [2, 3, 4, 5])]);
  });

  it("R3407000 the spared glint at the first exile of his play alone: not at a second in its entry, and still at a first behind another event", () => {
    const view = fullBoardView();
    const [a, b] = view.opponent.units;
    if (a === null || a === undefined || b === null || b === undefined) throw new Error("no foes");
    const before = [played("p1"), landing("p1")];
    const [first, second] = sweepInOneEntry([exile(a), exile(b)], view, 350, before);
    expect(glintsOf(first)).toHaveLength(8);
    expect(glintsOf(second)).toEqual([]);
    const buff: GameEvent = { type: "buffed", instanceId: b.instanceId, attack: 1, health: 1 };
    const [, behind] = sweepInOneEntry([buff, exile(a)], view, 350, before);
    expect(glintsOf(behind)).toHaveLength(9);
  });

  it("R2797000 under a hook of his (a Death's), an exile draws none of his sweep", () => {
    const view = fullBoardView();
    const foe = view.opponent.units[0];
    if (foe === null || foe === undefined) throw new Error("no foe");
    const env = envOf();
    env.memory.remember([played("p1"), landing("p1"), exile(foe)]);
    const play = env.memory.resolving();
    if (play === undefined) throw new Error("no play resolving");
    const entry = { ...galleryEntry([exile(foe)], view), durationMs: 350 };
    const p = { entry, view, env, D: 350 };
    expect(CARD_RECIPES.scornSweep(exile(foe), p, { ...play, hook: "death" })).toBeNull();
    expect(CARD_RECIPES.scornSweep(exile(foe), p, play)?.cues[0]?.kind).toBe("shot");
  });
});

describe("R3407000 Scornboi's signature as a page load builds it", () => {
  it("R3407000 his gate falls from above his zone, his list hangs at its top, all in his crimson", async () => {
    // His crimson and the gate's and list's points are built as the module loads: read from a module loaded inside the test, as cards/settings.test.ts does.
    vi.resetModules();
    const fresh = await import("./scornboi.ts");
    const tgt = testid.zone("you", "units", 3);
    const cues = fresh.scornGate({ D: 400, tgt, intensity: 1, view: fullBoardView(), landing: landing("p1") });
    const shot = cues.find((cue) => cue.kind === "shot");
    const list = cues.find((cue) => cue.kind === "figure");
    // Centred above the zone, and at its top half's middle.
    expect(shot?.kind === "shot" ? shot.from : undefined).toEqual(tid(tgt, { x: 0.5, y: -0.6 }));
    expect(list?.kind === "figure" ? list.at : undefined).toEqual(tid(tgt, { x: 0.5, y: 0.25 }));
    expect(BLOOD?.colors).not.toEqual(FLAVOR_LOOKS[FLAVOR_FALLBACK].colors);
    expect(shot?.kind === "shot" ? shot.colors : undefined).toEqual(BLOOD?.colors);
  });
});
