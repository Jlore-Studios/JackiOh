// Polish 6, slice B: the tall card face in jsdom (docs/polish/6-cards.md, "CardFace DOM",
// behaviours B8's gem, B10's `RulesText`, B12–B14, and the jsdom half of B15).
//
// jsdom has no layout, so nothing here measures fit. What it can check is the DOM contract: which
// elements exist, what they carry, and that `useFitText` leaves an element with no layout alone.
// The pixel half of B15 is e2e/cypress/component/card-faces.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, render } from "@testing-library/react";
import { useRef, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID, TRIBAL_TAGS } from "@jackioh/engine/config";
import { fillParams, type CardDef, type CardType, type Rarity, type Tag } from "@jackioh/shared";

import { CardFace } from "./CardFace.tsx";
import { CHINESE_TERMS } from "./chinese.ts";
import { RulesText } from "./RulesText.tsx";
import {
  FACE_ASPECT,
  FACE_TEXT_MIN_HEIGHT_PX,
  FIT_MIN,
  FIT_STEPS,
  NAME_TIER_MAX,
  TEXT_TIER_MAX,
  TIER_SCALE,
} from "./constants.ts";
import { flushFits, nameTier, textTier, useFitText, type LengthTier } from "./fit.ts";
import { faceModel, type FaceSource } from "./model.ts";
import { termsIn } from "./rules.ts";
import { SET_MARK_MIN_FACE_PX, setMarkOf } from "./setMark.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";

afterEach(() => {
  cleanup();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
  vi.unstubAllGlobals();
});

/* -------------------------------------------------------------------------------------- helpers */

const DEFS: readonly CardDef[] = Object.values(CATALOG);
const XHTML = "http://www.w3.org/1999/xhtml";
const UNKNOWN_ID = "core-999";
/**
 * The sweeps below render both faces of all 317 catalog entries (v0.2.0's Core, Classic and
 * Classic+), which outruns vitest's 5 s default on a loaded machine.
 */
const CATALOG_SWEEP_TIMEOUT_MS = 60_000;

/** vitest stubs CSS imports, so the set mark's stylesheet is read as text. */
const setMarkCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "setmark.css"), "utf8");

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

function renderFace(source: FaceSource, layout: "full" | "compact" = "full"): HTMLElement {
  const { container } = render(<CardFace face={faceModel(source)} layout={layout} />);
  flushFits();
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`CardFace rendered no .cf for ${source.defId}`);
  expect(container.firstElementChild).toBe(cf);
  return cf;
}

function catalogFace(id: string, radiant = false, layout: "full" | "compact" = "full", extra: Partial<FaceSource> = {}): HTMLElement {
  return renderFace({ defId: id, def: def(id), radiant, ...extra }, layout);
}

function one(scope: Element, selector: string): HTMLElement {
  const found = scope.querySelector<HTMLElement>(selector);
  if (found === null) throw new Error(`no ${selector} inside .${scope.getAttribute("class") ?? ""}`);
  return found;
}

/** The gem's own number, without the small embiggen price beside it. */
function gemText(gem: Element): string {
  const copy = gem.cloneNode(true) as Element;
  for (const alt of copy.querySelectorAll(".cf-cost-alt")) alt.remove();
  return (copy.textContent ?? "").trim();
}

/**
 * Everything B12 and the "Forbidden inside a face" list rule out, as a list of findings. A face
 * must sit inside a `<button>`, so its HTML is span, strong and img only. The Surface also gives
 * the icons as "aria-hidden SVG, no <text>", so SVG content is accepted only inside an
 * `svg[aria-hidden="true"]` and never as `<text>` or `<title>`.
 */
function faceProblems(cf: Element): string[] {
  const problems: string[] = [];
  for (const el of [cf, ...cf.querySelectorAll("*")]) {
    const tag = el.tagName.toLowerCase();
    const label = `<${tag} class="${el.getAttribute("class") ?? ""}">`;
    if (el.namespaceURI === XHTML) {
      if (!["span", "strong", "img"].includes(tag)) problems.push(`${label} is not span, strong or img`);
    } else {
      const svg = el.closest("svg");
      if (svg === null || svg.getAttribute("aria-hidden") !== "true") problems.push(`${label} is outside an aria-hidden svg`);
      if (tag === "text" || tag === "title") problems.push(`${label} is SVG text`);
    }
    for (const attribute of ["data-attack", "data-health", "data-keyword", "data-testid"]) {
      if (el.hasAttribute(attribute)) problems.push(`${label} carries ${attribute}`);
    }
    if (el.classList.contains("card")) problems.push(`${label} carries the card class token`);
    const style = (el as HTMLElement).style as CSSStyleDeclaration | undefined;
    if (style !== undefined && (style.width !== "" || style.minWidth !== "")) {
      problems.push(`${label} has an inline width or min-width`);
    }
  }
  return problems;
}

const RARITY_CARD: readonly (readonly [Rarity, string])[] = [
  ["Common", "core-002"],
  ["Rare", "core-007"],
  ["Epic", "core-018"],
  ["Legendary", "core-052"],
  ["Mythic", "core-096"],
];

const TYPE_CARD: readonly (readonly [CardType, string, string])[] = [
  ["Unit", "core-020", "portrait"],
  ["Spell", "core-005", "window"],
  ["Field Spell", "core-006", "arch"],
  ["Trap", "core-041", "notched"],
  ["Field Trap", "core-018", "notched"],
];

/* ----------------------------------------------------------------------------------------- B12 */

