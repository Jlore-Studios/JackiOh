// The words a face in play prints where play and print part ways (SPEC §10.10), worked out by jackioh-shared's
// face model (`face/power.rs`). `POWER_WORDS` is the crate's data file of §8 #98's thirteen powers, keyed by the
// view's power name (R243). The engine holds the same clauses, so this test keeps the two tables one.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { subsystems } from "@jackioh/engine";
import { fillParams, setShips, type CardDef } from "@jackioh/shared";

import { faceModel } from "./model.ts";
import { POWER_WORDS } from "./faceData.ts";

const HEROIC_POWER_ID = "core-098";
const CONCEALED_TAG = "Call to Chaos";
const CONCEALED_TEXT_LOUD = "!!!";

/** The #98 Heroic Power's text in play for a rolled power, as the face model prints it (jackioh-shared `face/power.rs`). */
function powerFace(name: string, radiant: boolean, params?: Record<string, number>): string {
  const def = CATALOG[HEROIC_POWER_ID];
  return faceModel({ defId: HEROIC_POWER_ID, ...(def === undefined ? {} : { def }), radiant, inPlay: { power: { name }, ...(params === undefined ? {} : { params }) } }).text.full;
}

function concealedText(defId: string, radiant: boolean): string {
  return faceModel({ defId, def: CATALOG[defId], radiant, inPlay: {} }).text.full;
}

function concealedInPlay(tags: readonly string[]): boolean {
  return tags.includes(CONCEALED_TAG);
}

describe("#98 Heroic Power's rolled power, in words (R752)", () => {
  it("names every power the engine can roll, with the engine's X, names and words, base and radiant", () => {
    expect(Object.keys(POWER_WORDS).sort()).toEqual([...subsystems.HERO_POWER_NAMES].sort());
    for (const power of subsystems.HERO_POWERS) {
      expect(POWER_WORDS[power.name], power.name).toEqual({
        x: power.x,
        title: power.title,
        radiantTitle: power.radiantTitle,
        base: power.label,
        radiant: power.radiantLabel,
      });
    }
  });

  it("prints the keyword line, then the one power's name and its Activate, and nothing of the other twelve", () => {
    const text = powerFace("recruit", false);
    expect(text).toBe("Indestructible\nExpedition Map\nActivate: Spend (3): Recruit a permanent.");
    for (const other of ["Life Tap", "Deal 1 damage", "Rush Token", "Felinor Token", "Discover a Unit"]) {
      expect(text).not.toContain(other);
    }
  });

  it("prints the Radiant words and name on a radiant face, filling the card's numbers", () => {
    expect(powerFace("discover", true)).toMatch(/Witness Value\nActivate: Spend \(2\): Discover a Radiant Unit\.$/u);
    expect(powerFace("armor", true, { armor: 4 })).toMatch(/Tank Up\nActivate: Spend \(1\): Your hero gains 4 Armor, then this power refreshes\.$/u);
    expect(powerFace("burn", false, { shot: 6 })).toMatch(/Steady Shot\nActivate: Spend \(1\): Deal 6 damage to the enemy hero\.$/u);
    expect(POWER_WORDS["armor"]?.title).toBe("Armor Up");
  });

  it("leaves a name it does not know to the printed text", () => {
    expect(powerFace("not-a-power", false)).toBe(fillParams(CATALOG[HEROIC_POWER_ID] as CardDef, "base"));
  });

  it("is the card the catalog calls #98", () => {
    expect(CATALOG[HEROIC_POWER_ID]?.index).toBe("98");
  });

  it("R1430 fills each power's words with the numbers the catalog gives that power, and only those", () => {
    const params = CATALOG[HEROIC_POWER_ID]?.params ?? [];
    for (const [name, words] of Object.entries(POWER_WORDS)) {
      const written = [...`${words.base} ${words.radiant}`.matchAll(/\{(\w+)\}/g)].map((match) => match[1] ?? "");
      const own = params.filter((param) => param.power === name).map((param) => param.key);
      expect([...new Set(written)].sort(), name).toEqual([...own].sort());
    }
  });
});

describe("Call to Chaos in play", () => {
  it("conceals exactly the cards that carry the Call to Chaos tag: Core #95, the Classic+ Edition #73, the Meditative Edition #95, the Stinkyscape set's two and the Training-Grounds Edition #39", () => {
    // R1857000: the Stinkyscape set's two editions (#150, #151, #1884); Training Grounds' #39 (#2204).
    const concealed = Object.values(CATALOG).filter((def) => setShips(def.set) && concealedInPlay(def.tags));
    expect(concealed.map((def) => def.id)).toEqual([
      "core-095",
      "classicplus-073",
      "meditative-095",
      "stinkyscape-150",
      "stinkyscape-151",
      "traininggrounds-039",
    ]);
    expect(CATALOG["core-095"]?.tags).toContain(CONCEALED_TAG);
  });

  it("reads ??? in play, and the Classic+ and Meditative Editions' Radiant faces the designer's !!!", () => {
    expect(concealedText("core-095", false)).toBe("???");
    expect(concealedText("core-095", true)).toBe("???");
    expect(concealedText("classicplus-073", false)).toBe("???");
    expect(concealedText("classicplus-073", true)).toBe(CONCEALED_TEXT_LOUD);
    expect(concealedText("meditative-095", false)).toBe("???");
    expect(concealedText("meditative-095", true)).toBe(CONCEALED_TEXT_LOUD);
    expect(CONCEALED_TEXT_LOUD).toBe("!!!");
  });
});
