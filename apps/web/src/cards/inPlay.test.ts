// inPlay.ts: the words a face in play prints where play and print part ways (SPEC §10.10).
//
// `POWER_WORDS` is the client's copy of §8 #98's thirteen powers, keyed by the name the view gives a
// rolled power (R243). The engine holds the same clauses beside each power's effects, and the
// client cannot load the engine online, so this test is what keeps the two tables one table.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { subsystems } from "@jackioh/engine";

import {
  CONCEALED_TAG,
  CONCEALED_TEXT_LOUD,
  HEROIC_POWER_ID,
  POWER_WORDS,
  concealedInPlay,
  concealedText,
  powerText,
  powerTitle,
} from "./inPlay.ts";

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
    const text = powerText({ name: "recruit" }, false, "Indestructible");
    expect(text).toBe("Indestructible\nExpedition Map\nActivate: Spend (3): Recruit a permanent.");
    for (const other of ["Life Tap", "Deal 1 damage", "Rush Token", "Felinor Token", "Discover a Unit"]) {
      expect(text).not.toContain(other);
    }
  });

  it("prints the Radiant words and name on a radiant face, filling the card's numbers", () => {
    expect(powerText({ name: "discover" }, true, "Indestructible")).toBe(
      "Indestructible\nWitness Value\nActivate: Spend (2): Discover a Radiant Unit.",
    );
    expect(powerText({ name: "armor" }, true, "", { armor: 4 })).toBe("Tank Up\nActivate: Spend (1): Your hero gains 4 Armor, then this power refreshes.");
    expect(powerText({ name: "burn" }, false, "", { shot: 6 })).toBe("Steady Shot\nActivate: Spend (1): Deal 6 damage to the enemy hero.");
    expect(powerTitle({ name: "armor" }, false)).toBe("Armor Up");
  });

  it("leaves a name it does not know to the printed text", () => {
    expect(powerText({ name: "not-a-power" }, false, "Indestructible")).toBeNull();
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
