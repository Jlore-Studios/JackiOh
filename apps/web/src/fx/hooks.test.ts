// R2797000: a card's Death, start- or end-of-turn hook, Whenever trigger and Activate play their table
// entry from their own `hookResolving` marker (R2783000), and a Trap's `cast` from its `trapFired`
// (memory.ts opens the hook's entry, cardFx.ts `planCardMotif` keys it), planned the way FxLayer plans:
// remember, then plan. A hidden card's marker keys nothing (R202), and a modifier a hook adds wears no
// aura (R2787000).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { fullBoardView } from "../test/fixtures.ts";
import { attributeModifiers, sourceKey, type AuraSources } from "./aura/plan.ts";
import { planCardMotif } from "./cardFx.ts";
import type { FxCardEntry, FxHookSpec } from "./cards/tables.ts";
import { FX_INTENSITY_SCALE } from "./constants.ts";
import { createFxMemory } from "./memory.ts";
import { galleryEntry, motifFixture } from "./motifs/gallery.ts";
import type { FxCardHook } from "./motifs/names.ts";
import type { FxPlanEnv } from "./types.ts";

type HookKind = Extract<GameEvent, { type: "hookResolving" }>["hook"];

/** A Unit with no table entry of its own here: each test hands `planCardMotif` its own table. */
const UNIT = "classic-071";
/** A Trap. */
const TRAP = "core-041";
const SPELL = "core-035";

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

const marker = (hook: HookKind, sourceId: string, defId: string = UNIT, controller: PlayerId = "p1"): GameEvent => ({ type: "hookResolving", sourceId, defId, controller, hook });
const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 });

const BOLT: FxHookSpec = { motif: "bolt" };

function tableOf(defId: string, hooks: Partial<Record<FxCardHook, FxHookSpec>>): ReadonlyMap<string, FxCardEntry> {
  return new Map([[defId, { kind: "hooks", hooks }]]);
}

/** Remembers `before`, then plans the bolt fixture dealt from `sourceId` (null: no source) under `table`. */
function boltUnder(before: readonly GameEvent[], sourceId: string | null, table: ReadonlyMap<string, FxCardEntry>, view: PlayerView = fullBoardView()): ReturnType<typeof planCardMotif> {
  const env = envOf();
  env.memory.remember(before);
  const entry = { ...galleryEntry(motifFixture("bolt", view, sourceId, "p1"), view), durationMs: 400 };
  env.memory.remember(entry.events);
  return planCardMotif(entry, view, env, table);
}

/** Each marker kind and the table hook it opens. */
const OPENS: readonly (readonly [HookKind, FxCardHook])[] = [
  ["death", "death"],
  ["startOfTurn", "trigger"],
  ["endOfTurn", "trigger"],
  ["whenever", "trigger"],
  ["activate", "activate"],
];

