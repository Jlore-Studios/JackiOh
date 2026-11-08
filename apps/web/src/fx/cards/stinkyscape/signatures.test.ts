// R3395000: Stinkyscape 001 to 090's 24 signatures (this folder), planned as FxLayer plans them: remember,
// then plan. Each is its table's signature entry with its own recipe or entrance; each gallery play draws,
// every cue inside its entry plus FX_MAX_TAIL_MS (R200) and naming no card; from the caster's opponent's seat
// no beat names a hidden card or flies more than a card back (R202); with the effects off nothing draws. The
// beats that read the board (Siva 2's drips to the Humans, Flak v5's first wave, Beam v5's first hit, the
// Prototype-X2's welds) keep to what the view shows. R3395001: a signature's own beat replaces the event's
// system moment too (Tylannt's dial in place of the shared quest moment), and draws on a sweep in place of
// the fog (Flak v5's waves over every enemy Unit).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../../../cards/marks.ts";
import { animTestid, entranceEntries, planEntries, type EntranceResolver } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { ENTRANCES } from "../../entrances.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryBoardOf, galleryCardEvents, galleryEntry, galleryFieldView, galleryPlayOf } from "../../motifs/gallery.ts";
import { hasSetPiece } from "../../setPieces.ts";
import { entranceCues } from "../../takeover.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE, isSignature } from "../tables.ts";

/** The 24 signatures of #3395's table, each its entry's frame and recipe. */
const SIGNATURES: Readonly<Record<string, { bespoke?: string; entrance?: string; takeover?: string; aura?: string }>> = {
  "stinkyscape-006": { bespoke: "vaultHeist", entrance: "vaultStride", takeover: "vault", aura: "arcane" },
  "stinkyscape-007": { bespoke: "heistLocks", entrance: "heistDrop", aura: "laserGrid" },
  "stinkyscape-016": { bespoke: "rodBands", entrance: "rodRise", takeover: "rod", aura: "silverRise" },
  "stinkyscape-018": { bespoke: "magiBoot" },
  "stinkyscape-026": { bespoke: "campusLaunch", entrance: "lecternRise", aura: "drift" },
  "stinkyscape-034": { bespoke: "droneConveyor" },
  "stinkyscape-035": { bespoke: "seedPod" },
  "stinkyscape-037": { bespoke: "allureRibbon", entrance: "spotlightKiss", aura: "flurry" },
  "stinkyscape-039": { bespoke: "narcorCases", entrance: "labVault", aura: "labHaze" },
  "stinkyscape-044": { bespoke: "paperclipLink", takeover: "wire" },
  "stinkyscape-058": { bespoke: "beamCharge", entrance: "emitterBloom", aura: "ember" },
  "stinkyscape-059": { bespoke: "repeaterBurst", entrance: "barrelSpin", aura: "ember" },
  "stinkyscape-060": { bespoke: "cannonShell", entrance: "breechLock", aura: "chalk" },
  "stinkyscape-061": { bespoke: "flakWaves", entrance: "turretScan", aura: "drift" },
  "stinkyscape-066": { bespoke: "asteroidCrunch", entrance: "intakeMaw", aura: "timber" },
  "stinkyscape-070": { bespoke: "ghostHulls", entrance: "wraithGather", aura: "void" },
  "stinkyscape-071": { bespoke: "echoRings", entrance: "heraldHorn", aura: "storm" },
  "stinkyscape-073": { entrance: "paragonStreak", aura: "frost" },
  "stinkyscape-074": { entrance: "paragonDive", takeover: "whiteDwarf", aura: "holy" },
  "stinkyscape-080": { bespoke: "bellyBay", entrance: "flagshipDescent", aura: "flurry" },
  "stinkyscape-085": { bespoke: "wingMount", entrance: "testLurch", aura: "arcane" },
  "stinkyscape-086": { bespoke: "spineWeld", entrance: "forgeBurst", takeover: "forge", aura: "ember" },
  "stinkyscape-087": { bespoke: "glowLeash", entrance: "redCarpet", aura: "arcane" },
  "stinkyscape-088": { bespoke: "venomHand", entrance: "venomStep", aura: "toxic" },
};

/** The group's two heaviest plays by burst particles, the phone frame spec's (fx-phone-frame.cy.tsx). */
const HEAVIEST = ["stinkyscape-061", "stinkyscape-086"] as const;

