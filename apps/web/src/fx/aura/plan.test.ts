// R2787000: what wears an aura (a face-up Field Spell or Field Trap, a modifier a readable play added),
// what it covers, how auras stack, how each begins and ends, and the motes' budget. R3337003: an
// Anomaly's backdrop over its caster's side. R3400000: a face-up Unit's `tile` aura over its own unit zone.

import type { BackrowView, GameEvent, PlayerId, PlayerView, Row } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { laneZoneTestids } from "../../game/actions.ts";
import { testid, type Side } from "../../game/contract.ts";
import { baseView, emptySide, faceDownBackrow, faceUpBackrow, resetIds, unit } from "../../test/fixtures.ts";
import { parseFxCardTable } from "../cards/tables.ts";
import {
  AURA_FADE_IN_MS,
  AURA_FADE_OUT_MS,
  AURA_MAX_PER_ZONE,
  AURA_MOTES_PER_AURA,
  AURA_PARTICLE_SHARE,
  AURA_SOURCES_LIMIT,
  FX_INTENSITY_SCALE,
  FX_PARTICLE_CAP,
  FX_PARTICLE_CAP_MOBILE,
} from "../constants.ts";
import type { AnomalyCode } from "../../cards/anomalies.ts";
import { FLAVOR_LOOKS, FLAVOR_SUB_LOOKS, paletteLook } from "../flavors.ts";
import { createFxMemory } from "../memory.ts";
import { AURA_LOOK_NAMES, AURA_LOOKS, AURA_SCOPES, AURA_SHARED_LOOK_NAMES, type AuraLook, type AuraScope } from "./looks.ts";
import {
  attributeModifiers,
  auraBudget,
  auraTestid,
  auraZones,
  NO_AURAS,
  noteAnomalies,
  planAuras,
  reconcileAuras,
  settleAuras,
  sourceKey,
  stackAuras,
  type AuraPhase,
  type AuraSources,
  type AuraSpec,
  type TrackedAura,
} from "./plan.ts";

/**
 * Every look and every scope once: seven Field Spells, a Field Trap (core-018), a Spell (core-035) and, R3400000,
 * three Units on their tiles, one of them a bespoke entry's.
 */
const TABLE = parseFxCardTable(
  "aura.json5",
  `{
    "core-014": { aura: { look: "storm", scope: "side" } },
    "core-046": { aura: { look: "void", scope: "board" } },
    "classic-042": { aura: { look: "toxic", scope: "zone" } },
    "classic-075": { aura: { look: "holy", palette: "Fire", scope: "lane" } },
    "meditative-040": { aura: { look: "bloom", scope: "board" } },
    "meditativeplus-009": { aura: { look: "arcane", scope: "side" } },
    "stinkyscape-155": { aura: { look: "drift", scope: "side" } },
    "core-018": { aura: { look: "frost", scope: "zone" } },
    "core-035": { aura: { look: "ember", scope: "side" } },
    "meditativeplus-024": { bespoke: "bobSoldier", aura: { look: "flurry", palette: "Physical/wind", scope: "tile" } },
    "traininggrounds-038": { aura: { look: "chalk", scope: "tile" } },
    "traininggrounds-039-1": { aura: { look: "timber", scope: "tile" } },
  }`,
);

type Slot = BackrowView | null;

function field(owner: PlayerId, defId: string, over: Partial<Extract<BackrowView, { faceDown: false }>> = {}): BackrowView {
  return faceUpBackrow(owner, { defId, type: defId === "core-018" ? "Field Trap" : "Field Spell", ...over });
}

/** Five backrow slots a side, p1 viewing unless `viewer` says otherwise. */
function board(you: readonly Slot[], opponent: readonly Slot[], viewer: PlayerId = "p1", over: Partial<PlayerView> = {}): PlayerView {
  const other: PlayerId = viewer === "p1" ? "p2" : "p1";
  const pad = (slots: readonly Slot[]): Slot[] => [...slots, null, null, null, null, null].slice(0, 5);
  return baseView({
    viewer,
    active: viewer,
    you: emptySide(viewer, { backrow: pad(you) }),
    opponent: emptySide(other, { hand: { count: 4 }, backrow: pad(opponent) }),
    ...over,
  });
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 });
const modifier = (player: PlayerId, modifierId: string, added = true): GameEvent => ({ type: "modifierChanged", player, modifierId, added });
const trapFired = (controller: PlayerId, row: Row, lane: number): GameEvent => ({ type: "trapFired", instanceId: "t9", defId: "core-084", controller, row, lane });

