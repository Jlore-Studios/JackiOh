// R3407000: IN33DH3, IN33DH3 Prime and the Wardrum Blob (in33dh3.ts), planned as FxLayer plans them (remember,
// then plan): each table entry, each entrance at D 400 and after a hold, IN33DH3's Death as a data packet into
// the deck, the Prime's Cry over the Blob it summons, the Blob's two drums and its ripple to the helpful
// cast's target; each gallery run within its Legendary ceiling, the beats the game plays outside a play
// within it from their markers, nothing with the effects off, R200 at five D's, and from the other seat no
// cue names a card or a hand card while every beat still draws (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { HIDDEN_ID, animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { ceilingOf, cuesOfRun, describeRun, endOf, envOf, expectGalleryWithinCeiling, galleryRun, latestEnd, planWithOffsets } from "../../../test/signaturePass.ts";
import { frac, tid } from "../../build.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_SLAM_AT, SIGNATURE_PERMANENT_MAX_TRAUMA } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import { paletteLook } from "../../flavors.ts";
import { galleryEntry } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { blobDrumFixture, dataPacketFixture, primeConductFixture } from "./in33dh3.ts";

const IN33DH3 = "stinkyscape-147";
const PRIME = "stinkyscape-147-1";
const BLOB = "stinkyscape-147-2";
const D = 400;
const DS = [120, 200, 400, 732, 1400] as const;
const WAIT = 200;
const CARD_ID = /(core|classic|meditative|stinkyscape|traininggrounds)[a-z]*-\d/;
const HOLOGRAM = paletteLook("Tech/hologram")?.colors;

type Planned = { cues: FxCue[]; row: string | undefined };

const beats = (cues: readonly FxCue[]): string[] => cues.map((cue) => `${cue.kind}@${String(cue.delayMs)}`);
const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: true, costPaid: 1 });
const unitLanding = (player: PlayerId, instanceId: string, defId: string): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId, defId, row: "units", lane: 3 });
const fieldLanding = (player: PlayerId, instanceId: string, defId: string, lane = 1): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId, defId, row: "backrow", lane });
const death = (player: PlayerId, instanceId: string): GameEvent => ({ type: "hookResolving", sourceId: instanceId, defId: IN33DH3, controller: player, hook: "death" });
const turnHook = (player: PlayerId, instanceId: string): GameEvent => ({ type: "hookResolving", sourceId: instanceId, defId: BLOB, controller: player, hook: "startOfTurn" });
const shuffled = (player: PlayerId, hidden = false): GameEvent => ({ type: "shuffledIn", player, instanceId: hidden ? HIDDEN_ID : "pr", defId: hidden ? HIDDEN_ID : PRIME, position: 0 });
const announced = (player: PlayerId, targets: string[]): GameEvent => ({ type: "cardAnnounced", player, instanceId: "helped", defId: "core-016", cardType: "Spell", costPaid: 0, targets });

/** Each of `events` as its own entry lasting `ms`, after `before` is remembered: remember, then plan through the dispatch. */
function planEach(events: readonly GameEvent[], ms: number, before: readonly GameEvent[] = [], view: PlayerView = fullBoardView(), env: FxPlanEnv = envOf()): Planned[] {
  env.memory.remember(before);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: ms };
    const result = planCardFx(event, { entry, view, env, D: ms });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

