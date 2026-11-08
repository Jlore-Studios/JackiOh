// R3389000: a signature's own aura looks (looks.ts `AURA_SIGNATURE_LOOKS`), each within the signature
// tier's cap and worn by one signature alone, and a Unit's `zone` aura standing on its own unit zone
// (plan.ts), never for a hidden Unit and never again on what it installs.

import { describe, expect, it } from "vitest";

import { testid } from "../../game/contract.ts";
import { baseView, emptySide, faceDownBackrow, faceUpBackrow, unit } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { entryAura, FX_CARD_TABLE, isSignature, parseFxCardTable } from "../cards/tables.ts";
import { AURA_SIGNATURE_MAX_INTENSITY } from "../constants.ts";
import { createFxMemory } from "../memory.ts";
import { AURA_LOOKS, AURA_LOOK_NAMES, AURA_SIGNATURE_LOOK_NAMES, AURA_SIGNATURE_LOOKS, auraLookSpec, isAuraLook, isSignatureAuraLook } from "./looks.ts";
import { attributeModifiers, auraZones, planAuras, stackAuras, reconcileAuras, NO_AURAS, type AuraSources } from "./plan.ts";

describe("R3389000 a signature's own aura", () => {
  it("R3389000 every signature look keeps its ranges and cap, no two equal", () => {
    expect(Object.keys(AURA_SIGNATURE_LOOKS)).toEqual([...AURA_SIGNATURE_LOOK_NAMES]);
    for (const name of AURA_SIGNATURE_LOOK_NAMES) {
      const look = AURA_SIGNATURE_LOOKS[name];
      expect(auraLookSpec(name), name).toBe(look);
      expect(isAuraLook(name) && isSignatureAuraLook(name), name).toBe(true);
      for (const range of [look.vx, look.vy, look.size, look.life]) expect(range[0], name).toBeLessThanOrEqual(range[1]);
      expect(look.size[0], name).toBeGreaterThan(0);
      expect(look.life[0], name).toBeGreaterThan(0);
      expect(look.glow, name).toBeGreaterThan(0);
      for (const opacity of [look.alpha, look.tint, look.edge]) {
        expect(opacity, name).toBeGreaterThan(0);
        expect(opacity, name).toBeLessThanOrEqual(AURA_SIGNATURE_MAX_INTENSITY);
      }
    }
    // The shared nine stay shared, and no look, shared or a signature's, has another's numbers.
    for (const name of AURA_LOOK_NAMES) {
      expect(isSignatureAuraLook(name), name).toBe(false);
      expect(auraLookSpec(name), name).toBe(AURA_LOOKS[name]);
    }
    const specs = [...AURA_LOOK_NAMES.map((name) => AURA_LOOKS[name]), ...AURA_SIGNATURE_LOOK_NAMES.map((name) => AURA_SIGNATURE_LOOKS[name])].map((look) => JSON.stringify(look));
    expect(new Set(specs).size).toBe(specs.length);
    // Each moves as its concept does: the iris slowly, the breeze sideways, the smoke and the moss rising, the whirlpool widest.
    expect(AURA_SIGNATURE_LOOKS.seeingIris.vx).toEqual([-4, 4]);
    expect(AURA_SIGNATURE_LOOKS.graveBreeze.vx[0]).toBeGreaterThan(0);
    expect(AURA_SIGNATURE_LOOKS.craterSmoke.vy[1]).toBeLessThan(0);
    expect(AURA_SIGNATURE_LOOKS.mossCreep.vy[1]).toBeLessThan(0);
    expect(AURA_SIGNATURE_LOOKS.abyssWhirl.vx).toEqual([-14, 14]);
  });

  it("R3389000 each signature look is worn by one card, a signature", () => {
    const wearers = new Map<string, string[]>();
    for (const [defId, entry] of FX_CARD_TABLE) {
      const aura = entryAura(entry);
      if (aura === undefined || !isSignatureAuraLook(aura.look)) continue;
      expect(isSignature(entry), defId).toBe(true);
      wearers.set(aura.look, [...(wearers.get(aura.look) ?? []), defId]);
    }
    expect([...wearers.keys()].sort()).toEqual([...AURA_SIGNATURE_LOOK_NAMES].sort());
    for (const [look, defIds] of wearers) expect(defIds, look).toHaveLength(1);
    expect(Object.fromEntries([...wearers].map(([look, [defId]]) => [defId, look]))).toEqual({
      "classic-004": "seeingIris",
      "classic-007": "sceptreLoop",
      "classic-028": "graveBreeze",
      "classic-033": "silkWeb",
      "classic-045": "mossCreep",
      "classic-056": "crownSigil",
      "classic-080": "craterSmoke",
      "classic-085": "cloakShimmer",
      "classic-090": "abyssWhirl",
    });
  });

  it("R3389000 a Unit's zone aura stands on its own unit zone, never hidden or on its modifiers", () => {
    const titan = unit("p1", { defId: "classic-045", instanceId: "titan" });
    const theirs = unit("p2", { defId: "classic-080", instanceId: "max" });
    const hidden = unit("p2", { defId: "hidden", instanceId: "hidden" });
    const view = baseView({
      you: emptySide("p1", { units: [null, titan, null, null, null], backrow: [faceUpBackrow("p1", { defId: "classic-090", instanceId: "deep", type: "Field Spell" }), faceDownBackrow, null, null, null] }),
      opponent: emptySide("p2", { units: [theirs, null, hidden, null, null] }),
    });
    const planned = planAuras(view, new Map());
    expect(planned.map((aura) => [aura.key, aura.side, aura.lane, aura.scope, aura.look, aura.row])).toEqual([
      ["field-deep", "you", 1, "zone", "abyssWhirl", undefined],
      ["unit-titan", "you", 2, "zone", "mossCreep", "units"],
      ["unit-max", "opponent", 1, "zone", "craterSmoke", "units"],
    ]);
    // It covers its own unit zone alone, as a Field card's covers its backrow zone.
    const [field, mine, other] = planned;
    if (field === undefined || mine === undefined || other === undefined) throw new Error("three auras");
    expect(auraZones(field)).toEqual([testid.zone("you", "backrow", 1)]);
    expect(auraZones(mine)).toEqual([testid.zone("you", "units", 2)]);
    expect(auraZones(other)).toEqual([testid.zone("opponent", "units", 1)]);
    // Each on its own zone, so all three show, however many a zone may hold.
    expect(stackAuras(reconcileAuras(NO_AURAS, planned, 0, false).auras, 1).map((aura) => aura.spec.key).sort()).toEqual(["field-deep", "unit-max", "unit-titan"]);
    // The planner's defId never reaches what a zone shows: a hidden Unit wears none (R202).
    expect(planned.some((aura) => aura.instanceId === "hidden")).toBe(false);

    // A modifier its play installs wears no second copy of it: the aura stands with the Unit itself.
    const memory = createFxMemory();
    const sources: AuraSources = new Map();
    attributeModifiers(
      [
        { type: "cardPlayed", player: "p1", instanceId: "titan", defId: "classic-045", costPaid: 2 },
        { type: "modifierChanged", player: "p1", modifierId: "m1", added: true },
        { type: "cardResolved", player: "p1", instanceId: "titan", defId: "classic-045", permanent: true, costPaid: 2 },
      ],
      memory,
      sources,
    );
    expect(sources.size).toBe(1);
    const withModifier = { ...view, you: { ...view.you, modifiers: [{ id: "m1", label: "a rider" }] } };
    expect(planAuras(withModifier, sources).map((aura) => aura.key)).toEqual(["field-deep", "unit-titan", "unit-max"]);

    // A Unit's side or board aura still stands with what it installs; a Unit with no aura wears none.
    const table = parseFxCardTable("test.json5", `{ "classic-045": { bespoke: "titanLeaf", entrance: "titanBloom", aura: { look: "mossCreep", scope: "side" }, signature: true } }`);
    expect(planAuras(withModifier, sources, table).map((aura) => [aura.key, aura.scope])).toEqual([["mod-p1-m1", "side"]]);
    expect(CATALOG["classic-006"]?.type).toBe("Unit");
    const plain = baseView({ you: emptySide("p1", { units: [unit("p1", { defId: "classic-006", instanceId: "toe" }), null, null, null, null] }) });
    expect(planAuras(plain, new Map())).toEqual([]);
    // A Field card standing in a unit zone (Animated, R383) is no Unit: its zone aura is its backrow zone's alone.
    expect(CATALOG["core-040"]?.type).toBe("Field Spell");
    const animated = baseView({ you: emptySide("p1", { units: [unit("p1", { defId: "core-040", instanceId: "echo" }), null, null, null, null] }) });
    expect(planAuras(animated, new Map())).toEqual([]);
  });
});