const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

function envOf(intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv {
  return { intensity, card: () => undefined, memory: createFxMemory() };
}

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
    case "shot":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

/** Every entry of `defId`'s gallery play over its gallery board, with its cues as the layer plans them (entrance, then the plan). */
function galleryPlan(defId: string, intensity: number = FX_INTENSITY_SCALE.normal): { D: number; cues: FxCue[] }[] {
  const entry = FX_CARD_TABLE.get(defId);
  const play = entry === undefined ? null : galleryPlayOf(defId, entry);
  if (play === null) throw new Error(`${defId} has no gallery play`);
  const board = galleryBoardOf(defId, fullBoardView());
  const events = galleryCardEvents(defId, play, board, 1);
  const env = envOf(intensity);
  return entranceEntries(planEntries(events, withEvents(galleryFieldView(defId, board), events), false, undefined, hasSetPiece), resolve, { seen: new Set() }).map((planned) => {
    env.memory.remember(planned.events);
    return { D: planned.durationMs, cues: [...entranceCues(planned, env, planned.durationMs), ...planFx(planned, planned.view, env)] };
  });
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 4 });
const tid = (value: string, at?: { x: number; y: number }) => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });

/** Plays `defId` as `player` and each of `events` under it, an entry each lasting D, through the dispatch (planCardFx). */
function run(defId: string, events: readonly GameEvent[], D: number, player: PlayerId = "p1", view: PlayerView = fullBoardView()): { cues: FxCue[]; row: string | undefined }[] {
  const env = envOf();
  env.memory.remember([played(player, "ex", defId)]);
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    const result = planCardFx(event, { entry, view, env, D });
    return { cues: result?.cues ?? [], row: result?.row };
  });
}

