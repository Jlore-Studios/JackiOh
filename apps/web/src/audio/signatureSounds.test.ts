// R3399004: each of Classic+'s 35 signature cards plays a sound effect of its own (#3399).
import { describe, expect, it } from "vitest";

import { CATALOG } from "../wire/cards.ts";
import { CARD_AUDIO } from "./voiceData.ts";

const SIGNATURE_IDS = [
  "classicplus-012", "classicplus-012-1", "classicplus-012-2", "classicplus-012-3", "classicplus-012-4",
  "classicplus-012-5", "classicplus-012-6", "classicplus-012-7", "classicplus-012-8", "classicplus-013",
  "classicplus-019", "classicplus-019-1", "classicplus-019-2", "classicplus-019-3", "classicplus-019-4",
  "classicplus-019-5", "classicplus-027", "classicplus-029", "classicplus-035", "classicplus-037",
  "classicplus-042", "classicplus-042-1", "classicplus-043", "classicplus-046", "classicplus-046-1",
  "classicplus-047", "classicplus-048", "classicplus-065-4", "classicplus-065-5", "classicplus-073",
  "classicplus-073-1", "classicplus-074", "classicplus-075", "classicplus-075-1", "classicplus-078",
];

describe("Classic+'s signature sounds (R3399004)", () => {
  it("R3399004 each Classic+ signature's play or cast names an effect of its own", () => {
    expect(SIGNATURE_IDS).toHaveLength(35);
    const names = SIGNATURE_IDS.map((id) => {
      const hook = CATALOG[id]?.type === "Unit" ? "play" : "cast";
      const name = CARD_AUDIO.cards[id]?.[hook]?.effect;
      expect(name, `${id} ${hook} names an effect`).toBeTypeOf("string");
      return name ?? "";
    });
    expect(new Set(names).size, "35 distinct effect names").toBe(35);

    const signatures = new Set(SIGNATURE_IDS);
    const sharedWith = Object.entries(CARD_AUDIO.cards)
      .filter(([id]) => !signatures.has(id))
      .flatMap(([id, entry]) =>
        Object.entries(entry)
          .filter(([, hook]) => typeof hook === "object" && hook !== null && "effect" in hook && names.includes(String(hook.effect)))
          .map(([hookName]) => `${id} ${hookName}`),
      );
    expect(sharedWith, "other cards using a signature's effect").toEqual([]);

    const recipes = names.map((name) => {
      const effect = CARD_AUDIO.effects[name];
      expect(effect, `${name} is in the effects bank`).toBeDefined();
      return JSON.stringify([effect?.sfx, effect?.pitch, effect?.gain, effect?.params ?? {}]);
    });
    expect(new Set(recipes).size, "35 distinct (sfx, pitch, gain, params)").toBe(35);
  });
});
