// R915010, #920 (K3): the editor's draft moves between the builder and the code view without losing
// the card; a card edited in code beyond what its model writes is code-only, and says why.

import { describe, expect, it } from "vitest";

import type { CustomCard } from "@jackioh/shared";

import { emptyModel, writeScript, writeText, type BuilderModel } from "./builder.ts";
import { DRAFT_ID, builderRefusal, cardOf, draftOf, isDirty, newDraft, restartInBuilder, withFrame, withModel, withSource, withView } from "./draft.ts";

const STRIKE: BuilderModel = {
  ...emptyModel(),
  target: "an enemy unit",
  hooks: [{ trigger: "cry", steps: [{ verb: "damage", target: { kind: "declared" }, amount: 2 }] }],
};

describe("R915010 the builder and the code view are two views of one card", () => {
  it("R915010 a builder edit writes the script, the numbers and the text; the code view shows that script and the builder shows it back", () => {
    const built = withModel(newDraft(), STRIKE);
    expect(built.source).toBe(writeScript(STRIKE));
    expect(built.frame.params).toEqual([{ key: "dmg", base: 2, radiant: 4 }]);
    expect(built.frame.text).toBe("Cry: Deal {dmg} damage to an enemy unit.");

    const code = withView(built, "code");
    expect(code.view).toBe("code");
    expect(code.source).toBe(built.source);
    expect(builderRefusal(code)).toBeNull();
    const back = withView(code, "builder");
    expect(back.view).toBe("builder");
    expect(back.model).toEqual(STRIKE);
    expect(cardOf(back).builder).toEqual(STRIKE);
  });

  it("R915010 a script changed in code is code-only: the builder refuses the switch and says why, and undoing the change brings it back", () => {
    const built = withView(withModel(newDraft(), STRIKE), "code");
    const edited = withSource(built, built.source.replace("n.dmg", "n.dmg + 1"));
    expect(builderRefusal(edited)).toMatch(/changed in the code view/u);
    expect(withView(edited, "builder")).toBe(edited);
    expect(cardOf(edited).builder, "a code-only card saves no model").toBeUndefined();

    const undone = withSource(edited, built.source);
    expect(builderRefusal(undone)).toBeNull();
    expect(withView(undone, "builder").view).toBe("builder");
  });

  it("a card as the server writes it back, its empty lists dropped and its keys in serde's order, is not unsaved", () => {
    const draft = { ...newDraft(), id: "custom-0123456789abcdef0123456789abcdef", version: 1 };
    const { name, type, cost, rarity, attack, health, text } = draft.frame;
    const model = draft.model as unknown as Record<string, unknown>;
    const echoed: CustomCard = {
      id: "custom-0123456789abcdef0123456789abcdef",
      version: 1,
      // No `keywords: []` or `params: []`, which `CustomFrame` skips; the builder's keys reversed.
      frame: { text, health, attack, rarity, cost, type, name },
      source: draft.source,
      builder: Object.fromEntries(Object.entries(model).reverse()),
    };
    expect(isDirty(draft, echoed)).toBe(false);
    expect(isDirty(withFrame(draft, { ...draft.frame, cost: draft.frame.cost + 1 }), echoed)).toBe(true);
  });

  it("R915010 a card written in code opens in the code view, and the builder says it cannot show it", () => {
    const card: CustomCard = {
      id: "custom-0123456789abcdef0123456789abcdef",
      version: 3,
      frame: { name: "Hand made", type: "Spell", cost: 1, rarity: "Rare", text: "Draw a card.", params: [] },
      source: "return Card {\n  cry = function(c, n)\n    c:draw(1)\n  end,\n}\n",
    };
    const draft = draftOf(card);
    expect(draft.view).toBe("code");
    expect(draft.model).toBeNull();
    expect(builderRefusal(draft)).toMatch(/written in code/u);
    expect(isDirty(draft, card)).toBe(false);
    expect(isDirty(withSource(draft, `${card.source}\n`), card)).toBe(true);

    const restarted = restartInBuilder(draft);
    expect(restarted.view).toBe("builder");
    expect(restarted.source).toBe(writeScript(emptyModel()));
    expect(builderRefusal(restarted)).toBeNull();
  });

  it("R915010 a saved card's model opens it in the builder; an unsaved draft is checked under a well-formed id", () => {
    const built = withModel(newDraft(), STRIKE);
    const saved: CustomCard = { ...cardOf(built), id: "custom-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", version: 1 };
    expect(draftOf(saved).view).toBe("builder");
    expect(cardOf(built).id).toBe(DRAFT_ID);
  });

  it("R915010 the text follows the blocks until the player writes their own", () => {
    const built = withModel(newDraft(), STRIKE);
    const twice = withModel(built, { ...STRIKE, hooks: [{ trigger: "cry", steps: [{ verb: "draw", amount: 1 }] }] });
    expect(twice.frame.text).toBe("Cry: Draw {cards|card|cards}.");
    const own = withFrame(twice, { ...twice.frame, text: "My words {cards}." });
    const after = withModel(own, STRIKE);
    expect(after.frame.text).toBe("My words {cards}.");
    expect(after.frame.params).toEqual([{ key: "dmg", base: 2, radiant: 4 }]);
    expect(writeText(STRIKE)).not.toBe(after.frame.text);
  });

  it("R915010 a Spell has no stats or keywords, and a Unit gets them back", () => {
    const unit = withFrame(newDraft(), { ...newDraft().frame, keywords: ["Taunt"] });
    const spell = withFrame(unit, { ...unit.frame, type: "Spell" });
    expect(spell.frame.attack).toBeUndefined();
    expect(spell.frame.health).toBeUndefined();
    expect(spell.frame.keywords).toEqual([]);
    const again = withFrame(spell, { ...spell.frame, type: "Unit" });
    expect(again.frame.attack).toBe(1);
    expect(again.frame.health).toBe(1);
  });
});