describe("R2797000 a card's hooks play their entry from their marker", () => {
  it("R2797000 a Death, a start- or end-of-turn hook, a Whenever and an Activate play their own hook's motif from their marker", () => {
    for (const [kind, hook] of OPENS) {
      expect(boltUnder([marker(kind, "src")], "src", tableOf(UNIT, { [hook]: BOLT })), kind).not.toBeNull();
      // Only that hook: a Unit's `cry` is its play, and the other hooks are not this one.
      expect(boltUnder([marker(kind, "src")], "src", tableOf(UNIT, { cry: BOLT })), `${kind} cry`).toBeNull();
      for (const [, other] of OPENS) {
        if (other !== hook) expect(boltUnder([marker(kind, "src")], "src", tableOf(UNIT, { [other]: BOLT })), `${kind} ${other}`).toBeNull();
      }
    }
  });

  it("R2797000 a Trap's cast plays from its trapFired", () => {
    const fired: GameEvent = { type: "trapFired", instanceId: "trap", defId: TRAP, controller: "p1", row: "backrow", lane: 3 };
    expect(boltUnder([fired], "trap", tableOf(TRAP, { cast: BOLT }))).not.toBeNull();
    // Its firing is no Death or trigger.
    expect(boltUnder([fired], "trap", tableOf(TRAP, { trigger: BOLT }))).toBeNull();
    // Before this ruling a firing opened nothing: without it, the same hits key no entry.
    expect(boltUnder([], "trap", tableOf(TRAP, { cast: BOLT }))).toBeNull();
  });

  it("R2797000 a hook ends at the next marker, a cardResolved, an attack or a new turn", () => {
    const table = tableOf(UNIT, { death: BOLT });
    const closers: readonly GameEvent[] = [
      marker("cry", "other", SPELL),
      resolved("p1", "other", SPELL),
      { type: "countered", player: "p2", instanceId: "other", defId: SPELL, byInstanceId: null, to: "graveyard" },
      { type: "attackDeclared", attackerId: "u1", targetId: "hero-p2", forced: false },
      { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 },
      { type: "turnStarted", player: "p2", turn: 4 },
      { type: "gameOver", winner: "p1", reason: "hero-death" },
    ];
    expect(boltUnder([marker("death", "src")], "src", table)).not.toBeNull();
    for (const closer of closers) expect(boltUnder([marker("death", "src"), closer], "src", table), closer.type).toBeNull();
    // A second marker opens its own card's hook in the first one's place.
    expect(boltUnder([marker("death", "src"), marker("activate", "src2", "classic-080")], "src2", new Map([...table, ...tableOf("classic-080", { activate: BOLT })]))).not.toBeNull();
  });

  it("R2797000 a Cry's own marker opens nothing: a Unit's cry plays by its play", () => {
    const stream = [played("p1", "ex", UNIT), marker("cry", "ex")];
    expect(boltUnder(stream, "ex", tableOf(UNIT, { cry: BOLT }))).not.toBeNull();
    expect(boltUnder(stream, "ex", tableOf(UNIT, { death: BOLT }))).toBeNull();
    // A Death inside a play is that card's, not the play's: the Spell's cast does not draw on it, the dying card's death does.
    const inside = [played("p1", "ex", SPELL), marker("cry", "ex", SPELL), marker("death", "src")];
    expect(boltUnder(inside, "src", tableOf(SPELL, { cast: BOLT }))).toBeNull();
    expect(boltUnder(inside, "src", tableOf(UNIT, { death: BOLT }))).not.toBeNull();
    // Once the play resolves its next hits are the play's again.
    expect(boltUnder([...inside, resolved("p1", "ex", SPELL), played("p1", "ex2", SPELL)], "ex2", tableOf(SPELL, { cast: BOLT }))).not.toBeNull();
  });

  it("R2797000 R202 a marker naming the sentinel keys nothing, neither its card's entry nor the outer play's", () => {
    const table = new Map([...tableOf(SPELL, { cast: BOLT }), ...tableOf(UNIT, { death: BOLT })]);
    const outer = [played("p1", "ex", SPELL)];
    // A hit with no source is the play's while it resolves…
    expect(boltUnder(outer, null, table)).not.toBeNull();
    // …but under a hidden card's hook it is no one's.
    expect(boltUnder([...outer, marker("death", "hidden", "hidden")], null, table)).toBeNull();
    expect(boltUnder([...outer, marker("death", "hidden", "hidden")], "hidden", table)).toBeNull();
    const fired: GameEvent = { type: "trapFired", instanceId: "hidden", defId: "hidden", controller: "p2", row: "backrow", lane: 2 };
    expect(boltUnder([...outer, fired], null, table)).toBeNull();
  });

  it("R2797000 R2787000 a modifier a hook adds wears no aura", () => {
    const added = (before: readonly GameEvent[]): AuraSources => {
      const sources: AuraSources = new Map();
      attributeModifiers([...before, { type: "modifierChanged", player: "p1", modifierId: "m1", added: true }], createFxMemory(), sources);
      return sources;
    };
    expect(added([played("p1", "ex", SPELL)]).get(sourceKey("p1", "m1"))).toEqual({ defId: SPELL, instanceId: "ex" });
    for (const [kind] of OPENS) expect(added([played("p1", "ex", SPELL), marker(kind, "src")]).has(sourceKey("p1", "m1")), kind).toBe(false);
    expect(added([marker("activate", "src")]).has(sourceKey("p1", "m1"))).toBe(false);
    const fired: GameEvent = { type: "trapFired", instanceId: "trap", defId: TRAP, controller: "p1", row: "backrow", lane: 3 };
    expect(added([fired]).has(sourceKey("p1", "m1"))).toBe(false);
  });
});
