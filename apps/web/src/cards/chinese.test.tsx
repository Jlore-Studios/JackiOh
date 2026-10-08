// ME-CN: a card the view marks Chinese (`CardView.chinese`) is drawn in the Chinese tables' words in a
// match, every face of it (R1301), from the two sidecars bundled beside the catalog (R1303). The
// flag itself, and on which views it appears, is the engine's to prove (crates/engine, R1300); here
// every view is a fixture shaped as `viewFor` builds it.

import { cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, CardView, UnitView } from "@jackioh/shared";

import { lookupFromDefs, type CardInfo } from "../game/catalog.ts";
import { liveFace } from "../game/faces.ts";
import { card, fusedDef, unit } from "../test/fixtures.ts";
import { CardFace } from "./CardFace.tsx";
import { CHINESE, CHINESE_TERMS, chineseDef, chineseKeyword } from "./chinese.ts";
import { GLOSSARY } from "./glossary.ts";
import { Glossary } from "./inspect/Glossary.tsx";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel, type FaceModel } from "./model.ts";
import { markedText } from "./radiantDiff.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { namesOf } from "./refs.ts";
import { glossaryFor } from "./rules.ts";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

function zh(id: string): { name: string; base: string; radiant: string } {
  const entry = CHINESE[id];
  if (entry === undefined) throw new Error(`the Chinese table has no ${id}`);
  return entry;
}

function info(id: string, radiant = false): CardInfo {
  const found = lookup(id, radiant);
  if (found === undefined) throw new Error(`the lookup has no ${id}`);
  return found;
}

/** The face in play the board draws for `view`, as faces.ts builds it. */
function inPlay(view: CardView): FaceModel {
  return liveFace(info(view.defId, view.radiant), view);
}

function draw(face: FaceModel): HTMLElement {
  return render(<CardFace face={face} layout="full" />).container;
}

function withCatalog(node: ReactElement): ReactElement {
  return <CardDefsProvider defs={CATALOG}>{node}</CardDefsProvider>;
}

function text(root: Element, selector: string): string {
  return root.querySelector(selector)?.textContent ?? "";
}

describe("R1303 the frame's words", () => {
  it("R1303 the terms' glossary names exactly GLOSSARY's entries", () => {
    expect(Object.keys(CHINESE_TERMS.glossary).sort()).toEqual(Object.keys(GLOSSARY).sort());
    for (const [id, entry] of Object.entries(CHINESE_TERMS.glossary)) {
      expect(entry.label.trim(), id).not.toBe("");
      expect(entry.rule.trim(), id).not.toBe("");
    }
  });
});