describe("B12: the full face's DOM", () => {
  it(
    "B12 every catalog card, both faces: only span, strong and img, and nothing a face may not carry",
    () => {
      for (const card of DEFS) {
        for (const radiant of [false, true]) {
          const cf = catalogFace(card.id, radiant);
          expect(faceProblems(cf), `${card.id} radiant=${String(radiant)}`).toEqual([]);
          cleanup();
        }
      }
    },
    CATALOG_SWEEP_TIMEOUT_MS,
  );

  it("B12 cost gem, art frame holding the art, the name as one text node, and the type line", () => {
    const cf = catalogFace("core-020");
    expect(cf.getAttribute("data-layout")).toBe("full");
    one(cf, ".cf-scale .cost-gem");
    const art = one(cf, ".cf-art-frame > .cf-art");
    expect(art.getAttribute("aria-hidden")).toBe("true");

    const name = one(cf, ".card-name");
    expect(name.childNodes).toHaveLength(1);
    expect(name.firstChild?.nodeType).toBe(Node.TEXT_NODE);
    expect(name.textContent).toBe("Pointmaster");

    expect(one(cf, ".card-type").textContent).toBe("Unit");
  });

  it("B12 the art frame's shape follows the type: portrait, window, arch, notched, notched", () => {
    for (const [type, id, shape] of TYPE_CARD) {
      const cf = catalogFace(id);
      const art = one(cf, ".cf-art-frame > .cf-art");
      expect(art.classList.contains(`cf-art--${shape}`), `${type} (${id})`).toBe(true);
      expect(one(cf, ".card-type").textContent, id).toBe(type);
      cleanup();
    }
  });

  it("B12 .cf-gem carries the rarity for Common, Rare, Epic, Legendary and Mythic", () => {
    for (const [rarity, id] of RARITY_CARD) {
      const cf = catalogFace(id);
      const gems = cf.querySelectorAll(".cf-gem");
      expect(gems, id).toHaveLength(1);
      expect(gems[0]?.getAttribute("data-rarity"), id).toBe(rarity);
      cleanup();
    }
  });

  it("B12 no .cf-gem on a Token card or on an unknown card", () => {
    for (const id of ["core-t-rush", "core-051-1", "core-093-1", "core-095-1"]) {
      expect(catalogFace(id).querySelector(".cf-gem"), id).toBeNull();
      cleanup();
    }
    const unknown = renderFace({ defId: UNKNOWN_ID, name: "Nobody", type: "Spell", radiant: false });
    expect(unknown.querySelector(".cf-gem")).toBeNull();
  });

  it("B12 one .cf-tag[data-tag] per tag, Token included, in the def's order", () => {
    const tags = catalogFace("core-t-felinor").querySelectorAll(".cf-tags > .cf-tag");
    expect([...tags].map((tag) => tag.getAttribute("data-tag"))).toEqual(["Felinor", "Token"]);
    cleanup();

    const human = catalogFace("core-002").querySelectorAll(".cf-tag");
    expect([...human].map((tag) => tag.getAttribute("data-tag"))).toEqual(["Human"]);
  });

  it("B12 a card with no tags renders no .cf-tag", () => {
    expect(catalogFace("core-005").querySelectorAll(".cf-tag")).toHaveLength(0);
  });

  it("B12 .card-text is present and empty when the card has no text", () => {
    const text = one(catalogFace("core-t-felinor"), ".card-text");
    expect(text.textContent).toBe("");
    expect(text.querySelector(".cf-term")).toBeNull();
  });

  it("B12 a unit shows .cf-atk[data-face-attack][data-tone] and .cf-hp[data-face-health][data-tone]", () => {
    const cf = catalogFace("core-020");
    const atk = one(cf, ".cf-stats .cf-atk");
    const hp = one(cf, ".cf-stats .cf-hp");
    expect(atk.getAttribute("data-face-attack")).toBe("7");
    expect(atk.getAttribute("data-tone")).toBe("base");
    expect(atk.textContent?.trim()).toBe("7");
    expect(hp.getAttribute("data-face-health")).toBe("1");
    expect(hp.getAttribute("data-tone")).toBe("base");
    expect(hp.textContent?.trim()).toBe("1");
  });

  it("B12 a radiant unit's stats come from its radiant face, and live stats carry their tones", () => {
    const radiant = catalogFace("core-011", true);
    expect(one(radiant, ".cf-atk").getAttribute("data-face-attack")).toBe("6");
    expect(one(radiant, ".cf-hp").getAttribute("data-face-health")).toBe("6");
    cleanup();

    const hurt = catalogFace("core-013", false, "full", {
      live: { attack: 9, health: 4, maxHealth: 10, keywords: [] },
    });
    expect(one(hurt, ".cf-atk").getAttribute("data-face-attack")).toBe("9");
    expect(one(hurt, ".cf-atk").getAttribute("data-tone")).toBe("buffed");
    expect(one(hurt, ".cf-hp").getAttribute("data-face-health")).toBe("4");
    expect(one(hurt, ".cf-hp").getAttribute("data-tone")).toBe("damaged");
  });

  it("B12 Spell, Field Spell, Trap and Field Trap faces have no stats", () => {
    for (const [type, id] of TYPE_CARD) {
      if (type === "Unit") continue;
      const cf = catalogFace(id);
      expect(cf.querySelector(".cf-stats"), id).toBeNull();
      expect(cf.querySelector(".cf-atk"), id).toBeNull();
      expect(cf.querySelector(".cf-hp"), id).toBeNull();
      cleanup();
    }
  });

  it("B12 a unit with printed keywords still carries no data-keyword, data-attack or data-health", () => {
    const cf = catalogFace("core-056", false, "full", {
      liveCost: 2,
      live: { attack: 3, health: 1, maxHealth: 2, keywords: [{ kind: "Taunt" }, { kind: "Armor", n: 2 }] },
    });
    expect(cf.querySelectorAll("[data-keyword], [data-attack], [data-health], [data-testid]")).toHaveLength(0);
  });
});

