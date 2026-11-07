// R3336000: the Drone Battleship's sounds. Every row of `battleshipCues` names one SfxId for one beat of the
// great ship, each read from the event and the viewer's `PlayerView` alone, so a card the view cannot show
// (R97's sentinel, or an id it cannot place) sounds nothing (R203).

import type { GameEvent, PlayerView, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { SHIP_DRONE_IDS, SHIP_LAYER_IDS } from "../cards/art/stinkyscape.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { battleshipCues, REACTOR_PULSE_MAX, REACTOR_PULSE_MIN, reactorPulseAmount } from "./battleshipCues.ts";
import { HIDDEN_DEF_ID, LANE_PAN_MAX } from "./constants.ts";
import { cuesFor, type CueContext } from "./cues.ts";
import { SFX } from "./sfx.ts";
import { FakeAudio } from "./test/fakeAudio.ts";
import type { SfxId, SfxParams, SoundCue } from "./types.ts";

const LAYER_LANE = 0;
const NO_LINES: CueContext["lines"] = { voices: {}, effects: {}, cards: {}, emotes: {} };

/** The unit on top of lane 0 of the opponent's field: the Battleship shows one layer at a time (R1842000). */
function layer(which: keyof typeof SHIP_LAYER_IDS, over: Partial<UnitView> = {}): UnitView {
  return unit("p2", { instanceId: `ship-${which}`, defId: SHIP_LAYER_IDS[which], maxHealth: 10, health: 10, buried: 0, ...over });
}

function boardOf(top: UnitView): PlayerView {
  return baseView({ opponent: emptySide("p2", { hand: { count: 4 }, units: [top, null, null, null, null] }) });
}

function ctxOf(view: PlayerView): CueContext {
  return { view, lines: NO_LINES, manaBefore: () => 0 };
}

const sfxOf = (cues: readonly SoundCue[]): { id: SfxId; params: SfxParams | undefined }[] =>
  cues.map((cue) => {
    if (cue.kind !== "sfx") throw new Error(`expected only sfx cues, got ${cue.kind}`);
    return { id: cue.id, params: cue.params };
  });
const ids = (cues: readonly SoundCue[]): SfxId[] => sfxOf(cues).map((c) => c.id);
const soundOf = (event: GameEvent, view: PlayerView): SfxId[] => ids(battleshipCues(event, ctxOf(view)));

const summoned = (defId: string, row: "units" | "backrow" = "units"): GameEvent => ({
  type: "summoned",
  player: "p2",
  instanceId: "arrived",
  defId,
  row,
  lane: LAYER_LANE,
});
const damage = (targetId: string, amount: number, over: { sourceId?: string | null; absorbed?: number } = {}): GameEvent => ({
  type: "damage",
  sourceId: over.sourceId === undefined ? "hero-p1" : over.sourceId,
  targetId,
  amount,
  combat: false,
  ...(over.absorbed === undefined ? {} : { absorbed: over.absorbed }),
});
const redirected = (toId: string): GameEvent => ({ type: "redirected", what: "damage", fromId: "hero-p2", toId, byInstanceId: null });
const activated = (instanceId: string, defId: string): GameEvent => ({ type: "activated", player: "p2", instanceId, defId, ability: "activate" });
const attackDeclared = (attackerId: string): GameEvent => ({ type: "attackDeclared", attackerId, targetId: "hero-p1", forced: false });
const destroyed = (instanceId: string, defId: string): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId,
  owner: "p2",
  controller: "p2",
  attack: 3,
  maxHealth: 10,
  killerId: null,
});

