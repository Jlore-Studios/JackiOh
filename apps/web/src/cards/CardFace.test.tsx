// Polish 6, slice B: the tall card face in jsdom (docs/polish/6-cards.md, "CardFace DOM": B8's gem,
// B10's `RulesText`, B12–B14, the jsdom half of B15). jsdom has no layout, so this checks the DOM
// contract only; the pixel half of B15 is e2e/cypress/component/card-faces.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, render, screen } from "@testing-library/react";
import { useRef, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID, TRIBAL_TAGS } from "@jackioh/engine/config";
import { SHIPPED_SETS, fillParams, type CardDef, type CardType, type Keyword, type PreviewValue, type Rarity, type Tag } from "@jackioh/shared";

import { CardBack } from "./CardBack.tsx";
import { CardFace, TOKEN_WORD, cx, faceText, typeLine } from "./CardFace.tsx";
import { CHINESE_TERMS } from "./chinese.ts";
import { GLITCH_WORDS } from "./glitch.ts";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { INSPECT_FACE, INSPECT_GLOSSARY, INSPECT_STATES } from "./inspect/testids.ts";
import { MinionFace } from "./MinionFace.tsx";
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
import {
  SET_MARK_MIN_FACE_PX,
  WATERMARK_INK,
  WATERMARK_INK_ON_DARK,
  WATERMARK_OPACITY,
  WATERMARK_OPACITY_ON_DARK,
  WATERMARK_SIZE_PCT,
  glyphUri,
  setMarkOf,
  watermarkOf,
} from "./setMark.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { SUMMIT_DEF_ID, ascentLevelOf, summitShortText } from "./shortText.ts";
import { fusedDef, unit } from "../test/fixtures.ts";

// R1017001: a full-art face prints none of its words, so full art is off here (fullArt.test.tsx).
beforeEach(() => {
  writeCardSettings({ fullArt: false });
});

afterEach(() => {
  cleanup();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
  vi.unstubAllGlobals();
});

/* helpers */

const DEFS: readonly CardDef[] = Object.values(CATALOG);
const XHTML = "http://www.w3.org/1999/xhtml";
const UNKNOWN_ID = "core-999";
/** The catalog sweeps below outrun vitest's 5 s default on a loaded machine. */
const CATALOG_SWEEP_TIMEOUT_MS = 60_000;

/** vitest stubs CSS imports, so the set mark's stylesheet is read as text. */
const setMarkCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "setmark.css"), "utf8");
/** And the faces' own, which draw no token mark (R2647000). */
const cardsCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "cards.css"), "utf8");
/** R2718000: the sheets that hold the watermark's ground and its inks. */
const cardStateCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "cardstate.css"), "utf8");
const inspectCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "inspect", "inspect.css"), "utf8");

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
 * Everything B12 and the "Forbidden inside a face" list rule out. A face must sit inside a `<button>`,
 * so its HTML is span, strong and img only; the Surface gives the icons as "aria-hidden SVG, no <text>".
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

/* B12 */

describe("glyphUri", () => {
  it("draws a glyph's body on a 24-unit square, as the set marks are drawn (#2160)", () => {
    const body = "<circle cx='12' cy='12' r='3'/>";
    const uri = glyphUri(body);
    expect(uri.startsWith("data:image/svg+xml,")).toBe(true);
    expect(decodeURIComponent(uri.slice("data:image/svg+xml,".length))).toBe(
      `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'>${body}</svg>`,
    );
  });
});