/* ------------------------------------------------------------------------- B18 (compact layout) */

describe("B18: the compact layout a face-up backrow card uses", () => {
  it("B18 compact renders cost, art, name, rarity gem and type line, but no text, tags or stats", () => {
    const cf = catalogFace("core-018", false, "compact");
    expect(cf.getAttribute("data-layout")).toBe("compact");
    one(cf, ".cost-gem");
    one(cf, ".cf-art");
    expect(one(cf, ".card-name").textContent).toBe("Bread and Butter");
    expect(one(cf, ".cf-gem").getAttribute("data-rarity")).toBe("Epic");
    expect(one(cf, ".card-type").textContent).toBe("Field Trap");
    expect(cf.querySelector(".card-text")).toBeNull();
    expect(cf.querySelector(".cf-tags")).toBeNull();
    expect(cf.querySelector(".cf-stats")).toBeNull();
  });

  it("B18 a compact unit face has no stats either", () => {
    const cf = catalogFace("core-020", false, "compact");
    expect(cf.querySelector(".cf-stats")).toBeNull();
    expect(cf.querySelector(".card-text")).toBeNull();
    expect(faceProblems(cf)).toEqual([]);
  });
});

/* ----------------------------------------------------------------------------------- B8 (DOM) */

describe("B8: the cost gem renders the model's cost", () => {
  it("B8 .cost-gem shows text, with data-cost set to value and data-tone to the tone", () => {
    const x = one(catalogFace("core-024", false, "full", { liveCost: 3 }), ".cost-gem");
    expect(gemText(x)).toBe("X");
    expect(x.getAttribute("data-cost")).toBe("3");
    expect(x.getAttribute("data-tone")).toBe("base");
    cleanup();

    const down = one(catalogFace("core-004", false, "full", { liveCost: 0 }), ".cost-gem");
    expect(gemText(down)).toBe("0");
    expect(down.getAttribute("data-cost")).toBe("0");
    expect(down.getAttribute("data-tone")).toBe("down");
    cleanup();

    const up = one(catalogFace("core-004", false, "full", { liveCost: 3 }), ".cost-gem");
    expect(gemText(up)).toBe("3");
    expect(up.getAttribute("data-tone")).toBe("up");
  });

  it("B8 an embiggen card's gem shows its base price and a .cf-cost-alt with the embiggen price", () => {
    const gem = one(catalogFace("core-046"), ".cost-gem");
    expect(gemText(gem)).toBe("2");
    expect(gem.getAttribute("data-cost")).toBe("2");
    expect(one(gem, ".cf-cost-alt").textContent).toContain("4");
  });

  it("B8 an embiggen card's gem prints the view's live cost: a discounted 1, or the 4 it was paid on the field", () => {
    const discounted = one(catalogFace("core-084", false, "full", { liveCost: 1 }), ".cost-gem");
    expect(gemText(discounted)).toBe("1");
    expect(discounted.getAttribute("data-tone")).toBe("down");
    expect(discounted.querySelector(".cf-cost-alt")).toBeNull();
    cleanup();
    const paid = one(catalogFace("core-059", false, "compact", { liveCost: 4 }), ".cost-gem");
    expect(gemText(paid)).toBe("4");
    expect(paid.getAttribute("data-tone")).toBe("base");
    expect(paid.querySelector(".cf-cost-alt")).toBeNull();
  });

  it("B8 a card that is not embiggen has no .cf-cost-alt", () => {
    for (const id of ["core-004", "core-024", "core-100"]) {
      expect(catalogFace(id).querySelector(".cf-cost-alt"), id).toBeNull();
      cleanup();
    }
  });
});

/* ----------------------------------------------------------------------- B10 (RulesText in DOM) */

