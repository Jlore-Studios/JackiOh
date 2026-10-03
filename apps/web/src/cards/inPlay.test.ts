// inPlay.ts: the words a face in play prints where play and print part ways (SPEC §10.10).
//
// `POWER_WORDS` is the client's copy of §8 #98's thirteen titles and clauses (patch v0.2.1), keyed
// by the stored name the view gives a rolled power (R103, R243). The engine holds the same words
// beside each power's effects, and the client cannot load the engine online, so this test is what
// keeps the two tables one table — and holds both to the catalog's printed list.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { subsystems } from "@jackioh/engine";
import { fillParams } from "@jackioh/shared";

import {
  CONCEALED_TAG,
  CONCEALED_TEXT_LOUD,
  HEROIC_POWER_ID,
  POWER_WORDS,
  concealedInPlay,
  concealedText,
  fillPowerParams,
  powerLine,
  powerText,
  powerTitle,
  powerX,
} from "./inPlay.ts";

function heroic() {
  const def = CATALOG[HEROIC_POWER_ID];
  if (def === undefined) throw new Error("the catalog has no #98");
  return def;
}

describe("#98 Heroic Power's rolled power, in words", () => {
  it("names every power the engine can roll, in the engine's own titles and words, base and radiant, with its X", () => {
    expect(Object.keys(POWER_WORDS)).toEqual([...subsystems.HERO_POWER_NAMES]);
    expect(Object.keys(POWER_WORDS)).toHaveLength(13);
    for (const power of subsystems.HERO_POWERS) {
      expect(POWER_WORDS[power.name], power.name).toEqual({
        title: power.title,
        radiantTitle: power.radiantTitle,
        base: power.label,
        radiant: power.radiantLabel,
      });
      expect(powerX(power.name), power.name).toBe(power.x);
    }
  });

  it("every power reads, title, X and clause, exactly as the catalog's printed list prints it, on both faces", () => {
    const def = heroic();
    for (const radiant of [false, true]) {
      const printed = fillParams(def, radiant ? "radiant" : "base");
      for (const name of Object.keys(POWER_WORDS)) {
        const x = powerX(name) ?? 0;
        const clause = fillPowerParams(`(${String(x)}) ${powerTitle(name, radiant) ?? ""}: ${radiant ? POWER_WORDS[name]?.radiant : POWER_WORDS[name]?.base}.`, radiant, def.params);
        expect(printed, `${name} radiant ${String(radiant)}`).toContain(clause);
      }
    }
  });

  it("prints the keyword line, then the one power as an Activate ability with its X, and nothing of the other twelve", () => {
    const text = powerText({ name: "recruit", x: 3 }, false, "Indestructible");
    expect(text).toBe("Indestructible\nActivate: Spend (3): Expedition Map: Recruit a permanent.");
    for (const other of ["Life Tap", "Take 2 damage", "Ping", "Rush Token", "Felinor Token", "Discover a Unit", "13 random powers", "Playing it"]) {
      expect(text).not.toContain(other);
    }
  });

  it("prints the Radiant title and clause on a radiant face: Armor Up's is Tank Up", () => {
    expect(powerText({ name: "discover", x: 2 }, true, "Indestructible")).toBe(
      "Indestructible\nActivate: Spend (2): Witness Value: Discover a Radiant Unit.",
    );
    expect(powerTitle("armor", false)).toBe("Armor Up");
    expect(powerTitle("armor", true)).toBe("Tank Up");
    expect(powerLine({ name: "armor", x: 1 }, true)).toBe("Activate: Spend (1): Tank Up: Your hero gains 4 Armor. Refresh this power.");
  });

  it("R637 Steady Shot's {shot} is filled from the view's number, else the printed one, and never shows raw", () => {
    const line = powerLine({ name: "burn", x: 1 }, true) ?? "";
    expect(line).toContain("{shot}");
    expect(fillPowerParams(line, true, heroic().params, { shot: 8 })).toBe(
      "Activate: Spend (1): Steady Shot: Deal 8 damage to the enemy hero. Upgrade this permanently by +2 damage.",
    );
    expect(fillPowerParams(line, true, heroic().params)).toContain("Deal 4 damage");
    expect(fillPowerParams(powerLine({ name: "burn", x: 1 }, false) ?? "", false, heroic().params)).toContain("Deal 2 damage");
    // With no catalog to hand, the view's number alone fills it.
    expect(fillPowerParams(line, true, undefined, { shot: 6 })).toContain("Deal 6 damage");
  });

  it("leaves a name it does not know to the printed text, a prototype key included", () => {
    expect(powerText({ name: "not-a-power", x: 1 }, false, "Indestructible")).toBeNull();
    expect(powerText({ name: "toString", x: 1 }, false, "Indestructible")).toBeNull();
    expect(powerTitle("constructor", false)).toBeNull();
    expect(powerX("toString")).toBeNull();
  });

  it("is the card the catalog calls #98", () => {
    expect(CATALOG[HEROIC_POWER_ID]?.index).toBe("98");
  });
});

describe("Call to Chaos in play", () => {
  it("conceals exactly the cards that carry the Call to Chaos tag: Core #95 and the Classic+ Edition #73", () => {
    const concealed = Object.values(CATALOG).filter((def) => concealedInPlay(def.tags));
    expect(concealed.map((def) => def.id)).toEqual(["core-095", "classicplus-073"]);
    expect(CATALOG["core-095"]?.tags).toContain(CONCEALED_TAG);
  });

  it("reads ??? in play, and the Classic+ Edition's Radiant face its designer's !!!", () => {
    expect(concealedText("core-095", false)).toBe("???");
    expect(concealedText("core-095", true)).toBe("???");
    expect(concealedText("classicplus-073", false)).toBe("???");
    expect(concealedText("classicplus-073", true)).toBe(CONCEALED_TEXT_LOUD);
    expect(CONCEALED_TEXT_LOUD).toBe("!!!");
  });
});