function spec(key: string, scope: AuraScope, side: Side = "you", lane: number | null = 1, look: AuraLook = "storm"): AuraSpec {
  return { key, defId: "core-014", instanceId: key, side, lane, scope, look, colors: FLAVOR_LOOKS.Tech.colors };
}

function tracked(key: string, scope: AuraScope, seen: number, phase: AuraPhase = "standing", side: Side = "you", lane: number | null = 1): TrackedAura {
  return { spec: spec(key, scope, side, lane), phase, since: 0, seen };
}

const keys = (auras: readonly { spec: AuraSpec }[]): string[] => auras.map((aura) => aura.spec.key);

describe("R2787000 what wears an aura", () => {
  it("R2787000 R3400000 every face-up Field Spell, Field Trap and tile-aura Unit with an aura wears it, in all twelve looks and all five scopes, in its own or its entry's palette", () => {
    resetIds();
    const you = ["core-014", "core-046", "classic-042", "classic-075", "meditative-040"].map((defId) => field("p1", defId));
    const opponent = [field("p2", "meditativeplus-009"), field("p2", "core-018"), field("p2", "core-020"), field("p2", "stinkyscape-155")];
    const memory = createFxMemory();
    const sources: AuraSources = new Map();
    attributeModifiers([played("p1", "c9", "core-035"), modifier("p1", "m5"), resolved("p1", "c9", "core-035")], memory, sources);
    const view = board(you, opponent);
    view.you.modifiers = [{ id: "m5", label: "Your Spells cost (1) less" }];
    view.you.units = [unit("p1", { defId: "meditativeplus-024" }), unit("p1", { defId: "traininggrounds-038" }), null, null, null];
    view.opponent.units = [null, null, unit("p2", { defId: "traininggrounds-039-1" }), null, null];
    const planned = planAuras(view, sources, TABLE);
    expect(planned.map((aura) => [aura.defId, aura.side, aura.lane, aura.scope, aura.look])).toEqual([
      ["core-014", "you", 1, "side", "storm"],
      ["core-046", "you", 2, "board", "void"],
      ["classic-042", "you", 3, "zone", "toxic"],
      ["classic-075", "you", 4, "lane", "holy"],
      ["meditative-040", "you", 5, "board", "bloom"],
      ["meditativeplus-024", "you", 1, "tile", "flurry"],
      ["traininggrounds-038", "you", 2, "tile", "chalk"],
      ["core-035", "you", null, "side", "ember"],
      ["meditativeplus-009", "opponent", 1, "side", "arcane"],
      ["core-018", "opponent", 2, "zone", "frost"],
      ["stinkyscape-155", "opponent", 4, "side", "drift"],
      ["traininggrounds-039-1", "opponent", 3, "tile", "timber"],
    ]);
    // R3407000: the signature looks each stand on their own card's zone or tile (fx/cards/stinkyscape/signatures.test.ts).
    expect(new Set(planned.map((aura) => aura.look))).toEqual(new Set(AURA_SHARED_LOOK_NAMES));
    expect(new Set(planned.map((aura) => aura.scope))).toEqual(new Set(AURA_SCOPES));
    expect(planned[0]?.key).toBe(`field-${you[0] && !you[0].faceDown ? you[0].instanceId : ""}`);
    expect(planned[7]?.key).toBe("mod-p1-m5");
    expect(planned[5]?.key).toBe(`unit-${view.you.units[0]?.instanceId ?? ""}`);
    expect(planned[0]?.colors).toEqual(FLAVOR_LOOKS.Tech.colors);
    expect(planned[2]?.colors).toEqual(FLAVOR_LOOKS.Toxic.colors);
    // classic-075 is Holy; its entry names Fire.
    expect(planned[3]?.colors).toEqual(FLAVOR_LOOKS.Fire.colors);
    expect(planned[7]?.colors).toEqual(FLAVOR_LOOKS.Fire.colors);
    // R3400000: Bob's bespoke entry names Physical's wind.
    expect(planned[5]?.colors).toEqual(FLAVOR_SUB_LOOKS.Physical?.wind?.colors);
  });

  it("R2787000 an entry's palette may name a sub-look (#2792), whose colors the aura wears", () => {
    resetIds();
    const table = parseFxCardTable("aura.json5", `{ "core-014": { aura: { look: "storm", palette: "Tech/rocket", scope: "side" } } }`);
    const planned = planAuras(board([field("p1", "core-014")], []), new Map(), table);
    const rocket = FLAVOR_SUB_LOOKS.Tech?.rocket?.colors;
    expect(rocket).toBeDefined();
    expect(planned.map((aura) => aura.colors)).toEqual([rocket]);
  });

  it("R2787000 a zone aura covers its zone, a lane aura the lane's four zones, a side aura its ten and a board aura all twenty; a modifier's zone aura is its side's", () => {
    const zone = auraZones({ scope: "zone", side: "opponent", lane: 3 });
    expect(zone).toEqual([testid.zone("opponent", "backrow", 3)]);
    expect(auraZones({ scope: "lane", side: "you", lane: 2 })).toEqual(laneZoneTestids(2));
    expect(auraZones({ scope: "lane", side: "you", lane: 2 })).toHaveLength(4);
    const side = auraZones({ scope: "side", side: "you", lane: 2 });
    expect(side).toHaveLength(10);
    expect(side.every((id) => id.startsWith("zone-you-"))).toBe(true);
    const all = auraZones({ scope: "board", side: "you", lane: null });
    expect(all).toHaveLength(20);
    expect(new Set(all).size).toBe(20);
    expect(auraZones({ scope: "zone", side: "you", lane: null })).toEqual(side);
    // A modifier's lane and zone scopes stand on its side.
    const memory = createFxMemory();
    const sources: AuraSources = new Map();
    attributeModifiers([played("p1", "c9", "core-035"), modifier("p1", "m5"), resolved("p1", "c9", "core-035")], memory, sources);
    const zoned = parseFxCardTable("aura.json5", `{ "core-035": { aura: { look: "ember", scope: "side" } } }`);
    const view = board([], []);
    view.you.modifiers = [{ id: "m5", label: "" }];
    expect(planAuras(view, sources, zoned)[0]?.scope).toBe("side");
  });

  it("R3400000 a face-up Unit's tile aura covers its unit zone; a hidden Unit and its play's modifier wear none", () => {
    resetIds();
    expect(auraZones({ scope: "tile", side: "opponent", lane: 4 })).toEqual([testid.zone("opponent", "units", 4)]);
    const view = board([], []);
    view.you.units = [null, null, unit("p1", { defId: "traininggrounds-038" }), null, null];
    view.opponent.units = [unit("p2", { defId: "hidden", instanceId: "hidden" }), null, null, null, null];
    const [tile, ...rest] = planAuras(view, new Map(), TABLE);
    expect(rest).toEqual([]);
    expect(tile).toMatchObject({ key: `unit-${view.you.units[2]?.instanceId ?? ""}`, defId: "traininggrounds-038", side: "you", lane: 3, scope: "tile", look: "chalk" });
    expect(tile === undefined ? [] : auraZones(tile)).toEqual([testid.zone("you", "units", 3)]);
    // The same Unit seen by the other seat stands on that seat's opponent side.
    const theirs = board([], [], "p2");
    theirs.opponent.units = view.you.units.map((u) => (u === null ? null : { ...u, owner: "p1", controller: "p1" }));
    expect(planAuras(theirs, new Map(), TABLE).map((aura) => [aura.side, aura.lane, aura.scope])).toEqual([["opponent", 3, "tile"]]);
    // A modifier its play adds wears no tile aura: the aura stands with the Unit.
    const memory = createFxMemory();
    const sources: AuraSources = new Map();
    attributeModifiers([played("p1", "u9", "traininggrounds-038"), modifier("p1", "m7"), resolved("p1", "u9", "traininggrounds-038")], memory, sources);
    const modded = board([], []);
    modded.you.modifiers = [{ id: "m7", label: "" }];
    expect(sources.size).toBe(1);
    expect(planAuras(modded, sources, TABLE)).toEqual([]);
    // A tile aura counts toward a zone's cap like any other.
    const capped = stackAuras([tracked("t1", "tile", 1, "standing", "you", 2), tracked("z1", "side", 2), tracked("z2", "board", 3)]);
    expect(keys(capped)).toEqual(["t1", "z1"]);
  });

  it("R2787000 a face-down card never wears an aura, on either seat: your own unrevealed Field Trap, the other seat's backs and a Unit standing in a backrow zone", () => {
    resetIds();
    const p1 = board([field("p1", "core-018", { unrevealed: true }), field("p1", "core-046", { unit: unit("p1") }), faceDownBackrow], [faceDownBackrow, faceDownBackrow, faceDownBackrow]);
    expect(planAuras(p1, new Map(), TABLE)).toEqual([]);
    const p2 = board([field("p2", "core-018", { unrevealed: true }), faceDownBackrow], [faceDownBackrow, faceDownBackrow], "p2");
    expect(planAuras(p2, new Map(), TABLE)).toEqual([]);
    // Once it reveals, it wears its aura on both seats.
    const revealed = field("p1", "core-018");
    expect(planAuras(board([revealed], []), new Map(), TABLE).map((aura) => aura.defId)).toEqual(["core-018"]);
    expect(planAuras(board([], [revealed], "p2"), new Map(), TABLE).map((aura) => [aura.defId, aura.side])).toEqual([["core-018", "opponent"]]);
  });

  it("R2787000 a modifier wears the aura of the readable play that added it, and none for a hidden play, a trap firing inside the play, no play or a Field card's own", () => {
    const view = (ids: readonly string[], player: PlayerId = "p1"): PlayerView => {
      const out = board([], []);
      const seat = player === "p1" ? out.you : out.opponent;
      seat.modifiers = ids.map((id) => ({ id, label: "" }));
      return out;
    };
    const memory = createFxMemory();
    const sources: AuraSources = new Map();
    attributeModifiers([played("p1", "c9", "core-035"), modifier("p1", "m5"), resolved("p1", "c9", "core-035")], memory, sources);
    expect(sources.get(sourceKey("p1", "m5"))).toEqual({ defId: "core-035", instanceId: "c9" });
    expect(planAuras(view(["m5"]), sources, TABLE).map((aura) => aura.key)).toEqual(["mod-p1-m5"]);
    // The modifier gone from the view, its aura is gone, whatever the source map still holds.
    attributeModifiers([modifier("p1", "m5", false)], memory, sources);
    expect(planAuras(view([]), sources, TABLE)).toEqual([]);

    attributeModifiers([played("p2", "hidden", "hidden"), modifier("p2", "m6"), resolved("p2", "hidden", "hidden")], memory, sources);
    expect(sources.has(sourceKey("p2", "m6"))).toBe(false);
    expect(planAuras(view(["m6"], "p2"), sources, TABLE)).toEqual([]);

    attributeModifiers([played("p1", "c10", "core-035"), trapFired("p2", "backrow", 2), modifier("p1", "m7"), resolved("p1", "c10", "core-035")], memory, sources);
    expect(sources.has(sourceKey("p1", "m7"))).toBe(false);

    attributeModifiers([modifier("p1", "m8")], memory, sources);
    expect(sources.has(sourceKey("p1", "m8"))).toBe(false);

    // Added again with no play resolving: the old source no longer holds.
    attributeModifiers([played("p1", "c11", "core-035"), modifier("p1", "m9"), resolved("p1", "c11", "core-035"), modifier("p1", "m9")], memory, sources);
    expect(sources.has(sourceKey("p1", "m9"))).toBe(false);

    // core-014 is a Field Spell: its aura stands with the card, not with its modifier.
    attributeModifiers([played("p1", "b3", "core-014"), modifier("p1", "m10"), resolved("p1", "b3", "core-014")], memory, sources);
    expect(sources.get(sourceKey("p1", "m10"))).toEqual({ defId: "core-014", instanceId: "b3" });
    expect(planAuras(view(["m10"]), sources, TABLE)).toEqual([]);
  });

  it("R2787000 the layer remembers at most AURA_SOURCES_LIMIT modifiers' sources, the oldest evicted", () => {
    const memory = createFxMemory();
    const sources: AuraSources = new Map();
    const events = Array.from({ length: AURA_SOURCES_LIMIT + 2 }, (_, i) => modifier("p1", `m${String(i)}`));
    attributeModifiers([played("p1", "c9", "core-035"), ...events, resolved("p1", "c9", "core-035")], memory, sources);
    expect(sources.size).toBe(AURA_SOURCES_LIMIT);
    expect(sources.has(sourceKey("p1", "m0"))).toBe(false);
    expect(sources.has(sourceKey("p1", "m1"))).toBe(false);
    expect(sources.has(sourceKey("p1", `m${String(AURA_SOURCES_LIMIT + 1)}`))).toBe(true);
  });
});