describe("B10: RulesText marks terms in bold", () => {
  it("B10 each term renders as strong.cf-term[data-term], and the text reads back unchanged", () => {
    const cry = catalogFace("core-002");
    const term = one(cry, ".card-text strong.cf-term");
    expect(term.getAttribute("data-term")).toBe("Cry");
    expect(term.textContent).toBe("Cry:");
    expect(one(cry, ".cf-text-base").textContent).toBe("Cry: Destroy an enemy non-Human Unit.");
    cleanup();

    const start = one(catalogFace("core-040"), '.card-text strong.cf-term[data-term="Start of turn"]');
    expect(start.textContent).toBe("Start of turn:");
    cleanup();

    const golem = catalogFace("core-055");
    const terms = [...golem.querySelectorAll(".card-text strong.cf-term")];
    expect(terms.map((strong) => strong.textContent)).toEqual(["Taunt", "Tribute 3"]);
    expect(terms.map((strong) => strong.getAttribute("data-term"))).toEqual(["Taunt", "Tribute"]);
  });

  it("B10 every catalog face, base and radiant, Classic and Classic+ too, draws each term its text names in bold, its keywords among them (#85)", () => {
    for (const card of DEFS) {
      for (const radiant of [false, true]) {
        const source: FaceSource = { defId: card.id, def: card, radiant };
        const label = `${card.id} ${radiant ? "radiant" : "base"}`;
        const drawn = new Set([...renderFace(source).querySelectorAll(".card-text strong.cf-term")].map((term) => term.getAttribute("data-term")));
        expect([...drawn], label).toEqual(termsIn(faceModel(source).text.full));
        for (const keyword of (radiant ? card.radiant : card.base).keywords) expect(drawn.has(keyword.kind), `${label} ${keyword.kind}`).toBe(true);
        cleanup();
      }
    }
  }, CATALOG_SWEEP_TIMEOUT_MS);

  it("B10 patch v0.2.0's words are bold terms (Activate ♾️, Brittle, Animated, Upgrade, Degrade), and so is Classic #65's End of turn (#85)", () => {
    const termsOf = (id: string): string[] =>
      [...catalogFace(id).querySelectorAll(".card-text strong.cf-term")].map((term) => `${term.getAttribute("data-term") ?? ""}=${term.textContent ?? ""}`);
    expect(termsOf("classic-078")).toContain("Activate=Activate ♾️:");
    cleanup();
    expect(termsOf("classicplus-074")).toContain("Brittle=Brittle");
    cleanup();
    expect(termsOf("classic-038")[0]).toBe("Animated=Animated");
    cleanup();
    const tuners = termsOf("classicplus-070").map((term) => term.split("=")[0]);
    expect(tuners).toContain("Upgrade");
    expect(tuners).toContain("Degrade");
    cleanup();
    expect(termsOf("classic-065")[0]).toBe("End of turn=End of turn:");
  });

  it("B10 a term keeps the rules box's ink in a tavern screen, whose own bold words are pale gold (#85)", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const rules = (file: string): { selector: string; body: string }[] =>
      [...readFileSync(join(here, file), "utf8").replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
        selector: (match[1] ?? "").trim(),
        body: match[2] ?? "",
      }));
    /** [ids, classes, types] of a selector made of ids, classes and type names, as both of these are. */
    const specificity = (selector: string): number[] => [
      selector.match(/#[\w-]+/g)?.length ?? 0,
      selector.match(/\.[\w-]+/g)?.length ?? 0,
      selector.split(/[\s>+~]+/).filter((part) => /^[a-z]/.test(part)).length,
    ];
    const outranks = (a: number[], b: number[]): boolean => {
      const at = a.findIndex((value, index) => value !== b[index]);
      return at >= 0 && (a[at] ?? 0) > (b[at] ?? 0);
    };

    const term = rules("cards.css").filter((rule) => /^\.cf .*\.cf-term$/.test(rule.selector) && !rule.selector.includes(".cf-mark"));
    expect(term, "the face's one rule for its terms").toHaveLength(1);
    expect(term[0]?.body).toMatch(/font-weight:\s*800/);
    expect(term[0]?.body).toMatch(/color:\s*inherit/);

    const tavern = rules("../auth/tavern.css").find((rule) => rule.selector === ".tavern.app-shell strong");
    expect(tavern?.body).toMatch(/color:\s*var\(--tavern-gold-pale\)/);
    expect(outranks(specificity(term[0]?.selector ?? ""), specificity(tavern?.selector ?? ""))).toBe(true);
  });

  it("R280 two values sharing one label group in one helper block after it, not one mid-text and one at the end", () => {
    const { container } = render(
      <RulesText
        text="Deal 4 damage. Your two largest piles add their effects."
        values={[
          { label: "Your two largest piles", value: 5, display: "Deck" },
          { label: "Your two largest piles", value: 3, display: "Graveyard" },
        ]}
      />,
    );
    const chips = [...container.querySelectorAll(".cf-value")];
    expect(chips.map((chip) => chip.getAttribute("data-value"))).toEqual(["Deck", "Graveyard"]);
    // Both chips sit after the label and before the words that follow it — one block, mid-text.
    expect(container.textContent).toBe("Deal 4 damage. Your two largest piles {Deck} {Graveyard} add their effects.");
    cleanup();
  });

  it("B10 lower-case or glued words are not bolded", () => {
    const plain: CardDef = {
      ...def("core-005"),
      base: { keywords: [], text: "taunt the Locked Rushing units" },
      radiant: { keywords: [], text: "taunt the Locked Rushing units" },
    };
    const cf = renderFace({ defId: plain.id, def: plain, radiant: false });
    expect(cf.querySelectorAll("strong")).toHaveLength(0);
    expect(one(cf, ".card-text").textContent).toBe("taunt the Locked Rushing units");
  });

  it(
    "B10 every catalog text reads back unchanged through RulesText, marks and all",
    () => {
      for (const card of DEFS) {
        for (const radiant of [false, true]) {
          const text = faceModel({ defId: card.id, def: card, radiant }).text;
          const cf = catalogFace(card.id, radiant);
          expect(one(cf, ".cf-text-base").textContent, card.id).toBe(text.full);
          // The face's catalog text, its `{key}` numbers filled in with its printed values (B3.4 rule 5).
          expect(text.full, card.id).toBe(fillParams(card, radiant ? "radiant" : "base"));
          cleanup();
        }
      }
    },
    CATALOG_SWEEP_TIMEOUT_MS,
  );
});

/* ----------------------------------------------------------------------------------------- B13 */