describe("R3395000 Stinkyscape 001 to 090's signatures", () => {
  it("R3395000 the 24 are signatures, each with its frame, recipe and aura", () => {
    for (const [defId, want] of Object.entries(SIGNATURES)) {
      const entry = FX_CARD_TABLE.get(defId);
      expect(isSignature(entry), defId).toBe(true);
      if (entry === undefined || entry.kind === "none") throw new Error(`${defId} has no entry`);
      expect(entry.kind === "bespoke" ? entry.key : undefined, defId).toBe(want.bespoke);
      expect(entry.entrance, defId).toBe(want.entrance);
      expect(entry.takeover, defId).toBe(want.takeover);
      const aura = entry.kind === "bespoke" || entry.kind === "hooks" ? entry.aura : undefined;
      expect(aura?.look, defId).toBe(want.aura);
      // A Unit's aura stands on its tile, a Field Spell's in its zone; a Mythic and none else takes a takeover.
      const def = CATALOG[defId];
      if (aura !== undefined) expect(aura.scope, defId).toBe(def?.type === "Unit" ? "tile" : "zone");
      expect(want.takeover !== undefined, defId).toBe((def?.printedRarity ?? def?.rarity) === "Mythic");
    }
    // Each entrance its own, each recipe its own.
    const entrances = Object.values(SIGNATURES).flatMap((want) => (want.entrance === undefined ? [] : [want.entrance]));
    expect(new Set(entrances).size).toBe(entrances.length);
    const recipes = Object.values(SIGNATURES).flatMap((want) => (want.bespoke === undefined ? [] : [want.bespoke]));
    expect(new Set(recipes).size).toBe(recipes.length);
  });

  it("R3395000 R200 every gallery play draws, each cue inside its entry and gone within FX_MAX_TAIL_MS, naming no card", () => {
    for (const defId of Object.keys(SIGNATURES)) {
      const entries = galleryPlan(defId);
      const cues = entries.flatMap((entry) => entry.cues);
      expect(cues.length, defId).toBeGreaterThan(0);
      for (const { D, cues: own } of entries) {
        for (const cue of own) {
          const where = `${defId} ${cue.kind}`;
          expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, where).toBeLessThanOrEqual(D);
          if (cue.kind === "pull" || cue.kind === "conceal") continue;
          expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
        }
      }
      expect(JSON.stringify(cues), defId).not.toContain(`"${defId}"`);
    }
  });

  it("R3395000 each entrance makes its one shake at the slam and draws without the board", () => {
    const D = 400;
    for (const [defId, want] of Object.entries(SIGNATURES)) {
      if (want.entrance === undefined) continue;
      const key = want.entrance as keyof typeof ENTRANCES;
      const cues = ENTRANCES[key]({ D, tgt: testid.zone("you", "units", 3), intensity: FX_INTENSITY_SCALE.normal });
      expect(cues.length, defId).toBeGreaterThan(3);
      expect(cues.filter((cue) => cue.kind === "shake"), defId).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA[key], delayMs: Math.round(FX_SLAM_AT * D) }]);
      // Its own figure, never the rarity's gold.
      expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "gold" && cue.spread === "area"), defId).toBe(false);
    }
  });

  it("R3395000 R202 from the caster's opponent's seat no beat names a hidden card, and every card flies as a back to the opponent's hand pile", () => {
    // The other seat plays: its cards added are hidden from this one, its prompts bare.
    const view = fullBoardView();
    const hidden: GameEvent = { type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" };
    const bare: GameEvent = { type: "promptAnswered", player: "p2" };
    const foe = view.you.units[0]?.instanceId ?? "";
    const plays: readonly [string, GameEvent[]][] = [
      // Tylannt: a quest of theirs completed, its reward hidden.
      ["stinkyscape-006", [{ type: "questCompleted", player: "p2", instanceId: view.opponent.backrow[0] === null ? "q" : "q", quest: "Initiation" }, hidden]],
      // The Greatest Heist's Discover, then its quest's reward.
      ["stinkyscape-007", [bare, hidden, { type: "hookResolving", sourceId: "ex", defId: "stinkyscape-007", controller: "p2", hook: "whenever" }, hidden]],
      // The Narcor Project's End of turn Narcor and its Activate's discard.
      ["stinkyscape-039", [{ type: "hookResolving", sourceId: "ex", defId: "stinkyscape-039", controller: "p2", hook: "endOfTurn" }, hidden, { type: "hookResolving", sourceId: "ex", defId: "stinkyscape-039", controller: "p2", hook: "activate" }, { type: "discarded", instanceId: "hidden", defId: "hidden", owner: "p2" }]],
      // Great Paperclip Theory's Drone Core I.
      ["stinkyscape-044", [hidden, { type: "modifierChanged", player: "p2", modifierId: "m", added: true }]],
      // Miner v5's Resource.
      ["stinkyscape-066", [{ type: "hookResolving", sourceId: "ex", defId: "stinkyscape-066", controller: "p2", hook: "activate" }, hidden]],
      // Galaxy's and the Prototypes' Discovered picks, fused in.
      ["stinkyscape-080", [{ type: "promptOpened", player: "p2" }, bare, { type: "fused", instanceIds: ["ex", "hidden"], resultInstanceId: "ex", defId: "stinkyscape-080" }]],
      ["stinkyscape-085", [bare, { type: "fused", instanceIds: ["ex", "hidden"], resultInstanceId: "ex", defId: "stinkyscape-085" }]],
      ["stinkyscape-086", [bare, { type: "fused", instanceIds: ["ex", "hidden"], resultInstanceId: "ex", defId: "stinkyscape-086" }]],
      // Siva 2's Radiant Cry's hand.
      ["stinkyscape-088", [hidden, hidden, hidden]],
      // Kwzka's and Siva 1's Allure on this seat's Unit.
      ["stinkyscape-037", [{ type: "hookResolving", sourceId: "ex", defId: "stinkyscape-037", controller: "p2", hook: "whenever" }, { type: "marked", instanceId: foe, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }]],
      ["stinkyscape-087", [{ type: "marked", instanceId: foe, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }]],
    ];
    for (const [defId, events] of plays) {
      const cues = run(defId, events, 400, "p2", view).flatMap((entry) => entry.cues);
      expect(cues.length, defId).toBeGreaterThan(0);
      const text = JSON.stringify(cues);
      expect(text, defId).not.toMatch(/hidden|core-\d{3}|stinkyscape-\d|hand-card-/);
      // No owner-only glint on the watching seat.
      expect(cues.some((cue) => cue.kind === "sheen" && JSON.stringify(cue.at) === JSON.stringify(tid(animTestid.hand("opponent")))), defId).toBe(false);
      for (const cue of cues) if (cue.kind === "ghost" && cue.to.kind === "testid" && cue.to.testid.startsWith("hand-")) expect(cue.to, defId).toEqual(tid(animTestid.hand("opponent")));
    }
    // A hidden play keys none of the 24.
    for (const defId of Object.keys(SIGNATURES)) {
      const env = envOf();
      env.memory.remember([played("p2", "hidden", "hidden")]);
      const event: GameEvent = { type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" };
      expect(planCardFx(event, { entry: galleryEntry([event], view), view, env, D: 250 }), defId).toBeNull();
    }
  });

  it("R3395000 Tylannt's dial slams home over the Quest's zone, and R3395001 it replaces the shared quest moment", () => {
    const view = fullBoardView();
    const quest = view.you.backrow[0];
    const questId = quest !== null && quest !== undefined && !quest.faceDown ? quest.instanceId : "q";
    const completed: GameEvent = { type: "questCompleted", player: "p1", instanceId: questId, quest: "Initiation" };
    const [dial] = run("stinkyscape-006", [completed], 500);
    expect(dial?.row).toBe("replace");
    expect(dial?.cues[0]).toMatchObject({ kind: "figure", figure: "tumbler", at: tid(testid.card(questId)), durationMs: 500 });
    // Through the whole plan: no shared banner, only the signature's own beat.
    const plan = (defId: string): FxCue[] => {
      const env = envOf();
      env.memory.remember([played("p1", "ex", defId), completed]);
      return planFx({ ...galleryEntry([completed], view), durationMs: 500 }, view, env);
    };
    expect(plan("stinkyscape-006").some((cue) => cue.kind === "banner")).toBe(false);
    expect(plan("stinkyscape-006").some((cue) => cue.kind === "figure")).toBe(true);
    // A card with no signature keeps the shared quest moment.
    expect(plan("core-002").some((cue) => cue.kind === "banner")).toBe(true);
  });

  it("R3395001 Flak v5's waves, each a sweep of every enemy Unit, draw its own beat in place of the fog", () => {
    const view = fullBoardView();
    const foes = view.opponent.units.flatMap((unit) => (unit === null ? [] : [unit.instanceId]));
    const wave = (): GameEvent[] => foes.map((targetId): GameEvent => ({ type: "damage", sourceId: "ex", targetId, amount: 1, combat: false }));
    const plan = (defId: string): { sweep: boolean; cues: FxCue[] }[] => {
      // Its Activate as the engine sends it: the marker, then two waves over every enemy Unit.
      const events: GameEvent[] = [{ type: "hookResolving", sourceId: "ex", defId, controller: "p1", hook: "activate" }, ...wave(), ...wave()];
      const env = envOf();
      return planEntries(events, withEvents(view, events), false, undefined, hasSetPiece).map((entry) => {
        env.memory.remember(entry.events);
        return { sweep: entry.sweep !== undefined, cues: planFx(entry, entry.view, env) };
      });
    };
    const sweeps = plan("stinkyscape-061").filter((entry) => entry.sweep);
    expect(sweeps).toHaveLength(2);
    const bursts = (cues: FxCue[], preset: string): number => cues.filter((cue) => cue.kind === "burst" && cue.preset === preset).length;
    // The first wave blooms shrapnel over each enemy Unit, the second puffs smoke; neither rolls the fog.
    expect(bursts(sweeps[0]?.cues ?? [], "spark")).toBe(foes.length);
    expect(bursts(sweeps[1]?.cues ?? [], "smoke")).toBe(foes.length);
    for (const { cues } of sweeps) expect(cues.some((cue) => cue.kind === "fog")).toBe(false);
    // A card with no signature keeps the fog.
    expect(plan("core-002").filter((entry) => entry.sweep).every((entry) => entry.cues.some((cue) => cue.kind === "fog"))).toBe(true);
  });

  it("R3395000 the board-reading beats: Siva 2's drips, Flak v5's first wave, Beam v5's first hit, the Prototype-X2's welds, A'Zoth's echo", () => {
    const view = fullBoardView();
    // Siva 2: a drip to each friendly face-up Human but her, landing at the slam.
    const humans = { ...view, you: { ...view.you, units: view.you.units.map((unit, k) => (unit === null || k > 1 ? unit : { ...unit, defId: "core-002" })) } };
    const landing: Extract<GameEvent, { type: "summoned" }> = { type: "summoned", player: "p1", instanceId: "siva", defId: "stinkyscape-088", row: "units", lane: 3 };
    const drips = ENTRANCES.venomStep({ D: 400, tgt: testid.zone("you", "units", 3), intensity: 1, view: humans, landing }).filter((cue) => cue.kind === "shot");
    expect(drips.map((cue) => (cue.kind === "shot" && cue.to.kind === "testid" ? cue.to.testid : ""))).toEqual([testid.zone("you", "units", 1), testid.zone("you", "units", 2)]);
    for (const drip of drips) if (drip.kind === "shot") expect(drip.delayMs + drip.flightMs).toBe(Math.round(FX_SLAM_AT * 400));
    // Flak v5: the first wave's hits bloom shrapnel; a hit past one per enemy Unit is a later wave's puff.
    const marker: GameEvent = { type: "hookResolving", sourceId: "ex", defId: "stinkyscape-061", controller: "p1", hook: "activate" };
    const hits = view.opponent.units.flatMap((unit) => (unit === null ? [] : [unit.instanceId]));
    const waves = run("stinkyscape-061", [marker, ...[...hits, hits[0] ?? ""].map((targetId): GameEvent => ({ type: "damage", sourceId: "ex", targetId, amount: 1, combat: false }))], 300);
    expect(waves.slice(1, -1).every((wave) => wave.cues[0]?.kind === "burst" && wave.cues[0].preset === "spark")).toBe(true);
    expect(waves.at(-1)?.cues[0]).toMatchObject({ kind: "burst", preset: "smoke" });
    // Beam v5: its first hit charges and beams, the next ones flare in a ring alone.
    const beam = run("stinkyscape-058", [{ ...marker, defId: "stinkyscape-058" }, ...hits.slice(0, 2).map((targetId): GameEvent => ({ type: "damage", sourceId: "ex", targetId, amount: 11, combat: false }))], 300);
    expect(beam[1]?.cues.map((cue) => cue.kind)).toEqual(["sheen", "shot", "brand"]);
    expect(beam[2]?.cues.map((cue) => cue.kind)).toEqual(["ring"]);
    // Prototype-X2: a weld at each fusion, the first with sparks and a shake, a third's shorter, alone.
    const fused = (k: number): GameEvent => ({ type: "fused", instanceIds: ["ex", `w${String(k)}`], resultInstanceId: "ex", defId: "stinkyscape-086" });
    const welds = run("stinkyscape-086", [fused(1), fused(2), fused(3)], 500);
    expect(welds.map((weld) => weld.cues.map((cue) => cue.kind))).toEqual([["weld", "ring", "burst", "shake"], ["weld", "ring"], ["weld"]]);
    expect(welds[2]?.cues[0]).toMatchObject({ durationMs: 200 });
    // A'Zoth: a replayed Cry pulses the echo over its Unit and over its target, over their rows.
    const called = view.you.units[1]?.instanceId ?? "";
    const echo = run("stinkyscape-071", [{ type: "hookResolving", sourceId: called, defId: "core-011", controller: "p1", hook: "cry" }, { type: "damage", sourceId: called, targetId: hits[0] ?? "", amount: 1, combat: false }], 300);
    expect(echo.map((entry) => [entry.row, entry.cues.map((cue) => cue.kind)])).toEqual([
      ["keep", ["ring"]],
      ["keep", ["ring"]],
    ]);
  });

  it("R3395000 HEAVIEST are the group's two plays throwing the most burst particles", () => {
    const weight = (defId: string): number =>
      galleryPlan(defId)
        .flatMap((entry) => entry.cues)
        .reduce((sum, cue) => sum + (cue.kind === "burst" ? cue.count : 0), 0);
    const ranked = Object.keys(SIGNATURES).sort((a, b) => weight(b) - weight(a) || a.localeCompare(b));
    expect(ranked.slice(0, 2)).toEqual([...HEAVIEST]);
  });

  it("R3395000 R200 with the effects off nothing draws", () => {
    for (const defId of Object.keys(SIGNATURES)) expect(galleryPlan(defId, FX_INTENSITY_SCALE.off).flatMap((entry) => entry.cues), defId).toEqual([]);
  });
});