describe("R3336000 the Drone Battleship's sounds", () => {
  it("R3336000 each layer's arrival has its own sound: a horn, a lock and an armor ring", () => {
    const view = baseView();
    expect(soundOf(summoned(SHIP_LAYER_IDS.meltdown), view)).toEqual(["horn"]);
    expect(soundOf(summoned(SHIP_LAYER_IDS.propagation), view)).toEqual(["lock"]);
    expect(soundOf(summoned(SHIP_LAYER_IDS.shield), view)).toEqual(["armorRing"]);
  });

  it("R3336000 a drone launched from the bay whooshes (Interceptor) or lands heavy (Corvette)", () => {
    const view = baseView();
    expect(sfxOf(battleshipCues(summoned(SHIP_DRONE_IDS.interceptor), ctxOf(view)))).toEqual([{ id: "whoosh", params: undefined }]);
    expect(sfxOf(battleshipCues(summoned(SHIP_DRONE_IDS.corvette), ctxOf(view)))).toEqual([{ id: "summon", params: { slamTier: "large" } }]);
  });

  it("R3336000 a hit redirected onto the Shield sounds a dome flare, onto anything else nothing", () => {
    const shield = layer("shield", { buried: 2 });
    expect(soundOf(redirected(shield.instanceId), boardOf(shield))).toEqual(["domeFlare"]);
    const propagation = layer("propagation");
    expect(soundOf(redirected(propagation.instanceId), boardOf(propagation))).toEqual([]);
    expect(soundOf(redirected("hero-p2"), boardOf(shield))).toEqual([]);
  });

  it("R3336000 an ordinary hit on the Shield clanks, once, whether or not Armor took half of it", () => {
    const shield = layer("shield", { buried: 2, armor: 3 });
    const view = boardOf(shield);
    expect(soundOf(damage(shield.instanceId, 4), view)).toEqual(["armorClank"]);
    // The damage row's own R1363 clank already speaks when Armor took half or more: no second one.
    const heavy = damage(shield.instanceId, 1, { absorbed: 3 });
    expect(soundOf(heavy, view)).toEqual([]);
    expect(ids(cuesFor(heavy, ctxOf(view))).filter((id) => id === "armorClank")).toHaveLength(1);
    expect(ids(cuesFor(damage(shield.instanceId, 4), ctxOf(view))).filter((id) => id === "armorClank")).toHaveLength(1);
    // A hit with nothing left of it makes no sound here, as the damage row makes none.
    expect(soundOf(damage(shield.instanceId, 0, { absorbed: 3 }), view)).toEqual([]);
  });

  it("R3336000 the Propagation layer's activate opens the bay door, another card's activate does not", () => {
    const propagation = layer("propagation");
    expect(soundOf(activated(propagation.instanceId, SHIP_LAYER_IDS.propagation), boardOf(propagation))).toEqual(["bayDoor"]);
    expect(soundOf(activated("u9", "core-004"), boardOf(propagation))).toEqual([]);
    expect(soundOf(activated(layer("shield").instanceId, SHIP_LAYER_IDS.shield), baseView())).toEqual([]);
  });

  it("R3336000 the Meltdown layer's own tick is a reactor pulse that grows with the damage it takes", () => {
    const melt = layer("meltdown");
    const view = boardOf(melt);
    const amounts = [1, 3, 6, 10].map((hit) => {
      const [cue] = sfxOf(battleshipCues(damage(melt.instanceId, hit, { sourceId: melt.instanceId }), ctxOf(view)));
      expect(cue?.id).toBe("reactorPulse");
      return cue?.params?.amount ?? 0;
    });
    expect(amounts).toEqual([2, 4, 6, REACTOR_PULSE_MAX]);
    // The same tick on a core that has already lost health is harder, and the pulse never leaves its range.
    const worn = boardOf(layer("meltdown", { health: 4 }));
    const wornTick = sfxOf(battleshipCues(damage(melt.instanceId, 1, { sourceId: melt.instanceId }), ctxOf(worn)))[0]?.params?.amount ?? 0;
    expect(wornTick).toBeGreaterThan(amounts[0] ?? Infinity);
    expect(reactorPulseAmount(melt, 1_000)).toBe(REACTOR_PULSE_MAX);
    expect(reactorPulseAmount({ ...melt, health: melt.maxHealth }, 0)).toBe(REACTOR_PULSE_MIN);
    expect(reactorPulseAmount({ ...melt, maxHealth: 0 }, 1)).toBe(REACTOR_PULSE_MAX);
  });

  it("R3336000 a hit on the Meltdown layer from anything else, or one that does no damage, is no pulse", () => {
    const melt = layer("meltdown");
    const view = boardOf(melt);
    expect(soundOf(damage(melt.instanceId, 3, { sourceId: "u9" }), view)).toEqual([]);
    expect(soundOf(damage(melt.instanceId, 3, { sourceId: null }), view)).toEqual([]);
    expect(soundOf(damage(melt.instanceId, 0, { sourceId: melt.instanceId }), view)).toEqual([]);
  });

  it("R3336000 the Meltdown layer's attack burns", () => {
    const melt = layer("meltdown");
    expect(soundOf(attackDeclared(melt.instanceId), boardOf(melt))).toEqual(["burn"]);
    const shield = layer("shield");
    expect(soundOf(attackDeclared(shield.instanceId), boardOf(shield))).toEqual([]);
  });

  it("R3336000 the Meltdown layer's destruction is the reactor's blast, the hardest pulse", () => {
    const melt = layer("meltdown");
    const cues = sfxOf(battleshipCues(destroyed(melt.instanceId, SHIP_LAYER_IDS.meltdown), ctxOf(boardOf(melt))));
    expect(cues).toEqual([{ id: "reactorPulse", params: { amount: REACTOR_PULSE_MAX } }]);
  });

  it("R3336000 a Shield or Propagation layer that breaks over decks shears them, one over nothing does not", () => {
    for (const which of ["shield", "propagation"] as const) {
      const over = layer(which, { buried: 1 });
      expect(soundOf(destroyed(over.instanceId, SHIP_LAYER_IDS[which]), boardOf(over)), which).toEqual(["deckShear"]);
      const bare = layer(which, { buried: 0 });
      expect(soundOf(destroyed(bare.instanceId, SHIP_LAYER_IDS[which]), boardOf(bare)), which).toEqual([]);
    }
  });

  it("R3336000 a card the view cannot show sounds nothing: the sentinel, or an id it cannot place", () => {
    const shown = layer("shield", { buried: 2 });
    const goneShield = layer("shield", { instanceId: "gone-shield", buried: 2 });
    const melt = layer("meltdown");
    const propagation = layer("propagation", { buried: 1 });
    const view = boardOf(shown);
    const hidden: GameEvent[] = [
      summoned(HIDDEN_DEF_ID),
      summoned(HIDDEN_DEF_ID, "backrow"),
      summoned(SHIP_LAYER_IDS.meltdown, "backrow"),
      redirected(HIDDEN_DEF_ID),
      damage(HIDDEN_DEF_ID, 5),
      damage(HIDDEN_DEF_ID, 5, { sourceId: HIDDEN_DEF_ID }),
      activated(HIDDEN_DEF_ID, HIDDEN_DEF_ID),
      attackDeclared(HIDDEN_DEF_ID),
      destroyed(HIDDEN_DEF_ID, HIDDEN_DEF_ID),
      // A layer the view shows nowhere (it is under another card or not yet arrived) is not placed either.
      redirected(goneShield.instanceId),
      damage(goneShield.instanceId, 5),
      damage(melt.instanceId, 5, { sourceId: melt.instanceId }),
      attackDeclared(melt.instanceId),
      destroyed(melt.instanceId, SHIP_LAYER_IDS.meltdown),
      destroyed(propagation.instanceId, SHIP_LAYER_IDS.propagation),
    ];
    for (const event of hidden) expect(battleshipCues(event, ctxOf(view)), JSON.stringify(event)).toEqual([]);
    // Even a unit whose defId is the sentinel on the field is no layer, whatever its instance.
    const masked = unit("p2", { instanceId: "masked", defId: HIDDEN_DEF_ID, buried: 2 });
    expect(soundOf(destroyed("masked", HIDDEN_DEF_ID), boardOf(masked))).toEqual([]);
    expect(soundOf(redirected("masked"), boardOf(masked))).toEqual([]);
  });

  it("R3336000 an event about no layer adds nothing, and no cue names a card", () => {
    const plain = unit("p2", { instanceId: "plain", defId: "core-004", buried: 2 });
    const view = boardOf(plain);
    const events: GameEvent[] = [
      summoned("core-004"),
      redirected(plain.instanceId),
      damage(plain.instanceId, 4, { sourceId: plain.instanceId }),
      activated(plain.instanceId, "core-004"),
      attackDeclared(plain.instanceId),
      destroyed(plain.instanceId, "core-004"),
    ];
    for (const event of events) expect(battleshipCues(event, ctxOf(view)), event.type).toEqual([]);
    const shield = layer("shield", { buried: 2 });
    const named = [
      summoned(SHIP_LAYER_IDS.meltdown),
      summoned(SHIP_DRONE_IDS.corvette),
      redirected(shield.instanceId),
      damage(shield.instanceId, 4),
      destroyed(shield.instanceId, SHIP_LAYER_IDS.shield),
    ].flatMap((event) => battleshipCues(event, ctxOf(boardOf(shield))));
    expect(named.length).toBeGreaterThan(0);
    expect(named.every((cue) => cue.kind === "sfx"), "only sounds, no voice or card effect").toBe(true);
    expect(JSON.stringify(named)).not.toContain("stinkyscape");
  });

  it("R3336000 cuesFor adds the ship's sounds after the row's own, and never pans them to a lane", () => {
    const shield = layer("shield", { buried: 2 });
    // Lane 0 is panned hard left; the ship is the board's, so only the row's own death is.
    const cues = sfxOf(cuesFor(destroyed(shield.instanceId, SHIP_LAYER_IDS.shield), ctxOf(boardOf(shield))));
    expect(cues.map((c) => c.id)).toEqual(["death", "deckShear"]);
    expect(cues[0]?.params?.pan).toBe(-LANE_PAN_MAX);
    expect(cues[1]?.params).toBeUndefined();
    const arrival = sfxOf(cuesFor(summoned(SHIP_LAYER_IDS.meltdown), ctxOf(baseView())));
    expect(arrival.map((c) => c.id)).toEqual(["summon", "horn"]);
  });
});