describe("B13: .cf attributes, crest and foil", () => {
  it("B13 .cf carries data-layout, data-card-type, data-rarity, data-name-tier, data-text-tier and data-foil", () => {
    const cf = catalogFace("core-052");
    expect(cf.getAttribute("data-layout")).toBe("full");
    expect(cf.getAttribute("data-card-type")).toBe("Unit");
    expect(cf.getAttribute("data-rarity")).toBe("Legendary");
    expect(cf.getAttribute("data-name-tier")).toBe(nameTier("Silly Silas"));
    expect(cf.getAttribute("data-text-tier")).toBe("l");
    expect(cf.getAttribute("data-foil")).toBe("none");
    expect(one(cf, ".cf-scale").parentElement).toBe(cf);
  });

  it("B13 data-card-type is the type for each card type", () => {
    for (const [type, id] of TYPE_CARD) {
      expect(catalogFace(id).getAttribute("data-card-type"), id).toBe(type);
      cleanup();
    }
  });

  it("B13 data-rarity is omitted when the rarity is unknown", () => {
    const cf = renderFace({ defId: UNKNOWN_ID, name: "Nobody", type: "Unit", radiant: false });
    expect(cf.hasAttribute("data-rarity")).toBe(false);
    expect(cf.getAttribute("data-card-type")).toBe("Unit");
  });

  it("B13 .cf-crest renders for Legendary and Mythic only", () => {
    for (const [rarity, id] of RARITY_CARD) {
      const crested = rarity === "Legendary" || rarity === "Mythic";
      expect(catalogFace(id).querySelectorAll(".cf-crest"), `${rarity} ${id}`).toHaveLength(crested ? 1 : 0);
      cleanup();
    }
    expect(catalogFace("core-t-rush").querySelector(".cf-crest")).toBeNull();
    cleanup();
    expect(renderFace({ defId: UNKNOWN_ID, radiant: false }).querySelector(".cf-crest")).toBeNull();
  });

  it("B13 data-foil is animated for Mythic and radiant faces while animatedFoil is on", () => {
    writeCardSettings({ animatedFoil: true });
    expect(catalogFace("core-096").getAttribute("data-foil")).toBe("animated");
    cleanup();
    expect(catalogFace("core-002", true).getAttribute("data-foil")).toBe("animated");
    cleanup();
    expect(catalogFace("core-t-rush", true).getAttribute("data-foil")).toBe("animated");
  });

  it("B13 data-foil is static for Mythic and radiant faces while animatedFoil is off", () => {
    writeCardSettings({ animatedFoil: false });
    expect(catalogFace("core-096").getAttribute("data-foil")).toBe("static");
    cleanup();
    expect(catalogFace("core-002", true).getAttribute("data-foil")).toBe("static");
  });

  it("B13 data-foil is none for base Common, Rare, Epic, Legendary and Token faces, whatever the setting", () => {
    for (const animatedFoil of [true, false]) {
      writeCardSettings({ animatedFoil });
      for (const id of ["core-002", "core-007", "core-018", "core-052", "core-t-rush"]) {
        expect(catalogFace(id).getAttribute("data-foil"), `${id} animatedFoil=${String(animatedFoil)}`).toBe("none");
        cleanup();
      }
    }
  });
});

/* ----------------------------------------------------------------------------------------- B14 */

describe("B14: the radiant face", () => {
  it("B14 a radiant face sets data-radiant-face and draws radiant art", () => {
    const cf = catalogFace("core-011", true);
    expect(cf.getAttribute("data-radiant-face")).toBe("true");
    expect(one(cf, ".cf-art-frame .cf-art").getAttribute("data-art-variant")).toBe("radiant");
  });

  it("B14 a base face has no data-radiant-face and draws base art", () => {
    const cf = catalogFace("core-011", false);
    expect(cf.hasAttribute("data-radiant-face")).toBe(false);
    expect(one(cf, ".cf-art-frame .cf-art").getAttribute("data-art-variant")).toBe("base");
  });

  it("R277 a radiant face prints its whole text once and marks in gold what the base face lacks", () => {
    const cf = catalogFace("core-004", true);
    const text = one(cf, ".card-text");
    expect(one(text, ".cf-text-base").textContent).toBe("Cry: Flip 7 coins. Gain +2 attack per heads and +2 health per tails. Flip a coin. If heads, gain Divine Shield; if tails, gain Rush.");
    expect(text.querySelector(".cf-text-radiant")).toBeNull();
    expect([...text.querySelectorAll(".cf-mark")].map((mark) => mark.textContent)).toEqual(["7", "+2", "+2"]);
  });

  it("B14 a radiant face prints its own keyword line, never the base keywords it replaced (core-056, core-025)", () => {
    const jilliax = one(catalogFace("core-056", true), ".card-text");
    expect(jilliax.textContent).toBe("Rush, Taunt, Lifesteal, Divine Shield, Reborn");
    expect([...jilliax.querySelectorAll("strong.cf-term")].map((term) => term.getAttribute("data-term"))).toEqual([
      "Rush",
      "Taunt",
      "Lifesteal",
      "Divine Shield",
      "Reborn",
    ]);
    cleanup();
    expect(one(catalogFace("core-025", true), ".card-text").textContent).toBe("Armor 7, Reborn");
  });

  it("B14 a clause the radiant cell restates is printed once, in its radiant form (core-002, core-046)", () => {
    const bigot = one(catalogFace("core-002", true), ".card-text");
    expect(bigot.textContent).toBe("Cry: Destroy all enemy non-Human Units.");
    expect(bigot.querySelectorAll('strong.cf-term[data-term="Cry"]')).toHaveLength(1);
    cleanup();
    expect(one(catalogFace("core-046", true), ".card-text").textContent).toBe("Aura: Enemy Units have −2/−2.\nPaid (4): −4/−4 instead.");
  });

  it("R277 R276 the five cards that had no radiant form print a radiant face that differs, marked (core-038, core-080, core-093-1, core-095-1, core-096)", () => {
    for (const id of ["core-038", "core-080", "core-093-1", "core-095-1", "core-096"]) {
      const cf = catalogFace(id, true);
      expect(cf.getAttribute("data-radiant-face"), id).toBe("true");
      expect(one(cf, ".cf-text-base").textContent, id).toBe(fillParams(def(id), "radiant"));
      expect(cf.querySelectorAll(".cf-mark").length, id).toBeGreaterThan(0);
      cleanup();
    }
  });

  it("R277 a base face never marks anything", () => {
    for (const id of ["core-002", "core-011", "core-051", "core-093"]) {
      expect(catalogFace(id, false).querySelector(".cf-mark"), id).toBeNull();
      cleanup();
    }
  });

  it("R277 a printed radiant unit marks the stats it raised; a radiant face that kept them marks none", () => {
    const felinors = catalogFace("core-012", true);
    expect(one(felinors, ".cf-atk").getAttribute("data-grew")).toBe("true");
    expect(one(felinors, ".cf-hp").getAttribute("data-grew")).toBe("true");
    cleanup();
    const base = catalogFace("core-012", false);
    expect(one(base, ".cf-atk").hasAttribute("data-grew")).toBe(false);
    cleanup();
    // Big D-fender's 0 attack stays 0 (R275), so only its health is marked.
    const fender = catalogFace("core-001", true);
    expect(one(fender, ".cf-atk").hasAttribute("data-grew")).toBe(false);
    expect(one(fender, ".cf-hp").getAttribute("data-grew")).toBe("true");
  });
});