const replaced = (player: PlayerId, anomaly?: AnomalyCode): GameEvent => (anomaly === undefined ? { type: "boardReplaced", player } : { type: "boardReplaced", player, anomaly });

describe("R3337003 an Anomaly's backdrop", () => {
  it("R3337003 an Anomaly's backdrop stands until its side is replaced or the game ends", () => {
    resetIds();
    const boards = new Map<PlayerId, AnomalyCode>();
    const backdrops = (view: PlayerView = board([], [])): (readonly unknown[])[] =>
      planAuras(view, new Map(), TABLE, boards).map((aura) => [aura.key, aura.side, aura.look, aura.anomaly]);
    noteAnomalies([replaced("p1", "WH")], boards);
    const planned = planAuras(board([], []), new Map(), TABLE, boards);
    expect(planned).toHaveLength(1);
    expect(planned[0]).toMatchObject({ key: "anomaly-p1", side: "you", lane: null, scope: "side", look: "void", anomaly: "WH" });
    expect(planned[0]?.colors).toEqual(paletteLook("Tech/starfield")?.colors);
    expect(auraTestid(planned[0]?.key ?? "")).toBe("aura-anomaly-p1");
    // p1's side is the opponent's to p2.
    expect(backdrops(board([], [], "p2"))).toEqual([["anomaly-p1", "opponent", "void", "WH"]]);

    // The other player's replacement does not touch p1's; each side's backdrop follows its Field cards.
    noteAnomalies([replaced("p2")], boards);
    expect(backdrops()).toEqual([["anomaly-p1", "you", "void", "WH"]]);
    noteAnomalies([replaced("p2", "AL")], boards);
    const fields = board([field("p1", "core-014")], [field("p2", "meditativeplus-009")]);
    expect(planAuras(fields, new Map(), TABLE, boards).map((aura) => aura.anomaly ?? aura.defId)).toEqual(["core-014", "WH", "meditativeplus-009", "AL"]);

    // p1's side replaced by no Anomaly ends its backdrop; a new one replaces the code.
    noteAnomalies([replaced("p1")], boards);
    expect(backdrops()).toEqual([["anomaly-p2", "opponent", "drift", "AL"]]);
    noteAnomalies([replaced("p1", "WH"), replaced("p1", "FO")], boards);
    expect(backdrops()).toEqual([
      ["anomaly-p1", "you", "holy", "FO"],
      ["anomaly-p2", "opponent", "drift", "AL"],
    ]);

    // A finished game's view plans none, and its gameOver forgets them all.
    expect(backdrops(board([], [], "p1", { result: { winner: "p1", reason: "hero-death" } }))).toEqual([]);
    noteAnomalies([{ type: "gameOver", winner: "p1", reason: "hero-death" }], boards);
    expect(boards.size).toBe(0);
    expect(backdrops()).toEqual([]);
  });
});