describe("R3336000 the Drone Battleship's recipes", () => {
  const AT = 1.5;

  function render(id: SfxId, params: SfxParams): { seconds: number; hz: number; peaks: number } {
    const audio = new FakeAudio({ state: "running", currentTime: 1 });
    const out = audio.context.createGain();
    const first = audio.nodes.length;
    const seconds = SFX[id].recipe(audio.context, out, AT, params);
    const made = audio.nodes.slice(first);
    const thump = made.find((node) => node.kind === "oscillator");
    const hz = thump?.param("frequency").events.find((e) => e.method === "setValueAtTime");
    let peaks = 0;
    for (const node of made.filter((n) => n.kind === "gain")) {
      for (const event of node.param("gain").events) if (event.method === "linearRampToValueAtTime") peaks += event.value;
    }
    return { seconds, hz: hz?.method === "setValueAtTime" ? hz.value : 0, peaks };
  }

  it("R3336000 the reactor pulse is longer, higher and louder as its amount climbs from 1 to 10", () => {
    const pulses = [REACTOR_PULSE_MIN, 4, 7, REACTOR_PULSE_MAX].map((amount) => render("reactorPulse", { amount }));
    for (let i = 1; i < pulses.length; i += 1) {
      expect(pulses[i]?.seconds ?? 0, `amount step ${String(i)} is longer`).toBeGreaterThan(pulses[i - 1]?.seconds ?? Infinity);
      expect(pulses[i]?.hz ?? 0, `amount step ${String(i)} is higher`).toBeGreaterThan(pulses[i - 1]?.hz ?? Infinity);
      expect(pulses[i]?.peaks ?? 0, `amount step ${String(i)} is louder`).toBeGreaterThan(pulses[i - 1]?.peaks ?? Infinity);
    }
    expect(render("reactorPulse", {}).seconds, "no amount is the softest").toBe(pulses[0]?.seconds);
  });

  it("R3336000 the horn is about nine tenths of a second, the longest of the ship's sounds", () => {
    const horn = render("horn", {});
    expect(horn.seconds).toBeCloseTo(0.9, 5);
    for (const id of ["domeFlare", "bayDoor", "deckShear", "reactorPulse"] as const) {
      expect(render(id, { amount: REACTOR_PULSE_MAX }).seconds, id).toBeLessThan(horn.seconds);
    }
  });
});