/* ------------------------------------------------------------------------------ B15 (jsdom half) */

function stringOf(length: number): string {
  return "x".repeat(length);
}

describe("B15: length tiers, and useFitText without layout", () => {
  it("B15 the tier constants are the Surface's numbers", () => {
    expect(NAME_TIER_MAX).toEqual({ s: 12, m: 18, l: 24, xl: 30 });
    expect(TEXT_TIER_MAX).toEqual({ s: 40, m: 90, l: 160, xl: 260 });
    expect(TIER_SCALE).toEqual({ s: 1, m: 0.92, l: 0.82, xl: 0.72, xxl: 0.62 });
    expect(FIT_MIN).toBe(0.55);
    expect(FIT_STEPS).toBe(6);
    expect(FACE_ASPECT).toBe(5 / 7);
    expect(FACE_TEXT_MIN_HEIGHT_PX).toBe(150);
  });

  it("B15 nameTier: ≤12 s, ≤18 m, ≤24 l, ≤30 xl, else xxl, at each boundary", () => {
    const cases: readonly (readonly [number, LengthTier])[] = [
      [1, "s"],
      [12, "s"],
      [13, "m"],
      [18, "m"],
      [19, "l"],
      [24, "l"],
      [25, "xl"],
      [30, "xl"],
      [31, "xxl"],
      [200, "xxl"],
    ];
    for (const [length, tier] of cases) {
      expect(nameTier(stringOf(length)), `${length} characters`).toBe(tier);
    }
  });

  it("B15 textTier: ≤40 s, ≤90 m, ≤160 l, ≤260 xl, else xxl, at each boundary", () => {
    const cases: readonly (readonly [number, LengthTier])[] = [
      [1, "s"],
      [40, "s"],
      [41, "m"],
      [90, "m"],
      [91, "l"],
      [160, "l"],
      [161, "xl"],
      [260, "xl"],
      [261, "xxl"],
      [1000, "xxl"],
    ];
    for (const [length, tier] of cases) {
      expect(textTier(stringOf(length)), `${length} characters`).toBe(tier);
    }
  });

  it("B15 empty strings are the smallest tier", () => {
    expect(nameTier("")).toBe("s");
    expect(textTier("")).toBe("s");
  });

  it("B15 .cf's data-name-tier is nameTier(name) for every catalog card", () => {
    for (const card of DEFS) {
      expect(catalogFace(card.id).getAttribute("data-name-tier"), card.id).toBe(nameTier(card.name));
      cleanup();
    }
  });

  it("B15 .cf's data-text-tier counts the text the face prints", () => {
    const cases: readonly (readonly [string, boolean, LengthTier])[] = [
      ["core-008", false, "s"],
      ["core-008", true, "s"],
      ["core-t-felinor", true, "s"],
      ["core-052", false, "l"],
      ["core-052", true, "xl"],
      ["core-051", false, "l"],
      ["core-051", true, "xl"],
      ["core-093", false, "xxl"],
      ["core-095", true, "xxl"],
    ];
    for (const [id, radiant, tier] of cases) {
      expect(catalogFace(id, radiant).getAttribute("data-text-tier"), `${id} radiant=${String(radiant)}`).toBe(tier);
      cleanup();
    }
  });

  it("B15 useFitText is a no-op on an element with no layout: no --cf-fit, no data-clamped, on content change too", () => {
    const { container, rerender } = render(<FitProbe content="A long line of rules text that would overflow" />);
    flushFits();
    const probe = one(container, "[data-probe]");
    expect(probe.style.getPropertyValue("--cf-fit")).toBe("");
    expect(probe.hasAttribute("data-clamped")).toBe(false);

    rerender(<FitProbe content={stringOf(400)} />);
    flushFits();
    expect(probe.style.getPropertyValue("--cf-fit")).toBe("");
    expect(probe.hasAttribute("data-clamped")).toBe(false);
  });

  it("B15 useFitText stays a no-op without layout even when a ResizeObserver exists and fires", () => {
    const callbacks: ResizeObserverCallback[] = [];
    const observed: Element[] = [];
    class StubResizeObserver {
      constructor(callback: ResizeObserverCallback) {
        callbacks.push(callback);
      }
      observe(target: Element): void {
        observed.push(target);
      }
      unobserve(): void {}
      disconnect(): void {}
    }
    vi.stubGlobal("ResizeObserver", StubResizeObserver);

    const { container } = render(<FitProbe content={stringOf(300)} />);
    flushFits();
    const probe = one(container, "[data-probe]");
    act(() => {
      for (const callback of callbacks) {
        const entries = observed.map((target) => ({ target, contentRect: target.getBoundingClientRect() }));
        callback(entries as unknown as ResizeObserverEntry[], {} as ResizeObserver);
      }
    });
    flushFits();
    expect(probe.style.getPropertyValue("--cf-fit")).toBe("");
    expect(probe.hasAttribute("data-clamped")).toBe(false);
  });

  it("B15 in jsdom a rendered face leaves .card-name and .card-text with no inline style and no data-clamped", () => {
    for (const [id, radiant] of [
      ["core-093", false],
      ["core-095", true],
      ["core-098", true],
      ["core-051", true],
      ["core-028", false],
    ] as const) {
      const cf = catalogFace(id, radiant);
      for (const selector of [".card-name", ".card-text"]) {
        const el = one(cf, selector);
        expect(el.style.getPropertyValue("--cf-fit"), `${id} ${selector}`).toBe("");
        expect(el.style.length, `${id} ${selector}`).toBe(0);
        expect(el.hasAttribute("data-clamped"), `${id} ${selector}`).toBe(false);
      }
      cleanup();
    }
  });
});