/** R200 for one beat planned at `ms`: it draws, starts inside its entry, lands by its end and fades within FX_MAX_TAIL_MS. */
function expectInside(cues: readonly FxCue[], ms: number, where: string): void {
  expect(cues.length, `${where} at D=${String(ms)}`).toBeGreaterThan(0);
  for (const cue of cues) {
    const label = `${where} ${cue.kind} at D=${String(ms)}`;
    expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
    expect(cue.delayMs, label).toBeLessThanOrEqual(ms);
    if (cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(ms);
    if (cue.kind === "ghost") expect(cue.delayMs + cue.durationMs, label).toBeLessThanOrEqual(ms);
    expect(endOf(cue), label).toBeLessThanOrEqual(ms + FX_MAX_TAIL_MS);
    if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(SIGNATURE_PERMANENT_MAX_TRAUMA);
  }
}

/** The instance id of the viewer's face-up backrow card in lane 1 (the Blob standing there). */
function firstFieldId(view: PlayerView): string {
  const slot = view.you.backrow[0];
  return slot !== null && slot !== undefined && !slot.faceDown ? slot.instanceId : "";
}

/** The events as the other seat reads them: a deck's or a hand's card hidden, its prompts dropped (R202). */
function redacted(events: readonly GameEvent[]): GameEvent[] {
  return events.flatMap((event): GameEvent[] => {
    if (event.type === "promptOpened") return [];
    if (event.type === "shuffledIn" || event.type === "addedToHand" || event.type === "drawn") return [{ ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID }];
    return [event];
  });
}

describe("R3407000 IN33DH3, IN33DH3 Prime and the Wardrum Blob", () => {
  it("R3407000 the three are signatures with their entrances, recipes and auras", () => {
    expect(FX_CARD_TABLE.get(IN33DH3)).toEqual({ kind: "bespoke", key: "dataPacket", entrance: "in33dh3Drop", aura: { look: "aiwisp", palette: "Tech/hologram", scope: "tile" }, signature: true });
    expect(FX_CARD_TABLE.get(PRIME)).toEqual({ kind: "bespoke", key: "primeConduct", entrance: "primeCrown", aura: { look: "crownpulse", palette: "Tech/hologram", scope: "tile" }, signature: true });
    expect(FX_CARD_TABLE.get(BLOB)).toEqual({ kind: "bespoke", key: "blobDrum", entrance: "blobPlop", aura: { look: "drumbeat", palette: "Tech/hologram", scope: "zone" }, signature: true });
  });

  it("R3407000 IN33DH3's entrance: a headset crackles, the AI companion materialises first and points, he drops at the slam and cracks his knuckles", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.in33dh3Drop({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing: unitLanding("p1", "in", IN33DH3) });
    expect(beats(cues)).toEqual(["burst@0", "figure@40", "shot@120", `burst@${String(slam)}`, `ring@${String(slam)}`, "burst@300", "burst@348", `shake@${String(slam)}`]);
    expect(cues[0]).toMatchObject({ preset: "spark", at: tid(tgt, { x: 0.5, y: 0 }), colors: HOLOGRAM });
    expect(cues[1]).toMatchObject({ figure: "spirit", at: tid(tgt), side: "you", durationMs: D - 40 + 300 });
    expect(cues[2]).toMatchObject({ shot: { motion: "beam" }, from: tid(tgt, { x: 0.5, y: 0.2 }), to: tid(tgt, FOOT), flightMs: slam - 120 });
    expect(cues[3]).toMatchObject({ preset: "dust", at: tid(tgt, FOOT) });
    expect(cues[4]).toMatchObject({ at: tid(tgt) });
    for (const knuckle of cues.slice(5, 7)) expect(knuckle).toMatchObject({ preset: "spark", spread: "point", at: tid(tgt, { x: 0.5, y: 0.6 }) });
    expect(cues[7]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.in33dh3Drop, delayMs: slam });
    // The other seat's IN33DH3: the companion faces it.
    expect(ENTRANCES.in33dh3Drop({ D, tgt: testid.zone("opponent", "units", 3), intensity: 1, view, landing: unitLanding("p2", "in", IN33DH3) })[1]).toMatchObject({ side: "opponent" });
  });

  it("R3407000 IN33DH3 Prime's entrance: his command coat sweeps in, his neural crown lights, the hologram kneels at his side and he lands", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "units", 3);
    const cues = ENTRANCES.primeCrown({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing: unitLanding("p1", "pr", PRIME) });
    expect(beats(cues)).toEqual(["burst@0", "ring@120", "burst@120", "figure@160", "ring@240", "shake@240"]);
    expect(cues[0]).toMatchObject({ preset: "smoke", spread: "area", at: tid(tgt), colors: paletteLook("Dark/shadow")?.colors });
    expect(cues[1]).toMatchObject({ at: tid(tgt, { x: 0.5, y: 0.2 }) });
    expect(cues[2]).toMatchObject({ spread: "ring", at: tid(tgt, { x: 0.5, y: 0.2 }), colors: HOLOGRAM });
    expect(cues[3]).toMatchObject({ figure: "spirit", at: tid(tgt, { x: 0.85, y: 0.75 }), side: "you", size: "small", durationMs: D - 160 + 300 });
    expect(cues[4]).toMatchObject({ at: tid(tgt) });
    expect(cues[5]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.primeCrown, delayMs: 240 });
  });

  it("R3407000 the Wardrum Blob's entrance: a drip falls, it plops down at the slam, settles on its pedestal and wobbles twice", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "backrow", 1);
    const cues = ENTRANCES.blobPlop({ D, tgt, intensity: FX_INTENSITY_SCALE.normal, view, landing: fieldLanding("p1", "bl", BLOB) });
    expect(beats(cues)).toEqual(["burst@40", "burst@240", "ring@240", "sheen@240", "burst@300", "burst@348", "shake@240"]);
    expect(cues[0]).toMatchObject({ spread: "point", at: tid(tgt, { x: 0.5, y: 0 }), colors: HOLOGRAM });
    expect(cues[1]).toMatchObject({ spread: "area", at: tid(tgt, FOOT) });
    expect(cues[3]).toEqual({ kind: "sheen", at: tid(tgt), delayMs: 240, durationMs: D - 240 });
    for (const wobble of cues.slice(4, 6)) expect(wobble).toMatchObject({ spread: "ring", at: tid(tgt) });
    expect(cues[6]).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.blobPlop, delayMs: 240 });
  });

  it("R3407000 each entrance follows its entry's hold, landing at the slam of the motion after it", () => {
    const view = fullBoardView();
    const plans = [
      ["in33dh3Drop", testid.zone("you", "units", 3), unitLanding("p1", "in", IN33DH3)],
      ["primeCrown", testid.zone("you", "units", 3), unitLanding("p1", "pr", PRIME)],
      ["blobPlop", testid.zone("you", "backrow", 1), fieldLanding("p1", "bl", BLOB)],
    ] as const;
    for (const [key, tgt, landing] of plans) {
      const cues = ENTRANCES[key]({ D, tgt, intensity: 1, view, landing, wait: WAIT });
      for (const cue of cues) expect(cue.delayMs, `${key} ${cue.kind}`).toBeGreaterThanOrEqual(WAIT);
      expect(cues.find((cue) => cue.kind === "shake")?.delayMs, key).toBe(WAIT + frac(FX_SLAM_AT, D - WAIT));
      expectInside(cues, D, key);
    }
  });

  it("R3407000 IN33DH3's Death: he dissolves where he lies at its marker, and the Prime goes deckward as a teal data packet and a card back", () => {
    const view = fullBoardView();
    const graveyard = tid(animTestid.graveyard("you"));
    const deck = tid(animTestid.library("you"));
    // In combat: his `destroyed` comes before the hook opens, inside no play, and draws its own row only.
    const [destroyed, marker, packet] = planEach([{ type: "destroyed", instanceId: "in", defId: IN33DH3, owner: "p1" }, death("p1", "in"), shuffled("p1")], D, [], view);
    expect(destroyed).toEqual({ cues: [], row: undefined });
    expect(marker?.row).toBe("keep");
    expect(marker?.cues).toEqual([expect.objectContaining({ kind: "burst", preset: "circuit", spread: "area", at: graveyard, delayMs: 0, colors: HOLOGRAM })]);
    expect(packet?.row).toBe("replace");
    expect(beats(packet?.cues ?? [])).toEqual(["shot@40", "ghost@40", "ring@260", "burst@260"]);
    expect(packet?.cues[0]).toMatchObject({ shot: { motion: "bolt", trail: "circuit" }, from: graveyard, to: deck, flightMs: 220, colors: HOLOGRAM });
    expect(packet?.cues[1]).toEqual({ kind: "ghost", from: graveyard, to: deck, delayMs: 40, durationMs: 220 });
    expect(packet?.cues[2]).toMatchObject({ at: deck });
    expect(packet?.cues[3]).toMatchObject({ preset: "circuit", at: deck });
    // While the view still shows his card, the packet leaves it.
    const him = view.you.units[2]?.instanceId ?? "";
    const shown = planEach([death("p1", him), shuffled("p1")], D, [], view);
    expect(shown[0]?.cues[0]).toMatchObject({ at: tid(testid.card(him)) });
    expect(shown[1]?.cues[0]).toMatchObject({ from: tid(testid.card(him)), to: deck });
    // His play draws nothing of the recipe's: its landing is his entrance alone.
    const landing = unitLanding("p1", "in", IN33DH3);
    const own = planEach([played("p1", "in", IN33DH3), landing, resolved("p1", "in", IN33DH3)], D, [], view);
    expect(own[0]).toEqual({ cues: [], row: undefined });
    expect(own[1]).toEqual({ cues: ENTRANCES.in33dh3Drop({ D, tgt: testid.zone("you", "units", 3), intensity: 1, view, landing, wait: 0 }), row: "replace" });
    expect(own[2]).toEqual({ cues: [], row: undefined });
  });

  it("R3407000 the Prime's Cry: as the Blob lands in his backrow he raises a hand, rays light his backrow and a beam hums from him to it, beside the Blob's own entrance", () => {
    const view = fullBoardView();
    const blob = fieldLanding("p1", "bl", BLOB, 2);
    const zone = tid(testid.zone("you", "backrow", 2));
    const plan = (before: readonly GameEvent[], from: ReturnType<typeof tid>): void => {
      const [cry] = planEach([blob], D, before, view);
      const entrance = ENTRANCES.blobPlop({ D, tgt: testid.zone("you", "backrow", 2), intensity: 1, view, landing: blob, wait: 0 });
      expect(cry?.row).toBe("replace");
      expect(cry?.cues.slice(0, entrance.length)).toEqual(entrance);
      const recipe = cry?.cues.slice(entrance.length) ?? [];
      expect(beats(recipe)).toEqual(["burst@0", "rays@40", "ring@80", "shot@80"]);
      expect(recipe[0]).toMatchObject({ spread: "point", at: from, colors: HOLOGRAM });
      expect(recipe[1]).toMatchObject({ tone: "radiant", at: tid(animTestid.backrow("you")) });
      expect(recipe[2]).toMatchObject({ at: zone });
      expect(recipe[3]).toMatchObject({ shot: { motion: "beam" }, from, to: zone, flightMs: 160 });
    };
    // His landing zone while the view shows no card of his (the gallery's), his card while it does, else his hero.
    plan([played("p1", "pr", PRIME), unitLanding("p1", "pr", PRIME)], tid(testid.zone("you", "units", 3)));
    const him = view.you.units[2]?.instanceId ?? "";
    plan([played("p1", him, PRIME)], tid(testid.card(him)));
    plan([played("p1", "pr", PRIME)], tid(testid.hero("you")));
    // His own landing is his entrance alone; the other player's backrow summon and his units' draw nothing of the Cry.
    const own = planEach([unitLanding("p1", "pr", PRIME)], D, [played("p1", "pr", PRIME)], view);
    expect(own[0]?.cues).toEqual(ENTRANCES.primeCrown({ D, tgt: testid.zone("you", "units", 3), intensity: 1, view, landing: unitLanding("p1", "pr", PRIME), wait: 0 }));
    const theirs = planEach([{ type: "summoned", player: "p2", instanceId: "x", defId: "core-020", row: "backrow", lane: 2 }], D, [played("p1", "pr", PRIME)], view);
    expect(theirs[0]).toEqual({ cues: [], row: undefined });
  });

  it("R3407000 the Blob's Start of turn: it drums at its marker, drums again at the helpful cast's announce and ripples to its first shown target, else its controller's hero", () => {
    const view = fullBoardView();
    const blobId = firstFieldId(view);
    const zone = tid(testid.card(blobId));
    const foe = view.opponent.units[1]?.instanceId ?? "";
    const [marker, announce] = planEach([turnHook("p1", blobId), announced("p1", [foe])], D, [{ type: "turnStarted", player: "p1", turn: 3 }], view);
    expect(marker).toEqual({ cues: [expect.objectContaining({ kind: "ring", at: zone, delayMs: 0 }), expect.objectContaining({ kind: "burst", spread: "ring", at: zone, delayMs: 0, colors: HOLOGRAM })], row: "keep" });
    expect(announce?.row).toBe("keep");
    expect(beats(announce?.cues ?? [])).toEqual(["ring@120", "burst@120", "shot@120"]);
    expect(announce?.cues[2]).toMatchObject({ shot: { motion: "beam" }, from: zone, to: tid(testid.card(foe)), flightMs: 180 });
    // A hero target, none, a hidden one, a hand card's: the hero it names, else its controller's.
    const hand = Array.isArray(view.you.hand) ? (view.you.hand[0]?.instanceId ?? "") : "";
    const to = (targets: string[]): unknown => planEach([turnHook("p1", blobId), announced("p1", targets)], D, [], view)[1]?.cues[2];
    expect(to(["hero-p2"])).toMatchObject({ to: tid(testid.hero("opponent")) });
    for (const targets of [[], [HIDDEN_ID], [hand], ["gone"]]) expect(to(targets), JSON.stringify(targets)).toMatchObject({ to: tid(testid.hero("you")) });
    // The other player's announce, and the same events inside a play rather than its hook, draw nothing of the drum.
    expect(planEach([turnHook("p1", blobId), announced("p2", [])], D, [], view)[1]).toEqual({ cues: [], row: undefined });
    expect(planEach([announced("p1", [])], D, [played("p1", blobId, BLOB)], view)[0]).toEqual({ cues: [], row: undefined });
  });

  it("R3407000 each gallery run fits its Legendary ceiling", () => {
    for (const id of [IN33DH3, PRIME, BLOB]) {
      expect(ceilingOf(id), id).toBe(1600);
      expectGalleryWithinCeiling(id);
    }
    // The galleries play the Death, the Cry's Blob and the turn hook.
    expect(galleryRun(IN33DH3).events.map((event) => event.type)).toEqual(["cardPlayed", "summoned", "hookResolving", "shuffledIn", "cardResolved"]);
    expect(galleryRun(PRIME).events.map((event) => event.type)).toEqual(["cardPlayed", "summoned", "summoned", "cardResolved"]);
    expect(galleryRun(BLOB).events.map((event) => event.type)).toEqual(["cardPlayed", "summoned", "hookResolving", "cardAnnounced", "cardResolved"]);
  });

  it("R3407000 the beats the game plays outside a play end within the ceiling from their markers: IN33DH3's Death in combat, the Blob's turn hook", () => {
    const view = fullBoardView();
    const blobId = firstFieldId(view);
    const runs = [
      [IN33DH3, planWithOffsets([death("p1", "in"), shuffled("p1")], view)],
      [BLOB, planWithOffsets([turnHook("p1", blobId), announced("p1", [view.opponent.units[0]?.instanceId ?? ""])], view)],
    ] as const;
    for (const [id, run] of runs) {
      expect(cuesOfRun(run).length, id).toBeGreaterThan(0);
      expect(latestEnd(run), `${id}\n${describeRun(run)}`).toBeLessThanOrEqual(ceilingOf(id));
    }
  });

  it("R3407000 nothing draws with the effects off", () => {
    for (const id of [IN33DH3, PRIME, BLOB]) expect(cuesOfRun(galleryRun(id, envOf(FX_INTENSITY_SCALE.off)).run), id).toEqual([]);
    expect(cuesOfRun(planWithOffsets([death("p1", "in"), shuffled("p1")], fullBoardView(), envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 R200 each entrance and recipe beat starts inside its entry and is gone within FX_MAX_TAIL_MS at D 120, 200, 400, 732 and 1400", () => {
    const view = fullBoardView();
    const blobId = firstFieldId(view);
    for (const ms of DS) {
      expectInside(ENTRANCES.in33dh3Drop({ D: ms, tgt: testid.zone("you", "units", 3), intensity: 1, view, landing: unitLanding("p1", "in", IN33DH3) }), ms, "in33dh3Drop");
      expectInside(ENTRANCES.primeCrown({ D: ms, tgt: testid.zone("you", "units", 3), intensity: 1, view, landing: unitLanding("p1", "pr", PRIME) }), ms, "primeCrown");
      expectInside(ENTRANCES.blobPlop({ D: ms, tgt: testid.zone("you", "backrow", 1), intensity: 1, view, landing: fieldLanding("p1", "bl", BLOB) }), ms, "blobPlop");
      const [marker, packet] = planEach(dataPacketFixture(view, "in", "p1"), ms, [], view);
      expectInside(marker?.cues ?? [], ms, "dataPacket marker");
      expectInside(packet?.cues ?? [], ms, "dataPacket shuffledIn");
      const [cry] = planEach(primeConductFixture(view, "pr", "p1"), ms, [played("p1", "pr", PRIME), unitLanding("p1", "pr", PRIME)], view);
      expectInside(cry?.cues ?? [], ms, "primeConduct");
      const [drum, ripple] = planEach(blobDrumFixture(view, blobId, "p1"), ms, [], view);
      expectInside(drum?.cues ?? [], ms, "blobDrum marker");
      expectInside(ripple?.cues ?? [], ms, "blobDrum announce");
    }
  });

  it("R3407000 R202 from the other seat every beat still draws, names no card and no hand card", () => {
    const view = fullBoardView();
    expect(view.viewer).toBe("p1");
    const plays = [
      [IN33DH3, unitLanding("p2", "theirs", IN33DH3), dataPacketFixture(view, "theirs", "p2")],
      [PRIME, unitLanding("p2", "theirs", PRIME), primeConductFixture(view, "theirs", "p2")],
      [BLOB, fieldLanding("p2", "theirs", BLOB), blobDrumFixture(view, "theirs", "p2")],
    ] as const;
    for (const [id, landing, fixture] of plays) {
      const events = [played("p2", "theirs", id), landing, ...redacted(fixture), resolved("p2", "theirs", id)];
      const run = planWithOffsets(events, view);
      const keyed = run.filter(({ entry }) => entry.events.some((event) => fixture.some((own) => own.type === event.type)));
      expect(keyed.length, id).toBeGreaterThan(0);
      for (const { entry, cues } of keyed) expect(cues.length, `${id} ${entry.events.map((event) => event.type).join(",")}`).toBeGreaterThan(0);
      const json = JSON.stringify(cuesOfRun(run));
      expect(json, id).not.toMatch(CARD_ID);
      expect(json, id).not.toMatch(/hand-card-/);
    }
    // The Prime goes into the other seat's deck as a card back, though its ids are hidden.
    const packet = planEach([death("p2", "theirs"), shuffled("p2", true)], D, [], view)[1];
    expect(packet?.cues[1]).toEqual({ kind: "ghost", from: tid(animTestid.graveyard("opponent")), to: tid(animTestid.library("opponent")), delayMs: 40, durationMs: 220 });
  });
});