describe("R2787000 how auras stack", () => {
  it("R2787000 at most AURA_MAX_PER_ZONE cover any zone", () => {
    const auras = [
      tracked("board", "board", 1),
      tracked("you", "side", 2),
      tracked("them", "side", 3, "standing", "opponent"),
      tracked("lane1", "lane", 4),
      tracked("lane3", "lane", 5, "standing", "you", 3),
      tracked("zone1", "zone", 6),
      tracked("zone2", "zone", 7, "standing", "you", 2),
      tracked("zone1b", "zone", 8, "standing", "opponent", 1),
    ];
    for (const max of [1, 2, 3]) {
      const counts = new Map<string, number>();
      for (const aura of stackAuras(auras, max)) for (const zone of auraZones(aura.spec)) counts.set(zone, (counts.get(zone) ?? 0) + 1);
      expect(Math.max(...counts.values()), String(max)).toBeLessThanOrEqual(max);
    }
    expect(AURA_MAX_PER_ZONE).toBe(2);
  });

  it("R2787000 the nearest scope wins, then the newest; a zone and its lane bury the side and the board", () => {
    const auras = [tracked("board", "board", 4), tracked("side", "side", 3), tracked("lane", "lane", 1), tracked("zone", "zone", 2)];
    expect(keys(stackAuras(auras))).toEqual(["zone", "lane"]);
    // A zone on the other side leaves room for both: no zone holds more than two.
    const apart = [tracked("board", "board", 4), tracked("side", "side", 3), tracked("zone", "zone", 2, "standing", "opponent", 4)];
    expect(keys(stackAuras(apart))).toEqual(["zone", "side", "board"]);
    // A second side aura over the opponent's zone fills it, so the board aura goes.
    const full = [...apart, tracked("theirs", "side", 1, "standing", "opponent")];
    expect(keys(stackAuras(full))).toEqual(["zone", "side", "theirs"]);
    // Two of one scope: the newer first, then input order.
    const sides = [tracked("old", "side", 1), tracked("new", "side", 3), tracked("mid", "side", 2)];
    expect(keys(stackAuras(sides))).toEqual(["new", "mid"]);
    expect(keys(stackAuras([tracked("a", "side", 1), tracked("b", "side", 1)], 1))).toEqual(["a"]);
  });

  it("R2787000 an aura fading out comes after every aura still holding", () => {
    const auras = [tracked("zone", "zone", 9, "end"), tracked("side", "side", 1), tracked("board", "board", 2)];
    expect(keys(stackAuras(auras))).toEqual(["side", "board"]);
    expect(keys(stackAuras([tracked("zone", "zone", 9, "end"), tracked("side", "side", 1)]))).toEqual(["side", "zone"]);
  });
});