/* ---------------------------------------------------------------------------------------- R503 */

describe("R503: the set mark and a token's printed rarity", () => {
  function setMark(cf: HTMLElement): HTMLElement {
    const marks = cf.querySelectorAll<HTMLElement>(".cf-set");
    expect(marks).toHaveLength(1);
    const mark = marks[0];
    if (mark === undefined) throw new Error("no .cf-set");
    return mark;
  }

  it("R503 a face shows its set: a glyph per set, named by title and aria-label, with data-set", () => {
    for (const [id, set, kind] of [
      ["core-002", "Core", "core"],
      ["classic-043", "Classic", "classic"],
      ["classicplus-043", "Classic+", "classic-plus"],
      ["classicplus-012-1", "Classic+", "classic-plus"],
      ["meditative-027", "Meditative", "meditative"],
    ] as const) {
      const mark = setMark(catalogFace(id));
      expect(mark.getAttribute("data-set"), id).toBe(set);
      expect(mark.getAttribute("data-set-mark"), id).toBe(kind);
      expect(mark.getAttribute("role"), id).toBe("img");
      expect(mark.getAttribute("aria-label"), id).toBe(`${set} set`);
      expect(mark.getAttribute("title"), id).toBe(`${set} set`);
      const glyph = one(mark, "img.cf-set-glyph");
      expect(glyph.getAttribute("alt"), id).toBe("");
      expect(glyph.getAttribute("aria-hidden"), id).toBe("true");
      expect(glyph.getAttribute("src"), id).toBe(setMarkOf(set).src);
      expect(mark.textContent, id).toBe("");
      cleanup();
    }
  });

  it("R503 the five glyphs differ, and Classic+'s is Classic's temple with a plus", () => {
    const sources = new Set(["Core", "Classic", "Classic+", "Meditative", "Boss"].map((set) => setMarkOf(set).src));
    expect(sources.size).toBe(5);
    const classic = decodeURIComponent(setMarkOf("Classic").src);
    const plus = decodeURIComponent(setMarkOf("Classic+").src);
    const temple = /<path d='M12 1\.8L22 7\.2H2Z[^']*'/.exec(classic)?.[0];
    expect(temple).toBeDefined();
    expect(plus).toContain(temple ?? "");
    for (const src of sources) {
      const svg = decodeURIComponent(src);
      expect(svg).not.toContain("<text");
      expect(svg).not.toContain("<title");
    }
  });

  it("R503 the Meditative mark is the ensō with a red seal, not the diamond", () => {
    const mark = setMarkOf("Meditative");
    expect(mark.kind).toBe("meditative");
    expect(mark.label).toBe("Meditative set");
    expect(mark.src).not.toBe(setMarkOf("Boss").src);
    const svg = decodeURIComponent(mark.src);
    // The open ring is one arc that does not close (the ink's brush stroke), the seal a red square.
    expect(svg).toContain("A7.8 7.8 0 1 0");
    expect(svg).toContain("fill='#c8322b'");
    expect(svg).not.toContain("<text");
    expect(svg).not.toContain("<title");
    expect(setMark(catalogFace("meditative-027")).querySelector("img")?.getAttribute("src")).toBe(mark.src);
  });

  it("R503 a set with no glyph of its own gets the fallback, still named; an unknown card shows no set", () => {
    const boss: CardDef = { ...def("core-002"), set: "Boss" };
    const mark = setMark(renderFace({ defId: boss.id, def: boss, radiant: false }));
    expect(mark.getAttribute("data-set")).toBe("Boss");
    expect(mark.getAttribute("data-set-mark")).toBe("unknown");
    expect(mark.getAttribute("aria-label")).toBe("Boss set");
    cleanup();
    const unknown = renderFace({ defId: UNKNOWN_ID, name: "Nobody", type: "Spell", radiant: false });
    expect(unknown.querySelector(".cf-set")).toBeNull();
  });

  it("R503 the mark shows on the radiant and compact faces too", () => {
    expect(setMark(catalogFace("classic-043", true)).getAttribute("data-set")).toBe("Classic");
    cleanup();
    expect(setMark(catalogFace("classicplus-038", false, "compact")).getAttribute("data-set")).toBe("Classic+");
  });

  it(
    "R503 every catalog face carries exactly one mark, its own set's",
    () => {
      // Glitch is blank, set mark and all (glitch.test.tsx).
      for (const card of DEFS.filter((entry) => entry.id !== GLITCH_DEF_ID)) {
        expect(setMark(catalogFace(card.id)).getAttribute("data-set"), card.id).toBe(card.set);
        cleanup();
      }
    },
    CATALOG_SWEEP_TIMEOUT_MS,
  );

  it("R503 setmark.css hides the mark on the smallest faces and moves it for the small layout, at the constants' sizes", () => {
    expect(setMarkCss).toContain(`@container cardface (max-height: ${FACE_TEXT_MIN_HEIGHT_PX - 1}px)`);
    expect(setMarkCss).toContain(`@container cardface (max-height: ${SET_MARK_MIN_FACE_PX - 1}px)`);
    expect(SET_MARK_MIN_FACE_PX).toBeLessThan(FACE_TEXT_MIN_HEIGHT_PX);
    // Nothing about the mark moves, so reduced motion has nothing to stop.
    expect(setMarkCss).not.toMatch(/animation|transition/);
  });

  it("R503 a token that prints a rarity wears that rarity's frame, gem and crest; its rarity stays Token", () => {
    const top = def("classicplus-019-1");
    expect(top.rarity).toBe("Token");
    expect(top.printedRarity).toBe("Legendary");
    const cf = catalogFace(top.id);
    expect(cf.getAttribute("data-rarity")).toBe("Legendary");
    expect(cf.getAttribute("data-printed-rarity")).toBe("Legendary");
    expect(one(cf, ".cf-gem").getAttribute("data-rarity")).toBe("Legendary");
    expect(cf.querySelectorAll(".cf-crest")).toHaveLength(1);
    expect(faceModel({ defId: top.id, def: top, radiant: false }).rarity).toBe("Token");
    cleanup();

    const rotten = catalogFace("classicplus-065-1");
    expect(rotten.getAttribute("data-rarity")).toBe("Common");
    expect(one(rotten, ".cf-gem").getAttribute("data-rarity")).toBe("Common");
    expect(rotten.querySelector(".cf-crest")).toBeNull();
  });

  it("R503 a Mythic-printed token carries the Mythic foil, animated or still as the setting says", () => {
    writeCardSettings({ animatedFoil: true });
    expect(catalogFace("classicplus-065-5").getAttribute("data-foil")).toBe("animated");
    cleanup();
    writeCardSettings({ animatedFoil: false });
    expect(catalogFace("classicplus-065-5").getAttribute("data-foil")).toBe("static");
  });

  it("R503 a token with no printed rarity keeps Token's look: no gem, no crest, no printed rarity", () => {
    for (const id of ["core-t-rush", "classicplus-t-ai-01", "core-095-1"]) {
      const cf = catalogFace(id);
      expect(cf.getAttribute("data-rarity"), id).toBe("Token");
      expect(cf.hasAttribute("data-printed-rarity"), id).toBe(false);
      expect(cf.querySelector(".cf-gem"), id).toBeNull();
      expect(cf.querySelector(".cf-crest"), id).toBeNull();
      cleanup();
    }
  });

  it("R503 the face's art is drawn with the card's name, so it carries the name's motif", () => {
    const art = one(catalogFace("classic-036"), ".cf-art-frame > .cf-art");
    expect(art.getAttribute("data-art-motif")).toBe("flames");
  });
});