describe("R1301 a Chinese card in a match", () => {
  it("R1301 a Chinese card in play draws its Chinese name, text, type line and tags", () => {
    // Core #2 Bigot: a Human Unit.
    const root = draw(inPlay(card({ defId: "core-002", cost: 3, chinese: true })));
    expect(text(root, ".card-name")).toBe(zh("core-002").name);
    expect(text(root, ".cf-text-base")).toBe(zh("core-002").base);
    expect(text(root, ".card-type")).toBe(CHINESE_TERMS.types.Unit);
    expect([...root.querySelectorAll(".cf-tag")].map((tag) => tag.textContent)).toEqual([CHINESE_TERMS.tags.Human]);
    // The English is nowhere on it; the machine-read attributes keep the catalog's words.
    expect(root.textContent).not.toContain(def("core-002").name);
    expect(root.textContent).not.toContain("Unit");
    expect(root.querySelector(".cf-tag")).toHaveAttribute("data-tag", "Human");
    // Its Radiant face is the Chinese Radiant text, its gold marks read off the two Chinese faces (R1302).
    const radiant = inPlay(card({ defId: "core-002", cost: 3, radiant: true, chinese: true }));
    expect(radiant.text.full).toBe(zh("core-002").radiant);
    expect(markedText(radiant.text.full, radiant.text.marks)).toEqual(["所有"]);
  });

  it("R1301 the same card outside a match is English", () => {
    // The collection's face prints the catalog's words, and so does the card in play without the flag.
    for (const face of [faceModel({ defId: "core-002", def: def("core-002"), radiant: false }), inPlay(card({ defId: "core-002", cost: 3 }))]) {
      const root = draw(face);
      expect(face.chinese).toBeUndefined();
      expect(text(root, ".card-name")).toBe(def("core-002").name);
      expect(text(root, ".cf-text-base")).toBe(def("core-002").base.text);
      expect(text(root, ".card-type")).toBe("Unit");
      expect(text(root, ".cf-tag")).toBe("Human");
      cleanup();
    }
  });

  it("R1301 its params are the view's numbers", () => {
    // Core #1 Big D-fender prints +{armor} Armor, 2 on its base face; the view says it has 5 now.
    const face = inPlay(card({ defId: "core-001", cost: 2, chinese: true, params: { armor: 5 } }));
    expect(face.text.full).toBe(zh("core-001").base.replace("{armor}", "5"));
    expect(face.text.full).not.toContain("{");
    // R386: the moved number is marked where the Chinese text prints it.
    const root = draw(face);
    expect(text(root, ".cf-tuned")).toBe("5");
  });

  it("R1301 its glossary entries are Chinese", () => {
    // Core #3 Right-house defender: Taunt, Divine Shield, Reborn, and on its Radiant face Death.
    const english = glossaryFor(inPlay(card({ defId: "core-003", cost: 4, radiant: true })));
    const chinese = glossaryFor(inPlay(card({ defId: "core-003", cost: 4, radiant: true, chinese: true })));
    // The same terms, found in the English text, each in the Chinese table's words.
    expect(chinese.map((entry) => entry.id)).toEqual(english.map((entry) => entry.id));
    expect(chinese.map((entry) => entry.id)).toEqual(["Taunt", "Divine Shield", "Reborn", "Death"]);
    for (const entry of chinese) {
      expect({ label: entry.label, rule: entry.rule }, entry.id).toEqual(CHINESE_TERMS.glossary[entry.id]);
    }
    const root = render(<Glossary entries={chinese} />).container;
    expect(text(root, '[data-glossary-term="Taunt"] .inspect-glossary-label')).toBe("嘲讽");
    expect(root.textContent).not.toContain(GLOSSARY.Taunt.rule);
  });

  it("R1301 its refs are marked by their Chinese names", () => {
    // Core #90 CN-Viral Injection names CN-Virus; its Radiant face names the Radiant one ("光辉CN病毒").
    for (const radiant of [false, true]) {
      const root = render(withCatalog(<CardFace face={inPlay(card({ defId: "core-090", cost: 2, radiant, chinese: true }))} layout="full" />)).container;
      const refs = [...root.querySelectorAll(".cf-ref")];
      expect(refs.map((ref) => [ref.getAttribute("data-ref"), ref.getAttribute("data-ref-face"), ref.textContent])).toEqual([
        ["core-090-1", radiant ? "radiant" : "base", zh("core-090-1").name],
      ]);
      cleanup();
    }
    // A full-width parenthesis is a parenthesis: Call to Chaos is named by its name before it too.
    expect(namesOf(chineseDef(def("core-095")))).toEqual([zh("core-095").name, "混沌召唤"]);
  });

  it("R1301 a preview label shows in Chinese with its live number", () => {
    // Core #91 Fed Fauci on the field: "+{mana} mana per Plague Counter", 1 as printed and 3 tuned.
    for (const [mana, params] of [
      [1, undefined],
      [3, { mana: 3 }],
    ] as const) {
      const view: UnitView = unit("p1", {
        defId: "core-091",
        attack: 1,
        maxHealth: 6,
        health: 4,
        counters: { plague: 2 },
        chinese: true,
        preview: [{ label: `+${String(mana)} mana per Plague Counter`, value: 2 * mana }],
        ...(params === undefined ? {} : { params }),
      });
      const face = inPlay(view);
      const label = `每有一个瘟疫指示物，便+${String(mana)}点法力值`;
      expect(face.values).toEqual([{ label, value: 2 * mana }]);
      const root = draw(face);
      const value = root.querySelector(".cf-value");
      expect(value).toHaveAttribute("data-label", label);
      // The value stands right after its label in the Chinese text.
      expect(text(root, ".cf-text-base")).toContain(`${label} {${String(2 * mana)}}`);
      cleanup();
    }
  });

  it("R1301 a fused Chinese card joins its ingredients' Chinese", () => {
    // R468: Core #11 Tempo Timmy fused with Core #89 Corpse Eater, as the engine joins the English.
    const fused = fusedDef([def("core-011"), def("core-089")]);
    const base = faceModel({ defId: fused.id, def: fused, radiant: false, chinese: true, inPlay: {} });
    expect(base.name).toBe(`${zh("core-011").name} + ${zh("core-089").name}`);
    expect(base.text.full).toBe(`${zh("core-011").base}\n${zh("core-089").base}`);
    const radiant = faceModel({ defId: fused.id, def: fused, radiant: true, chinese: true, inPlay: {} });
    expect(radiant.text.full).toBe(`${zh("core-011").radiant}\n${zh("core-089").radiant}`);
    // R102, R1302: each Chinese Radiant line is marked against its own Chinese base line.
    expect(markedText(radiant.text.full, radiant.text.marks)).toEqual(["冲锋", "圣盾", "两倍"]);
    // R469: an ingredient fused on its Radiant face puts that face into the base form too.
    const forced = { ...fused, ingredients: [{ defId: "core-011", radiant: true as const }, { defId: "core-089" }] };
    expect(chineseDef(forced).base.text).toBe(`${zh("core-011").radiant}\n${zh("core-089").base}`);
    // The English of the same fusion is what the glossary reads.
    expect(base.englishText).toBe(fused.base.text);
  });

  it("R1301 keywords read in Chinese", () => {
    expect(chineseKeyword({ kind: "Armor", n: 7 })).toBe("护甲7");
    expect(chineseKeyword({ kind: "Taunt" })).toBe("嘲讽");
    // Core #2 Bigot on the field, which has gained Taunt and Armor 3 since it was printed.
    const view = unit("p1", { defId: "core-002", attack: 2, maxHealth: 2, health: 2, keywords: [{ kind: "Taunt" }, { kind: "Armor", n: 3 }], chinese: true });
    const face = inPlay(view);
    const root = draw(face);
    expect(text(root, ".cf-text-gained")).toBe("嘲讽，护甲3");
    cleanup();
    const minion = render(<MinionFace face={face} unit={view} />).container;
    expect([...minion.querySelectorAll(".keyword[data-keyword]")].map((chip) => chip.getAttribute("title"))).toEqual(["嘲讽", "护甲3"]);
    // Its name plate is Chinese, and its art keeps the English name's motif.
    expect(text(minion, ".card-name")).toBe(zh("core-002").name);
    expect(face.englishName).toBe(def("core-002").name);
  });
});