describe("R2787000 how an aura begins and ends", () => {
  const a = spec("a", "zone");
  const b = spec("b", "side");

  it("R2787000 what the first view shows stands; an arrival begins and stands after AURA_FADE_IN_MS; a departure ends and is gone after AURA_FADE_OUT_MS", () => {
    const first = reconcileAuras(NO_AURAS, [a], 0, true);
    expect(first.auras.map((aura) => [aura.spec.key, aura.phase])).toEqual([["a", "standing"]]);
    expect(first.started).toBe(true);
    const arrived = reconcileAuras(first, [a, b], 100, true);
    expect(arrived.auras.map((aura) => [aura.spec.key, aura.phase, aura.since])).toEqual([
      ["a", "standing", 0],
      ["b", "begin", 100],
    ]);
    expect(arrived.auras[1]?.seen).toBeGreaterThan(arrived.auras[0]?.seen ?? Infinity);
    expect(settleAuras(arrived, 100 + AURA_FADE_IN_MS - 1)).toBe(arrived);
    const stood = settleAuras(arrived, 100 + AURA_FADE_IN_MS);
    expect(stood.auras.map((aura) => aura.phase)).toEqual(["standing", "standing"]);
    const left = reconcileAuras(stood, [a], 2000, true);
    expect(left.auras.map((aura) => [aura.spec.key, aura.phase, aura.since])).toEqual([
      ["a", "standing", 0],
      ["b", "end", 2000],
    ]);
    expect(settleAuras(left, 2000 + AURA_FADE_OUT_MS - 1)).toBe(left);
    expect(keys(settleAuras(left, 2000 + AURA_FADE_OUT_MS).auras)).toEqual(["a"]);
    // One that comes back while it fades begins again.
    const back = reconcileAuras(left, [a, b], 2100, true);
    expect(back.auras.map((aura) => [aura.spec.key, aura.phase, aura.since])).toEqual([
      ["a", "standing", 0],
      ["b", "begin", 2100],
    ]);
  });

  it("R2787000 without motion nothing begins or ends: an arrival stands, a departure is gone at once and a begun aura stands", () => {
    const first = reconcileAuras(NO_AURAS, [a], 0, true);
    const arrived = reconcileAuras(first, [a, b], 100, false);
    expect(arrived.auras.map((aura) => [aura.spec.key, aura.phase])).toEqual([
      ["a", "standing"],
      ["b", "standing"],
    ]);
    expect(keys(reconcileAuras(arrived, [a], 200, false).auras)).toEqual(["a"]);
    const begun = reconcileAuras(first, [a, b], 100, true);
    expect(reconcileAuras(begun, [a, b], 150, false).auras.map((aura) => aura.phase)).toEqual(["standing", "standing"]);
    const ending = reconcileAuras(begun, [a], 150, true);
    expect(keys(reconcileAuras(ending, [a], 160, false).auras)).toEqual(["a"]);
  });

  it("R2787000 an aura takes its new spec while it holds", () => {
    const first = reconcileAuras(NO_AURAS, [a], 0, true);
    const moved = { ...a, lane: 4 };
    expect(reconcileAuras(first, [moved], 10, true).auras[0]).toEqual({ spec: moved, phase: "standing", since: 0, seen: 1 });
  });
});