/* ------------------------------------------------------------------------------------- R1382 */

describe("R1382: a card with every tribal tag prints All Tribes", () => {
  const withTags = (tags: readonly Tag[], chinese = false): HTMLElement => {
    const card: CardDef = { ...def("core-002"), tags: [...tags] };
    return renderFace({ defId: card.id, def: card, radiant: false, chinese });
  };
  const printed = (cf: HTMLElement): string[] => [...cf.querySelectorAll<HTMLElement>(".cf-tags > .cf-tag")].map((tag) => tag.textContent ?? "");

  it("R1382 the five tribal tags print as All Tribes, other tags after, and the four of five as themselves", () => {
    expect([...TRIBAL_TAGS]).toEqual(["Human", "Felinor", "KY", "CN", "Jlockeed"]);
    const all = withTags([...TRIBAL_TAGS]);
    expect(printed(all)).toEqual(["All Tribes"]);
    expect(one(all, ".cf-tag").getAttribute("data-tag")).toBe("All Tribes");
    cleanup();
    expect(printed(withTags(["Wincon", ...TRIBAL_TAGS]))).toEqual(["All Tribes", "Wincon"]);
    cleanup();
    expect(printed(withTags(TRIBAL_TAGS.slice(1)))).toEqual(["Felinor", "KY", "CN", "Jlockeed"]);
    cleanup();
    expect(printed(withTags(["Human"]))).toEqual(["Human"]);
  });

  it("R1382 a Chinese face prints the Chinese word, and the card's tags are unchanged", () => {
    expect(printed(withTags([...TRIBAL_TAGS], true))).toEqual([CHINESE_TERMS.allTribes]);
    cleanup();
    expect(printed(withTags(["Wincon", ...TRIBAL_TAGS], true))).toEqual([CHINESE_TERMS.allTribes, CHINESE_TERMS.tags.Wincon]);
    const model = faceModel({ defId: "core-002", def: { ...def("core-002"), tags: [...TRIBAL_TAGS] }, radiant: false });
    expect(model.tags).toEqual([...TRIBAL_TAGS]);
  });
});

function FitProbe({ content }: { content: string }): ReactElement {
  const ref = useRef<HTMLSpanElement>(null);
  useFitText(ref, content);
  return (
    <span data-probe="fit" ref={ref}>
      {content}
    </span>
  );
}