describe("cx", () => {
  it("joins the parts that are non-empty strings, in order (#2160)", () => {
    expect(cx("cf", false, "cf--full", undefined, null, "", "extra")).toBe("cf cf--full extra");
    expect(cx()).toBe("");
  });
});

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

  it("B12 one .cf-tag[data-tag] per tag but Token, which the type line carries (R1177001), in the def's order", () => {
    const tags = catalogFace("core-t-felinor").querySelectorAll(".cf-tags > .cf-tag");
    expect([...tags].map((tag) => tag.getAttribute("data-tag"))).toEqual(["Felinor"]);
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

/* B18 (compact layout) */

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

/* B8 (DOM) */

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

  it("B8 R1175000 the second cost badge is named for the Embiggened version, for a screen reader and on hover", () => {
    for (const [id, cost] of [["core-046", "4"], ["core-056", "3"], ["meditative-021", "1"], ["meditative-081", "2"]] as const) {
      const alt = one(one(catalogFace(id), ".cost-gem"), ".cf-cost-alt");
      expect(alt.textContent, id).toBe(cost);
      expect(alt.getAttribute("role"), id).toBe("img");
      expect(alt.getAttribute("aria-label"), id).toBe(`Embiggened (${cost})`);
      expect(alt.getAttribute("title"), id).toBe(`Embiggened (${cost})`);
      cleanup();
    }
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

/* B10 (RulesText in DOM) */

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
    // R910000: "Can use enemy Units as Tributes" bolds its plural too.
    expect(terms.map((strong) => strong.textContent)).toEqual(["Taunt", "Tribute 3", "Tributes"]);
    expect(terms.map((strong) => strong.getAttribute("data-term"))).toEqual(["Taunt", "Tribute", "Tribute"]);
  });

  it("B10 every catalog face, base and radiant, Classic and Classic+ too, draws each term its text names in bold, its keywords among them (#85)", () => {
    for (const card of DEFS) {
      for (const radiant of [false, true]) {
        const source: FaceSource = { defId: card.id, def: card, radiant };
        const label = `${card.id} ${radiant ? "radiant" : "base"}`;
        const drawn = new Set([...renderFace(source).querySelectorAll(".card-text strong.cf-term")].map((term) => term.getAttribute("data-term")));
        // The words the face prints: its short form where it has one (#1017).
        expect([...drawn], label).toEqual(termsIn(faceText(faceModel(source)).shown));
        for (const keyword of (radiant ? card.radiant : card.base).keywords) expect(drawn.has(keyword.kind), `${label} ${keyword.kind}`).toBe(true);
        cleanup();
      }
    }
  }, CATALOG_SWEEP_TIMEOUT_MS);

  it("B10 R1320 patch v0.2.0's words are bold terms (Activate ♾️, Brittle, Animated, Buff, Nerf), and so is Classic #65's End of turn (#85)", () => {
    const termsOf = (id: string): string[] =>
      [...catalogFace(id).querySelectorAll(".card-text strong.cf-term")].map((term) => `${term.getAttribute("data-term") ?? ""}=${term.textContent ?? ""}`);
    expect(termsOf("classic-015")).toContain("Activate=Activate ♾️:");
    cleanup();
    expect(termsOf("classicplus-074")).toContain("Brittle=Brittle");
    cleanup();
    expect(termsOf("classic-038")[0]).toBe("Animated=Animated");
    cleanup();
    const tuners = termsOf("classicplus-070").map((term) => term.split("=")[0]);
    expect(tuners).toContain("Buff");
    expect(tuners).toContain("Nerf");
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
      base: { keywords: [], text: "taunt the Locking Rushing units" },
      radiant: { keywords: [], text: "taunt the Locking Rushing units" },
    };
    const cf = renderFace({ defId: plain.id, def: plain, radiant: false });
    expect(cf.querySelectorAll("strong")).toHaveLength(0);
    expect(one(cf, ".card-text").textContent).toBe("taunt the Locking Rushing units");
  });

  it(
    "B10 every catalog text reads back unchanged through RulesText, marks and all",
    () => {
      for (const card of DEFS) {
        for (const radiant of [false, true]) {
          const model = faceModel({ defId: card.id, def: card, radiant });
          const text = model.text;
          const cf = catalogFace(card.id, radiant);
          // Its whole text, or the short form that starts it (#1017).
          expect(one(cf, ".cf-text-base").textContent, card.id).toBe(faceText(model).shown);
          // The face's catalog text, its `{key}` numbers filled in with its printed values (B3.4 rule 5).
          expect(text.full, card.id).toBe(fillParams(card, radiant ? "radiant" : "base"));
          cleanup();
        }
      }
    },
    CATALOG_SWEEP_TIMEOUT_MS,
  );
});

/* B13 */

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