describe("R2787000 the motes' budget", () => {
  it("R2787000 each aura draws at most AURA_MOTES_PER_AURA times the intensity, and all of them at most AURA_PARTICLE_SHARE of the cap", () => {
    for (const [name, intensity] of Object.entries(FX_INTENSITY_SCALE)) {
      for (const cap of [FX_PARTICLE_CAP, FX_PARTICLE_CAP_MOBILE]) {
        expect(auraBudget(0, intensity, cap)).toBe(0);
        for (let n = 1; n <= 8; n += 1) {
          const each = auraBudget(n, intensity, cap);
          expect(each, `${name} ${String(cap)} ${String(n)}`).toBeLessThanOrEqual(Math.floor(AURA_MOTES_PER_AURA * intensity));
          expect(each * n, `${name} ${String(cap)} ${String(n)}`).toBeLessThanOrEqual(Math.floor(AURA_PARTICLE_SHARE * cap));
          expect(each).toBeGreaterThanOrEqual(0);
        }
      }
    }
    expect(auraBudget(1, FX_INTENSITY_SCALE.off, FX_PARTICLE_CAP)).toBe(0);
    expect(auraBudget(1, FX_INTENSITY_SCALE.normal, FX_PARTICLE_CAP)).toBe(AURA_MOTES_PER_AURA);
  });

  it("R2787000 every look's ranges run low to high and every opacity is in (0, 1]", () => {
    expect(Object.keys(AURA_LOOKS)).toEqual([...AURA_LOOK_NAMES]);
    expect(AURA_SCOPES).toEqual(["tile", "zone", "lane", "side", "board"]);
    for (const name of AURA_LOOK_NAMES) {
      const look = AURA_LOOKS[name];
      for (const range of [look.vx, look.vy, look.size, look.life]) expect(range[0], name).toBeLessThanOrEqual(range[1]);
      expect(look.size[0], name).toBeGreaterThan(0);
      expect(look.life[0], name).toBeGreaterThan(0);
      expect(look.glow, name).toBeGreaterThan(0);
      for (const opacity of [look.alpha, look.tint, look.edge]) {
        expect(opacity, name).toBeGreaterThan(0);
        expect(opacity, name).toBeLessThanOrEqual(1);
      }
    }
  });
});