/* B14 */

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
    expect(one(text, ".cf-text-base").textContent).toBe("Cry: Flip 10 coins. Gain +1 Attack per heads and +1 Health per tails. Flip a coin. If heads, gain Divine Shield; if tails, gain Rush.");
    expect(text.querySelector(".cf-text-radiant")).toBeNull();
    expect([...text.querySelectorAll(".cf-mark")].map((mark) => mark.textContent)).toEqual(["10"]);
  });

  it("B14 a radiant face prints its own keyword line, never the base keywords it replaced (core-056, core-025)", () => {
    const jilliax = one(catalogFace("core-056", true), ".card-text");
    // "Embiggened (3): Equip." is on both faces, below the keyword line (R1175000).
    expect(jilliax.textContent).toBe("Rush, Taunt, Lifesteal, Divine Shield, Reborn\nEmbiggened (3): Equip.");
    expect([...jilliax.querySelectorAll("strong.cf-term")].map((term) => term.getAttribute("data-term"))).toEqual([
      "Rush",
      "Taunt",
      "Lifesteal",
      "Divine Shield",
      "Reborn",
      "Embiggened",
      "Equip",
    ]);
    cleanup();
    expect(one(catalogFace("core-025", true), ".card-text").textContent).toBe("Armor 7, Reborn");
  });

  it("B14 a clause the radiant cell restates is printed once, in its radiant form (core-002, core-046)", () => {
    const bigot = one(catalogFace("core-002", true), ".card-text");
    expect(bigot.textContent).toBe("Cry: Destroy all enemy non-Human Units.");
    expect(bigot.querySelectorAll('strong.cf-term[data-term="Cry"]')).toHaveLength(1);
    cleanup();
    expect(one(catalogFace("core-046", true), ".card-text").textContent).toBe("Aura: Enemy Units have −2/−2.\nEmbiggened (4): −4/−4 instead.");
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

/* B15 (jsdom half) */

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
      ["core-052", true, "l"],
      ["core-051", false, "l"],
      ["core-051", true, "l"],
      // A short form is what the face prints (#1017).
      ["core-093", false, "l"],
      ["core-095", true, "m"],
      ["meditative-022-1", false, "l"],
      // A clamp prints its whole text.
      ["meditative-045", true, "xl"],
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

/* R503 */

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

  it("R1883000 the Stinkyscape mark is a ringed green planet in deep space under stink lines, not the diamond", () => {
    const mark = setMarkOf("Stinkyscape");
    expect(mark.kind).toBe("stinkyscape");
    expect(mark.label).toBe("Stinkyscape set");
    const others = ["Core", "Classic", "Classic+", "Meditative", "Boss"].map((set) => setMarkOf(set).src);
    expect(others).not.toContain(mark.src);
    const svg = decodeURIComponent(mark.src);
    // The deep-space disc, the green planet, its tilted ring (its far half drawn before the planet,
    // its near half after) and the three stink lines.
    expect(svg).toContain("fill='#1c1446'");
    const far = svg.indexOf("M2.6 14.2A8 2.2");
    const planet = svg.indexOf("fill='#7fd23e'");
    const near = svg.indexOf("M18.6 14.2A8 2.2");
    expect(far).toBeGreaterThan(-1);
    expect(planet).toBeGreaterThan(far);
    expect(near).toBeGreaterThan(planet);
    expect(svg.match(/M[\d.]+ [\d.]+Q/g)).toHaveLength(3);
    expect(svg).not.toContain("<text");
    expect(svg).not.toContain("<title");
    const face = setMark(catalogFace("stinkyscape-017"));
    expect(face.getAttribute("data-set")).toBe("Stinkyscape");
    expect(face.getAttribute("data-set-mark")).toBe("stinkyscape");
    expect(face.getAttribute("aria-label")).toBe("Stinkyscape set");
    expect(face.querySelector("img")?.getAttribute("src")).toBe(mark.src);
  });

  it("R2202001 the Training Grounds mark is a straw target with red rings over two crossed wooden swords, not the diamond", () => {
    const mark = setMarkOf("Training Grounds");
    expect(mark.kind).toBe("training-grounds");
    expect(mark.label).toBe("Training Grounds set");
    const others = ["Core", "Classic", "Classic+", "Meditative", "Stinkyscape", "Meditative+", "Boss"].map((set) => setMarkOf(set).src);
    expect(others).not.toContain(mark.src);
    const svg = decodeURIComponent(mark.src);
    // The two swords (each an outline under its wood) are drawn before the target, and the target is
    // four rings, straw and red in turn.
    expect(svg.match(/stroke='#c58a48'/g)).toHaveLength(2);
    const target = svg.indexOf("fill='#f1dfa8'");
    expect(target).toBeGreaterThan(svg.lastIndexOf("stroke='#c58a48'"));
    expect(svg.match(/<circle/g)).toHaveLength(4);
    expect(svg.match(/fill='#d2452f'/g)).toHaveLength(2);
    expect(svg).not.toContain("<text");
    expect(svg).not.toContain("<title");
    const face = setMark(catalogFace("traininggrounds-017"));
    expect(face.getAttribute("data-set")).toBe("Training Grounds");
    expect(face.getAttribute("data-set-mark")).toBe("training-grounds");
    expect(face.getAttribute("aria-label")).toBe("Training Grounds set");
    expect(face.querySelector("img")?.getAttribute("src")).toBe(mark.src);
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

/* R1382 */

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

/* R1177001 */

/** Every token a face can show: the catalog's, Glitch aside (it stays blank, R674). */
const TOKENS: readonly CardDef[] = DEFS.filter((card) => card.token && card.id !== GLITCH_DEF_ID);

/** A board minion of `id`, as Card.tsx draws a unit on the field. */
function minionFace(id: string, extra: Partial<FaceSource> = {}): HTMLElement {
  const u = unit("p1", { defId: id });
  const face = faceModel({
    defId: id,
    def: def(id),
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
    inPlay: { created: true },
    ...extra,
  });
  const { container } = render(<MinionFace face={face} unit={u} />);
  return one(container, ".cf");
}

describe("R1177001: every token's type line says Token", () => {
  it(
    "R1177001 R2647000 every token's full face prints Token before its type, wears no token mark and no Token chip; no card's does",
    () => {
      expect(TOKENS.length).toBeGreaterThanOrEqual(79);
      for (const card of TOKENS) {
        const cf = catalogFace(card.id);
        expect(one(cf, ".card-type").textContent, card.id).toBe(`${TOKEN_WORD} ${card.type}`);
        expect(cf.querySelector(".cf-token, [data-token]"), card.id).toBeNull();
        expect(cf.querySelector('.cf-tag[data-tag="Token"]'), card.id).toBeNull();
        cleanup();
      }
      for (const card of DEFS.filter((entry) => !entry.token).slice(0, 40)) {
        const cf = catalogFace(card.id);
        expect(one(cf, ".card-type").textContent, card.id).toBe(card.type);
        cleanup();
      }
    },
    CATALOG_SWEEP_TIMEOUT_MS,
  );

  it("R1177001 the compact face of a face-up backrow token keeps its type line", () => {
    for (const id of ["classicplus-012-5", "classicplus-042-1", "classicplus-t-ai-10", "meditative-071-1"]) {
      const cf = catalogFace(id, false, "compact");
      expect(cf.getAttribute("data-layout"), id).toBe("compact");
      expect(one(cf, ".card-type").textContent, id).toBe(`${TOKEN_WORD} ${def(id).type}`);
      cleanup();
    }
  });

  it("R1177001 R503 a token that prints a rarity keeps its frame, gem and crest, and still says Token on its type line", () => {
    for (const id of ["classicplus-019-1", "classicplus-065-5", "meditative-071-1"]) {
      const printed = def(id).printedRarity;
      expect(printed, id).toBeDefined();
      for (const layout of ["full", "compact"] as const) {
        const cf = catalogFace(id, false, layout);
        expect(cf.getAttribute("data-rarity"), id).toBe(printed);
        expect(one(cf, ".cf-gem").getAttribute("data-rarity"), id).toBe(printed);
        expect(one(cf, ".card-type").textContent, id).toBe(`${TOKEN_WORD} ${def(id).type}`);
        cleanup();
      }
    }
  });

  it("R1177001 a token with three or four tags prints Token on its type line, and the tag row the others", () => {
    for (const [id, tags] of [
      ["meditative-045-1", ["CN", "KY", "Prime"]],
      ["meditative-049-1", ["CN", "Acclaimed", "Prime"]],
      ["classicplus-046-1", ["Felinor", "Prime"]],
    ] as const) {
      expect(def(id).tags, id).toContain("Token");
      const cf = catalogFace(id);
      expect([...cf.querySelectorAll(".cf-tags > .cf-tag")].map((tag) => tag.getAttribute("data-tag")), id).toEqual(tags);
      expect(one(cf, ".card-type").textContent, id).toBe("Token Unit");
      cleanup();
    }
  });

  it("R1177001 R1301 a Chinese token face prints 衍生物 before the table's type on its type line", () => {
    const token = CHINESE_TERMS.tags.Token;
    expect(token).toBe("衍生物");
    const cf = catalogFace("classicplus-012-5", false, "full", { chinese: true });
    expect(one(cf, ".card-type").textContent).toBe(`${token}${CHINESE_TERMS.types["Field Spell"]}`);
    expect(cf.textContent).not.toContain(TOKEN_WORD);
    // A Chinese card that is no token prints the table's type alone.
    expect(typeLine(faceModel({ defId: "core-011", def: def("core-011"), radiant: false, chinese: true }))).toBe(CHINESE_TERMS.types.Unit);
  });

  it("R1177001 R674 Glitch wears no token label: its type line stays corrupted", () => {
    const cf = catalogFace(GLITCH_DEF_ID);
    expect(one(cf, ".card-type").textContent).toBe(GLITCH_WORDS.type);
  });

  it("R1177001 R1177000 a token fused with a real card is labeled as the real card it is; tokens fused together as a token", () => {
    const real = fusedDef([def("core-t-rush"), def("core-011")]);
    expect(real.token).toBe(false);
    expect(real.tags).not.toContain("Token");
    const realFace = renderFace({ defId: real.id, def: real, radiant: false });
    expect(one(realFace, ".card-type").textContent).toBe("Unit");
    cleanup();

    const pair = fusedDef([def("core-t-rush"), def("core-t-sheep")], 2);
    expect(pair.token).toBe(true);
    expect(pair.tags).toContain("Token");
    const pairFace = renderFace({ defId: pair.id, def: pair, radiant: false });
    expect(one(pairFace, ".card-type").textContent).toBe("Token Unit");
  });
});

describe("R1107: a granted Death ability prints on its own line", () => {
  it("R1107 prints a granted ability after the gained keywords and inside the printed text", () => {
    const grant = "Death: Add 1 random Book to your hand.";
    const cf = catalogFace("meditative-058", false, "full", { inPlay: { grants: [grant] } });
    const text = one(cf, ".card-text");
    const granted = text.querySelectorAll(".cf-text-granted");
    expect(granted).toHaveLength(1);
    expect(granted[0]?.textContent).toBe(grant);
    expect(text.textContent).toContain(grant);
  });
});

/* issue #814: short faces */

describe("issue #814: long faces truncate with an ellipsis and keep their whole text", () => {
  const SUMMIT = SUMMIT_DEF_ID;

  function summitBox(level: number | null): HTMLElement {
    const { container } = render(
      <CardFace
        face={faceModel({ defId: SUMMIT, def: def(SUMMIT), radiant: false })}
        {...(level === null ? {} : { ascentLevel: level })}
      />,
    );
    flushFits();
    return one(container, ".card-text");
  }

  it("ascentLevelOf reads the engine's Ascent level preview (R280), and nothing else", () => {
    const level = (value: number): PreviewValue => ({ label: "Ascent level", value });
    expect(ascentLevelOf([])).toBeNull();
    expect(ascentLevelOf([{ label: "Damage", value: 2 }])).toBeNull();
    expect(ascentLevelOf([{ label: "Damage", value: 2 }, level(4)])).toBe(4);
  });

  it("summitShortText keeps the header and only the level's Ascent line", () => {
    const sample = ["Header one.", "Header two.", "Ascent 0: Rest.", "Ascent 1: Fight.", "Ascent 2: Win."].join("\n");
    expect(summitShortText(SUMMIT, sample, 1)).toBe("Header one.\nHeader two.\nAscent 1: Fight.");
    expect(summitShortText(SUMMIT, sample, 0)).toBe("Header one.\nHeader two.\nAscent 0: Rest.");
    // Nothing to shorten to: another card, no level, no Ascent lines, or a level past them.
    expect(summitShortText("core-098", sample, 1)).toBeNull();
    expect(summitShortText(SUMMIT, sample, null)).toBeNull();
    expect(summitShortText(SUMMIT, sample, undefined)).toBeNull();
    expect(summitShortText(SUMMIT, "no Ascent lines here", 1)).toBeNull();
    expect(summitShortText(SUMMIT, sample, 9)).toBeNull();
  });

  it("a Summit face with a level prints its header and only the next Ascent line", () => {
    const box = summitBox(1);
    expect(box.getAttribute("data-short")).toBe("true");
    expect(box.getAttribute("data-less")).toBe("true");
    expect(box.textContent).toContain("Costs (Ascent level)");
    expect(box.textContent).toContain("End of turn: Return this to hand.");
    expect(box.textContent).toContain("Ascent 1:");
    for (const hidden of ["Ascent 0:", "Ascent 2:", "Ascent 9:", "Ascent 10:"]) {
      expect(box.textContent, hidden).not.toContain(hidden);
    }
  });

  it("#912: with hover previews on (the default) the rules box has no tooltip, the preview has the whole text", () => {
    const box = summitBox(10);
    expect(box.getAttribute("data-short")).toBe("true");
    expect(box.textContent).toContain("Ascent 10: Win the game.");
    expect(box.textContent).not.toContain("Ascent 0:");
    expect(box.hasAttribute("title")).toBe(false);
  });

  it("#912: with hover previews off, a shortened Summit keeps its whole text in the rules box's tooltip", () => {
    writeCardSettings({ hoverPreviews: false });
    const box = summitBox(10);
    expect(box.getAttribute("data-short")).toBe("true");
    const whole = faceModel({ defId: SUMMIT, def: def(SUMMIT), radiant: false }).text.full;
    expect(box.getAttribute("title")).toBe(whole);
    expect(box.getAttribute("title")).toContain("Ascent 0:");
  });

  it("#1017: a Summit without a level prints its short form, its header ending in \"…\", as does one past its lines", () => {
    for (const level of [null, 99]) {
      const box = summitBox(level);
      expect(box.getAttribute("data-short"), String(level)).toBe("true");
      expect(box.textContent, String(level)).toContain("End of turn: Return this to hand…");
      expect(box.textContent, String(level)).not.toContain("Ascent 0:");
      cleanup();
    }

    // A level means nothing off the Summit: Heroic Power prints its own short form, the header over
    // its thirteen powers.
    const { container } = render(
      <CardFace face={faceModel({ defId: "core-098", def: def("core-098"), radiant: false })} ascentLevel={3} />,
    );
    flushFits();
    const heroic = one(container, ".card-text");
    expect(heroic.getAttribute("data-short")).toBe("true");
    expect(heroic.textContent).toContain("Gain one of 13 random powers");
    expect(heroic.textContent).not.toContain("Expedition Map");
  });

  it("#1017: a face that fits prints its whole text, and a clamp prints its whole text as printing less", () => {
    const { container } = render(<CardFace face={faceModel({ defId: "core-052", def: def("core-052"), radiant: true })} />);
    flushFits();
    const fits = one(container, ".card-text");
    expect(fits.hasAttribute("data-short")).toBe(false);
    expect(fits.hasAttribute("data-less")).toBe(false);
    cleanup();

    const clamp = faceModel({ defId: "meditative-049-2", def: def("meditative-049-2"), radiant: false });
    const view = render(<CardFace face={clamp} />);
    flushFits();
    const box = one(view.container, ".card-text");
    expect(box.hasAttribute("data-short")).toBe(false);
    expect(box.getAttribute("data-less")).toBe("true");
    expect(one(view.container, ".cf-text-base").textContent).toBe(clamp.text.full);
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

/* ----------------------------------------------------------------------------------- R2577000 */

const PREVIEW_ANCHOR = { left: 600, top: 200, width: 100, height: 140 };

describe("R2577000: the preview lists the keywords the board minion hides", () => {
  it("R2577000 a unit with Equip and Taunt hides Equip on the board, and its hover preview and full face still list it", () => {
    const id = "meditative-065";
    const keywords: Keyword[] = [{ kind: "Taunt" }, { kind: "Equip" }];
    const u = unit("p1", { defId: id, keywords });
    const face = faceModel({ defId: id, def: def(id), radiant: false, live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords } });
    const board = render(<MinionFace face={face} unit={u} />);
    expect(board.container.querySelector('.keywords > [data-keyword="Equip"]')?.getAttribute("data-chip")).toBe("hidden");
    expect(board.container.querySelector('[data-keyword-fx="Equip"]')).toBeNull();
    cleanup();
    render(<HoverPreview face={face} anchor={PREVIEW_ANCHOR} />);
    const glossary = screen.getByTestId(INSPECT_GLOSSARY);
    expect(glossary.querySelector('[data-glossary-term="Equip"]')).not.toBeNull();
    expect(glossary.querySelector('[data-glossary-term="Taunt"]')).not.toBeNull();
    expect(screen.getByTestId(INSPECT_FACE).querySelector('.card-text strong.cf-term[data-term="Equip"]')).not.toBeNull();
  });
});

/* ----------------------------------------------------------------------------------- R2647000 */

describe("R2647000: Token and Created live in the hover preview, not on the faces", () => {
  it(
    "R2647000 no token face wears a token mark: compact, on the board, or by a rule where the type line hides",
    () => {
      for (const card of TOKENS) {
        expect(catalogFace(card.id, false, "compact").querySelector(".cf-token, [data-token]"), card.id).toBeNull();
        cleanup();
      }
      const token = minionFace("core-t-rush");
      expect(token.querySelector(".cf-token, [data-token]")).toBeNull();
      const stats = one(token, ".stats");
      expect([...stats.children].map((child) => child.className)).toEqual(["stat stat-attack", "stat stat-health"]);
      expect(stats.textContent).toBe("34/4");
      // minionFace's face is Created: no badge on the board either.
      expect(token.querySelector('.cf-state[data-state="created"]')).toBeNull();
      // jsdom has no layout, so a small or full-art face is this same DOM: no stylesheet may bring a mark back.
      expect(cardsCss).not.toContain("cf-token");
    },
    CATALOG_SWEEP_TIMEOUT_MS,
  );

  it("R2647000 R1177001 R943 the hover preview still says both: Token Unit on its type line, Created in its state notes", () => {
    const id = "core-t-rush";
    const u = unit("p1", { defId: id });
    const source: FaceSource = {
      defId: id,
      def: def(id),
      radiant: false,
      live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
      inPlay: { created: true },
    };
    render(<HoverPreview face={faceModel(source)} anchor={PREVIEW_ANCHOR} />);
    const shown = screen.getByTestId(INSPECT_FACE);
    expect(one(shown, ".card-type").textContent).toBe(`${TOKEN_WORD} Unit`);
    expect(shown.querySelector('.cf-token, [data-token], .cf-state[data-state="created"]')).toBeNull();
    expect(one(screen.getByTestId(INSPECT_STATES), '.inspect-state[data-state="created"]').textContent).toContain(
      "Created: made during the game",
    );
    cleanup();
    render(<HoverPreview face={faceModel({ ...source, chinese: true })} anchor={PREVIEW_ANCHOR} />);
    expect(one(screen.getByTestId(INSPECT_FACE), ".card-type").textContent).toBe(`${CHINESE_TERMS.tags.Token}${CHINESE_TERMS.types.Unit}`);
  });
});

/* -------------------------------------------------------------------------------------- R2718000 */

/** WCAG relative luminance of `hex`, laid at `alpha` over `over` when given (palette.test.ts's). */
function luminance(hex: string, over?: string, alpha = 1): number {
  const channel = (at: number): number => {
    const own = parseInt(hex.slice(at, at + 2), 16);
    const under = over === undefined ? own : parseInt(over.slice(at, at + 2), 16);
    const c = (alpha * own + (1 - alpha) * under) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

function contrast(a: number, b: number): number {
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** The declarations of the first rule written exactly `selector {` in `css`. */
function ruleBody(css: string, selector: string): string {
  const open = css.indexOf(`${selector} {`);
  if (open < 0) throw new Error(`no ${selector} rule`);
  return css.slice(open, css.indexOf("}", open));
}

describe("R2718000: set watermarks", () => {
  /** One card of each set that ships, with its set. */
  const SEVEN = [
    ["core-002", "Core"],
    ["classic-043", "Classic"],
    ["classicplus-043", "Classic+"],
    ["meditative-027", "Meditative"],
    ["meditativeplus-001", "Meditative+"],
    ["stinkyscape-017", "Stinkyscape"],
    ["traininggrounds-017", "Training Grounds"],
  ] as const;

  function watermarks(cf: Element): HTMLElement[] {
    return [...cf.querySelectorAll<HTMLElement>(".cf-watermark")];
  }

  it("R2718000 every set that ships or the catalog holds has its own set mark and watermark, all distinct; Meditative+'s mark is the ensō with Classic+'s plus", () => {
    const sets = [...new Set<string>([...SHIPPED_SETS, ...DEFS.map((card) => card.set)])];
    expect(sets).toEqual(expect.arrayContaining(["Core", "Classic", "Classic+", "Meditative", "Meditative+", "Stinkyscape", "Training Grounds"]));
    for (const set of sets) {
      const kind = setMarkOf(set).kind;
      expect(kind, set).not.toBe("unknown");
      expect(watermarkOf(set)?.kind, set).toBe(kind);
    }
    const marks = sets.map((set) => setMarkOf(set).src);
    const prints = sets.map((set) => watermarkOf(set)?.src);
    const onDark = sets.map((set) => watermarkOf(set)?.srcOnDark);
    expect(new Set(marks).size).toBe(sets.length);
    expect(new Set(prints).size).toBe(sets.length);
    expect(new Set(onDark).size).toBe(sets.length);
    expect(prints).not.toContain(undefined);
    for (const none of [null, "Boss", "Boss-X", "Custom", "Nonesuch", "constructor"]) expect(watermarkOf(none), String(none)).toBeNull();

    const mark = setMarkOf("Meditative+");
    expect(mark.kind).toBe("meditative-plus");
    expect(mark.label).toBe("Meditative+ set");
    expect(mark.src).not.toBe(setMarkOf("Boss").src);
    const svg = decodeURIComponent(mark.src);
    expect(svg).toContain("A7.8 7.8 0 1 0");
    expect(svg).toContain("M17.6 0.8H21.2");
    expect(svg).toContain("scale(0.8)");
    expect(svg).not.toContain("<text");
    expect(svg).not.toContain("<title");
    expect(one(catalogFace("meditativeplus-001"), ".cf-set").getAttribute("data-set-mark")).toBe("meditative-plus");
  });

  it("R2718000 a watermark is its mark's own shapes in one ink at its named opacity", () => {
    for (const set of SHIPPED_SETS) {
      const mark = decodeURIComponent(setMarkOf(set).src);
      const body = mark.slice(mark.indexOf("viewBox='0 0 24 24'>") + "viewBox='0 0 24 24'>".length, mark.lastIndexOf("</svg>"));
      expect(body, set).not.toBe("");
      const watermark = watermarkOf(set);
      for (const [src, ink, opacity] of [
        [watermark?.src, WATERMARK_INK, WATERMARK_OPACITY],
        [watermark?.srcOnDark, WATERMARK_INK_ON_DARK, WATERMARK_OPACITY_ON_DARK],
      ] as const) {
        expect(src?.startsWith("data:image/svg+xml,"), set).toBe(true);
        const svg = decodeURIComponent(src?.slice("data:image/svg+xml,".length) ?? "");
        expect(svg, set).toContain(body);
        expect(svg.match(new RegExp(`flood-color='${ink}'`, "g")), set).toHaveLength(1);
        expect(svg, set).toContain(`opacity='${String(opacity)}'`);
        expect(svg, set).not.toContain("<text");
        expect(svg, set).not.toContain("<title");
      }
    }
  });

  it("R2718000 a card of each set prints its set's watermark first in its rules box, on base, Radiant and Chinese faces", () => {
    const faces: [string, string, boolean, Partial<FaceSource>][] = [];
    for (const [id, set] of SEVEN) for (const radiant of [false, true]) faces.push([id, set, radiant, {}]);
    faces.push(["meditative-027", "Meditative", false, { chinese: true }]);
    for (const [id, set, radiant, extra] of faces) {
      const label = `${id} radiant=${String(radiant)} ${JSON.stringify(extra)}`;
      const cf = catalogFace(id, radiant, "full", extra);
      const found = watermarks(cf);
      expect(found, label).toHaveLength(1);
      const image = found[0];
      expect(image?.tagName, label).toBe("IMG");
      expect(one(cf, ".card-text").firstElementChild, label).toBe(image);
      expect(image?.getAttribute("src"), label).toBe(watermarkOf(set)?.src);
      expect(image?.getAttribute("alt"), label).toBe("");
      expect(image?.getAttribute("aria-hidden"), label).toBe("true");
      expect(image?.getAttribute("data-watermark"), label).toBe(watermarkOf(set)?.kind);
      expect(faceProblems(cf), label).toEqual([]);
      cleanup();
    }
  });

  it("R2718000 a blank rules box shows it too", () => {
    const cf = catalogFace("classic-086");
    expect(one(cf, ".cf-text-base").textContent).toBe("");
    expect(watermarks(cf)).toHaveLength(1);
    expect(one(cf, ".card-text").firstElementChild).toBe(watermarks(cf)[0]);
  });

  it("R2718000 no watermark on Glitch, a compact face, a full-art face, a card back, a set with no symbol or an unknown card", () => {
    expect(watermarks(catalogFace(GLITCH_DEF_ID))).toHaveLength(0);
    cleanup();
    expect(watermarks(catalogFace("core-002", false, "compact"))).toHaveLength(0);
    cleanup();
    writeCardSettings({ fullArt: true });
    const art = catalogFace("core-001");
    expect(art.getAttribute("data-full-art")).toBe("true");
    expect(art.querySelector(".card-text")).toBeNull();
    expect(watermarks(art)).toHaveLength(0);
    cleanup();
    writeCardSettings({ fullArt: false });
    const back = render(<CardBack />).container;
    expect(back.querySelector("img")).toBeNull();
    expect(back.querySelector(".cf-watermark")).toBeNull();
    cleanup();
    for (const set of ["Boss", "Custom"] as const) {
      const card: CardDef = { ...def("core-002"), set };
      const cf = renderFace({ defId: card.id, def: card, radiant: false });
      expect(cf.querySelector(".card-text"), set).not.toBeNull();
      expect(watermarks(cf), set).toHaveLength(0);
      cleanup();
    }
    expect(watermarks(renderFace({ defId: UNKNOWN_ID, name: "Nobody", type: "Spell", radiant: false }))).toHaveLength(0);
  });

  it("R2718000 the watermark hides with the rules box below FACE_TEXT_MIN_HEIGHT_PX, lies under the text at WATERMARK_SIZE_PCT, takes no pointer", () => {
    expect(cardsCss).toMatch(
      new RegExp(`@container cardface \\(max-height: ${String(FACE_TEXT_MIN_HEIGHT_PX - 1)}px\\) \\{\\s*\\.cf:not\\(\\[data-layout="minion"\\]\\) \\.card-text,`),
    );
    const box = ruleBody(cardsCss, ".cf .card-text");
    expect(box).toContain("position: absolute;");
    expect(box).toContain("z-index: 1;");
    for (const [css, selector] of [
      [setMarkCss, ".cf .cf-watermark"],
      [inspectCss, ".inspect-watermark"],
    ] as const) {
      const rule = ruleBody(css, selector);
      for (const declaration of [
        "position: absolute;",
        "z-index: -1;",
        "pointer-events: none;",
        "object-fit: contain;",
        `top: ${String((100 - WATERMARK_SIZE_PCT) / 2)}%;`,
        `height: ${String(WATERMARK_SIZE_PCT)}%;`,
      ]) {
        expect(rule, `${selector} ${declaration}`).toContain(declaration);
      }
    }
    // It never moves, and every colour of it is in setMark.ts.
    expect(ruleBody(setMarkCss, ".cf .cf-watermark")).not.toMatch(/animation|transition|#[0-9a-f]{3}|rgb|hsl/i);
    expect(ruleBody(inspectCss, ".inspect-watermark")).not.toMatch(/animation|transition|#[0-9a-f]{3}|rgb|hsl/i);
  });

  it("R2718000 over the parchment the watermark leaves every ink as readable as the box's own edge, and the Radiant gold at 5:1", () => {
    const boxes = [
      ...cardsCss.matchAll(/radial-gradient\(120% 90% at 50% 40%, (#[0-9a-f]{6}), (#[0-9a-f]{6}) 70%, (#[0-9a-f]{6})\)/g),
    ].map((match) => match.slice(1, 4));
    // The normal rules box, then the Radiant one.
    expect(boxes.length).toBeGreaterThanOrEqual(2);
    const [normal, radiant] = boxes;
    expect(normal).toEqual(["#f6ecd4", "#e3d2ac", "#c9b48a"]);
    expect(radiant).toEqual(["#fff6d6", "#f1d98c", "#d9b458"]);
    const inks = ["#2b2116", "#6b4200", "#1d4f8a", "#1d5a1f", "#13692a", "#a3120f"];
    for (const ink of inks) expect(cardsCss + cardStateCss, ink).toContain(ink);
    for (const box of [normal, radiant]) {
      const [lightest, middle, edge] = box ?? [];
      if (lightest === undefined || middle === undefined || edge === undefined) throw new Error("a box lacks a stop");
      for (const stop of [lightest, middle]) {
        const blended = luminance(WATERMARK_INK, stop, WATERMARK_OPACITY);
        expect(blended, stop).toBeLessThan(luminance(stop));
        for (const ink of inks) {
          expect(contrast(luminance(ink), blended), `${ink} on ${stop} under the watermark`).toBeGreaterThanOrEqual(
            contrast(luminance(ink), luminance(edge)),
          );
        }
      }
    }
    // R277: the Radiant box's gold keeps 5:1 where the text sits, the watermark under it or not.
    for (const stop of (radiant ?? []).slice(0, 2)) {
      expect(contrast(luminance("#6b4200"), luminance(WATERMARK_INK, stop, WATERMARK_OPACITY)), stop).toBeGreaterThanOrEqual(5);
    }
    // The inspect panels' dark ground, and the lighter ink on it.
    expect(inspectCss).toContain("background: rgba(11, 13, 19, 0.94);");
    const ground = "#0b0d13";
    const lifted = luminance(WATERMARK_INK_ON_DARK, ground, WATERMARK_OPACITY_ON_DARK);
    expect(lifted).toBeGreaterThan(luminance(ground));
    for (const ink of ["#e8eaf2", "#ffffff", "#ffd24a", "#8cc4ff", "#7dff9a", "#ff8a8a", "#9aa1b8"]) {
      expect(contrast(luminance(ink), lifted), `${ink} on the dark ground under the watermark`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("R2718000 with Set watermarks off every face prints as before", () => {
    writeCardSettings({ setWatermarks: false });
    for (const [id] of SEVEN) {
      const cf = catalogFace(id);
      expect(cf.querySelector(".card-text"), id).not.toBeNull();
      expect(watermarks(cf), id).toHaveLength(0);
      expect(cf.querySelector("[data-watermark]"), id).toBeNull();
      cleanup();
    }
  });
});
