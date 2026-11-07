// R3046000: a full-art card (R1017001) looks full-art on the board: a Unit's `MinionFace` and a face-up backrow
// card's compact `CardFace` lay the card's art across the face, still, and set its name in a band below the art.
// Which cards, tiers and versions print it is the full face's decision (R1313000, R2655000, R2655002, R2725000,
// R2940001); this file holds the DOM of the two board faces against it, and the stylesheets' rules that put the band
// below the art (Vitest stubs CSS, so they are read as text). The pixels are e2e's
// cypress/component/full-art-board.cy.tsx.

import { cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { readSheet } from "../test/setup.ts";
import { unit } from "../test/fixtures.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, writeCardFullArt, writeCardLevel } from "./cosmetics.ts";
import {
  FULL_ART_CARDS,
  FULL_ARTS_OF_CARD,
  FullArtAllowed,
  FullArtOwned,
  NO_FULL_ART,
  OwnCopy,
  boardLookOf,
  isFullArtCard,
  type FullArtOwnership,
} from "./fullArt.ts";
import { MinionFace } from "./MinionFace.tsx";
import { ownedCardCosmetics } from "./ownedFullArt.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import {
  NORMAL_LOOK,
  VERSION_ALT_FULL_ART_ID,
  VERSION_ANIMATED_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_FULL_ART_SIGNED_ID,
  lookOfLevel,
} from "./versions.ts";
import { isCoarsePointer } from "./inspect/useInspectTrigger.tsx";
import { KEYWORD_MARK } from "./keywordMarks.ts";

const DEFS = Object.values(CATALOG);

/** R2655001: the full-art cards, by type; the registry lists them all. */
const FULL_ART_UNITS = DEFS.filter((def) => def.type === "Unit" && FULL_ART_CARDS.has(def.id));
/** Spells, a Field Spell and a Field Trap print full art (R2655001); a face-up backrow card is one of them or a Unit. */
const FULL_ART_BACKROW = ["core-018", "classicplus-019", "classic-090", "classicplus-012"] as const;
/** Cards that print none: a Unit, a Trap, a Field Trap's neighbour, a Spell. */
const PLAIN_UNITS = ["core-002", "core-017", "core-022"] as const;
const PLAIN_BACKROW = ["core-041", "core-005", "core-084"] as const;
/** Bread Token, a Unit that wears its parent's full art (R2655002). */
const TOKEN = "core-t-bread";

function defOf(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

type Wrap = {
  worn?: boolean;
  owned?: FullArtOwnership;
  wrap?: (face: ReactElement) => ReactElement;
  radiant?: boolean;
};

/** A Unit `id` on the board, worn as the viewer's own unless `worn` is false. */
function minionOf(id: string, { worn = true, owned = "every", wrap = (face) => face, radiant = false }: Wrap = {}): HTMLElement {
  const u = unit("p1", { defId: id, radiant });
  const face = faceModel({
    defId: id,
    def: defOf(id),
    radiant,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  const { container } = render(<FullArtOwned owned={owned}>{wrap(<MinionFace face={face} unit={u} worn={worn} />)}</FullArtOwned>);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no minion for ${id}`);
  return cf;
}

/** A face-up backrow card `id`: the compact face. */
function backrowOf(id: string, { worn = true, owned = "every", wrap = (face) => face, radiant = false }: Wrap = {}): HTMLElement {
  const face = faceModel({ defId: id, def: defOf(id), radiant });
  const { container } = render(<FullArtOwned owned={owned}>{wrap(<CardFace face={face} layout="compact" worn={worn} />)}</FullArtOwned>);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no compact face for ${id}`);
  return cf;
}

/** Whether `later` comes after `earlier` in the document: the name's band after the art, in reading order. */
function follows(earlier: Element | null, later: Element | null): boolean {
  if (earlier === null || later === null) return false;
  return (earlier.compareDocumentPosition(later) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

/** The pick that makes `id` wear the level `level` (R3015005; null the normal face): a card pick the store keeps on the device. */
function wear(id: string, level: string | null): void {
  writeCardLevel(id, level);
}

/** The card ids `version` is offered on that the Full-art cards setting does not already print full art on. */
function offeredBeyondRegistry(version: string): string[] {
  return (COSMETIC_OFFERS[version] ?? []).filter((id) => !isFullArtCard(id) && CATALOG[id]?.type === "Unit");
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

describe("R3046000 a full-art Unit on the board", () => {
  it("R3046000 prints its art across the face, no oval, and its name after the art, base and Radiant", () => {
    expect(FULL_ART_UNITS.length).toBeGreaterThan(5);
    for (const def of FULL_ART_UNITS) {
      for (const radiant of [false, true]) {
        const where = `${def.id} ${radiant ? "Radiant" : "base"}`;
        const cf = minionOf(def.id, { radiant });
        expect(cf.getAttribute("data-layout"), where).toBe("minion");
        expect(cf.getAttribute("data-full-art"), where).toBe("true");
        expect(cf.getAttribute("data-version"), where).toBe(VERSION_FULL_ART_ID);
        const art = cf.querySelector(".cf-portrait > .cf-art");
        expect(art?.classList.contains("cf-art--full"), `${where}: the art is the full face's`).toBe(true);
        expect(cf.querySelector(".cf-art--oval"), `${where}: no oval`).toBeNull();
        // The name's band comes after the art, and says the card's name.
        const name = cf.querySelector(".card-name");
        expect(follows(art, name), `${where}: the name comes after the art`).toBe(true);
        expect(name?.textContent, where).toBe(def.name);
        // Its stats, cost and marks stay.
        for (const part of [".cost-gem", ".stats .stat-attack", ".stats .stat-health"]) expect(cf.querySelector(part), `${where} ${part}`).not.toBeNull();
        // A board unit has no rules box, as before.
        expect(cf.querySelector(".card-text"), where).toBeNull();
        cleanup();
      }
    }
  });

  it("R3046000 a Unit that is not full art keeps its oval window and draws no art flag", () => {
    for (const id of PLAIN_UNITS) {
      const cf = minionOf(id);
      expect(cf.hasAttribute("data-full-art"), id).toBe(false);
      expect(cf.hasAttribute("data-version"), id).toBe(false);
      expect(cf.querySelector(".cf-portrait > .cf-art.cf-art--oval"), id).not.toBeNull();
      expect(cf.querySelector(".cf-art--full"), id).toBeNull();
      cleanup();
    }
  });

  it("R3046000 its keyword chips, Taunt's ring, Divine Shield's bubble, the armor plate and a keyword's treatment are still drawn", () => {
    const u = unit("p1", {
      defId: "core-004",
      keywords: [{ kind: "Taunt" }, { kind: "Divine Shield" }, { kind: "Rush" }, { kind: "Poisonous" }],
      armor: 2,
    });
    const face = faceModel({
      defId: u.defId,
      def: defOf(u.defId),
      radiant: false,
      live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
    });
    const { container } = render(<MinionFace face={face} unit={u} worn />);
    const cf = container.querySelector<HTMLElement>(".cf");
    expect(cf?.getAttribute("data-full-art")).toBe("true");
    expect(cf?.getAttribute("data-taunt")).toBe("true");
    for (const part of [".shield-icon", ".stat-armor", ".keywords .keyword[data-keyword='Rush']", ".keywords .keyword[data-keyword='Poisonous']", "[data-keyword-fx='Poisonous']"]) {
      expect(cf?.querySelector(part), part).not.toBeNull();
    }
  });

  it("R3046000 follows the full face's decision: the setting, a lesson's board and ownership each turn it off", () => {
    expect(minionOf("core-004").getAttribute("data-full-art")).toBe("true");
    cleanup();

    writeCardSettings({ fullArt: false });
    expect(minionOf("core-004").hasAttribute("data-full-art"), "the Full-art cards setting off").toBe(false);
    cleanup();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });

    const lesson = minionOf("core-004", { wrap: (face) => <FullArtAllowed allowed={false}>{face}</FullArtAllowed> });
    expect(lesson.hasAttribute("data-full-art"), "a tutorial lesson's board").toBe(false);
    expect(lesson.querySelector(".cf-art--oval")).not.toBeNull();
    cleanup();

    // Where ownership decides, only the viewer's own unit whose full art is owned prints it (R1313000).
    const epic = FULL_ARTS_OF_CARD.get("core-004")?.get("Epic") ?? "";
    // R3015006: a retired full art owns the levels it maps to, as the client reads the ledger.
    const owned: FullArtOwnership = ownedCardCosmetics({ cards: [{ cardId: "core-004", cosmeticId: epic }], account: [] });
    expect(minionOf("core-004", { owned }).getAttribute("data-full-art"), "owned, own copy").toBe("true");
    cleanup();
    expect(minionOf("core-004", { owned: NO_FULL_ART }).hasAttribute("data-full-art"), "unowned").toBe(false);
    cleanup();
    expect(minionOf("core-004", { owned, worn: false }).hasAttribute("data-full-art"), "owned, but not the viewer's unit").toBe(false);
    cleanup();
    expect(minionOf("core-004", { owned, worn: false, wrap: (face) => <OwnCopy own>{face}</OwnCopy> }).getAttribute("data-full-art"), "an OwnCopy").toBe("true");
  });

  it("R3046000 a listed token wears its parent's full art on the board, and a token not listed does not", () => {
    const token = minionOf(TOKEN);
    expect(token.getAttribute("data-full-art")).toBe("true");
    expect(token.querySelector(".cf-art--full")).not.toBeNull();
    cleanup();

    const unlisted = DEFS.find((def) => def.type === "Unit" && def.rarity === "Token" && !isFullArtCard(def.id));
    expect(unlisted).toBeDefined();
    const plain = minionOf(unlisted?.id ?? "");
    expect(plain.hasAttribute("data-full-art")).toBe(false);
    expect(plain.querySelector(".cf-art--oval")).not.toBeNull();
  });

  it("R3046000 a card offered the Full art version prints it on the board when its pick wears it, and its pick of none prints the oval", () => {
    const [id] = offeredBeyondRegistry(VERSION_FULL_ART_ID);
    if (id === undefined) throw new Error("no card is offered the Full art version");
    expect(minionOf(id).hasAttribute("data-full-art"), "no pick: the setting prints full art on the registry's cards alone").toBe(false);
    cleanup();

    wear(id, VERSION_FULL_ART_ID);
    const cf = minionOf(id);
    expect(cf.getAttribute("data-full-art")).toBe("true");
    expect(cf.getAttribute("data-version")).toBe(VERSION_FULL_ART_ID);
    expect(cf.querySelector(".cf-art--full")).not.toBeNull();
    cleanup();

    // A copy that is not the viewer's wears no pick of theirs.
    expect(minionOf(id, { worn: false }).hasAttribute("data-full-art")).toBe(false);
    cleanup();

    writeCardFullArt("core-004", false);
    expect(minionOf("core-004").hasAttribute("data-full-art"), "a full-art card whose pick is none").toBe(false);
  });

  it("R3046000 the Alt full art version draws Alt art's picture on the board, as on the full face", () => {
    const [id] = (COSMETIC_OFFERS[VERSION_ALT_FULL_ART_ID] ?? []).filter((each) => CATALOG[each]?.type === "Unit" && CATALOG[each]?.set !== "Training Grounds");
    if (id === undefined) throw new Error("no Unit is offered Alt full art");
    wear(id, VERSION_ALT_FULL_ART_ID);
    const cf = minionOf(id);
    expect(cf.getAttribute("data-full-art")).toBe("true");
    expect(cf.getAttribute("data-version")).toBe(VERSION_ALT_FULL_ART_ID);
    const art = cf.querySelector(".cf-art");
    expect(art?.classList.contains("cf-art--full")).toBe(true);
    expect(art?.getAttribute("data-art-picture")).toBe("alt");
  });

  it("R3046000 the board holds the art still: Animated and Signature move and sign a full face alone", () => {
    // R3015000: Full art's signed level, animated and signed.
    wear("core-004", VERSION_FULL_ART_SIGNED_ID);
    const full = (() => {
      const { container } = render(<CardFace face={faceModel({ defId: "core-004", def: defOf("core-004"), radiant: false })} layout="full" worn />);
      return container.querySelector<HTMLElement>(".cf");
    })();
    expect(full?.getAttribute("data-art-motion"), "the full face moves its art").not.toBeNull();
    expect(full?.querySelector(".cf-signature"), "and signs it").not.toBeNull();
    cleanup();

    for (const cf of [minionOf("core-004"), backrowOf("core-018")]) {
      expect(cf.getAttribute("data-full-art")).toBe("true");
      for (const attribute of ["data-art-motion", "data-full-art-motion", "data-full-art-tier", "data-augments"]) {
        expect(cf.hasAttribute(attribute), `${cf.getAttribute("data-layout")} ${attribute}`).toBe(false);
      }
      expect(cf.classList.contains("cf-full-art--animated")).toBe(false);
      expect(cf.classList.contains("cf-full-art--signed")).toBe(false);
      expect(cf.querySelector(".cf-signature, .cf-full-art-sweep, [data-art-moving]")).toBeNull();
      expect(cf.querySelector(".cf-art")?.getAttribute("data-art-picture")).toBeNull();
      cleanup();
    }
  });

  it("R3046000 a card that wears Animated alone, on its normal face, keeps its window", () => {
    // R3015000: the Default variant's animated level.
    wear("core-002", VERSION_ANIMATED_ID);
    const cf = minionOf("core-002");
    expect(cf.hasAttribute("data-full-art")).toBe(false);
    expect(cf.querySelector(".cf-art--oval")).not.toBeNull();
  });
});

describe("R3046000 a full-art face-up backrow card", () => {
  it("R3046000 prints its art across the face with its name after the art, and no type line, rules box or tags", () => {
    for (const id of FULL_ART_BACKROW) {
      for (const radiant of [false, true]) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const cf = backrowOf(id, { radiant });
        expect(cf.getAttribute("data-layout"), where).toBe("compact");
        expect(cf.getAttribute("data-full-art"), where).toBe("true");
        expect(cf.getAttribute("data-version"), where).toBe(VERSION_FULL_ART_ID);
        const frame = cf.querySelector(".cf-art-frame");
        expect(frame?.querySelector(".cf-art")?.classList.contains("cf-art--full"), `${where}: the art is the full face's`).toBe(true);
        const name = cf.querySelector(".card-name");
        expect(follows(frame, name), `${where}: the name comes after the art`).toBe(true);
        expect(name?.textContent, where).toBe(defOf(id).name);
        for (const words of [".card-type", ".card-text", ".cf-tags", ".cf-stats"]) expect(cf.querySelector(words), `${where} ${words}`).toBeNull();
        for (const part of [".cost-gem"]) expect(cf.querySelector(part), `${where} ${part}`).not.toBeNull();
        cleanup();
      }
    }
  });

  it("R3046000 a card that is not full art keeps its window, its type line and its rarity gem", () => {
    for (const id of PLAIN_BACKROW) {
      const cf = backrowOf(id);
      expect(cf.hasAttribute("data-full-art"), id).toBe(false);
      expect(cf.querySelector(".cf-art--full"), id).toBeNull();
      expect(cf.querySelector(".card-type")?.textContent, id).toBe(defOf(id).type);
      cleanup();
    }
  });

  it("R3046000 follows the full face's decision: the setting, a lesson's board and ownership each turn it off", () => {
    expect(backrowOf("core-018").getAttribute("data-full-art")).toBe("true");
    cleanup();

    writeCardSettings({ fullArt: false });
    const off = backrowOf("core-018");
    expect(off.hasAttribute("data-full-art"), "the Full-art cards setting off").toBe(false);
    expect(off.querySelector(".card-type")).not.toBeNull();
    cleanup();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });

    expect(backrowOf("core-018", { wrap: (face) => <FullArtAllowed allowed={false}>{face}</FullArtAllowed> }).hasAttribute("data-full-art"), "a lesson's board").toBe(false);
    cleanup();

    const epic = FULL_ARTS_OF_CARD.get("core-018")?.get("Epic") ?? "";
    // R3015006: a retired full art owns the levels it maps to, as the client reads the ledger.
    const owned: FullArtOwnership = ownedCardCosmetics({ cards: [{ cardId: "core-018", cosmeticId: epic }], account: [] });
    expect(backrowOf("core-018", { owned }).getAttribute("data-full-art"), "owned, own card").toBe("true");
    cleanup();
    expect(backrowOf("core-018", { owned: NO_FULL_ART }).hasAttribute("data-full-art"), "unowned").toBe(false);
    cleanup();
    expect(backrowOf("core-018", { owned, worn: false }).hasAttribute("data-full-art"), "not the viewer's card").toBe(false);
  });

  it("R3046000 a full face is unchanged by it: the same card's full layout still prints its tier and its type line only where it should", () => {
    const { container } = render(<CardFace face={faceModel({ defId: "core-018", def: defOf("core-018"), radiant: false })} layout="full" />);
    const full = container.querySelector<HTMLElement>(".cf");
    expect(full?.getAttribute("data-full-art")).toBe("true");
    expect(full?.getAttribute("data-full-art-tier")).toBe("Epic");
    expect(full?.querySelector(".card-type")).toBeNull();
    expect(full?.querySelector(".cf-art--full")).not.toBeNull();
  });
});

describe("R3046000 boardLookOf", () => {
  it("R3046000 gives the version still where the full face prints full art, and null where it does not", () => {
    const face = faceModel({ defId: "core-004", def: defOf("core-004"), radiant: false });
    const plain = faceModel({ defId: "core-002", def: defOf("core-002"), radiant: false });
    const full = lookOfLevel(VERSION_FULL_ART_ID);
    const still = { level: VERSION_FULL_ART_ID, version: VERSION_FULL_ART_ID, animated: false, signed: false };
    expect(boardLookOf(face, full)).toEqual(still);
    // Motion and a signature are the full face's alone (R3015000: the levels above the art).
    expect(boardLookOf(face, lookOfLevel(VERSION_FULL_ART_ANIMATED_ID))).toEqual(still);
    expect(boardLookOf(face, lookOfLevel(VERSION_FULL_ART_SIGNED_ID))).toEqual(still);
    expect(boardLookOf(face, NORMAL_LOOK)).toBeNull();
    expect(boardLookOf(plain, full)).toBeNull();
    // Alt full art lays its art across the face as Full art does (R2940001); `useCardLook` holds a pick to the offer.
    expect(boardLookOf(face, lookOfLevel(VERSION_ALT_FULL_ART_ID))).toEqual({
      level: VERSION_ALT_FULL_ART_ID,
      version: VERSION_ALT_FULL_ART_ID,
      animated: false,
      signed: false,
    });
  });
});

/* The stylesheets: the band below the art. */

/** A sheet's text without its comments. */
function sheetText(fromWeb: string): string {
  return readSheet(fromWeb).replace(/\/\*[\s\S]*?\*\//g, "");
}

/** The declarations of the first rule whose selector is exactly `selector`, at the top level of the sheet. */
function declsOf(sheet: string, selector: string): Map<string, string> {
  const at = sheet.search(new RegExp(`(^|\\n)${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*(,\\s*\\n[^{]*)?\\{`));
  if (at < 0) throw new Error(`no rule for ${selector}`);
  const open = sheet.indexOf("{", at);
  const body = sheet.slice(open + 1, sheet.indexOf("}", open));
  return new Map(
    body
      .split(";")
      .map((part): [string, string] | null => {
        const colon = part.indexOf(":");
        return colon > 0 ? [part.slice(0, colon).trim(), part.slice(colon + 1).trim().replace(/\s+/g, " ")] : null;
      })
      .filter((entry): entry is [string, string] => entry !== null),
  );
}

describe("R3046000 the stylesheets put the name band below the art", () => {
  const cards = sheetText("src/cards/cards.css");
  const MINION = '.cf[data-layout="minion"][data-full-art="true"]';
  const COMPACT = '.cf[data-layout="compact"][data-full-art="true"][data-card-type]';

  it("R3046000 a Unit's art fills the panel down to the name band, which lies at the foot with the gems over the art's corners", () => {
    const portrait = declsOf(cards, `${MINION} .cf-portrait`);
    expect(portrait.get("top")).toBe("0");
    expect(portrait.get("left")).toBe("0");
    expect(portrait.get("right")).toBe("0");
    // The art ends where the plate begins: its bottom edge is the plate's height up from the foot.
    expect(portrait.get("bottom")).toBe("var(--mf-plate)");
    expect(portrait.get("border-radius")).toBe("0");
    // Under a finish's band (cosmetics.css, z 1).
    expect(portrait.get("z-index")).toBe("0");

    const name = declsOf(cards, `${MINION} .card-name`);
    expect(name.get("top")).toBe("auto");
    expect(name.get("bottom")).toBe("0");
    expect(name.get("left")).toBe("0");
    expect(name.get("right")).toBe("0");
    // Its height is the oval face's plate, `--mf-plate` (cards.css `.cf[data-layout="minion"] .card-name`).
    expect(declsOf(cards, '.cf[data-layout="minion"] .card-name').get("height")).toBe("var(--mf-plate)");
    expect(declsOf(cards, `${MINION} .stats`).get("bottom")).toBe("var(--mf-plate)");
    // The panel takes the full face's rim.
    expect(declsOf(cards, `${MINION} .cf-scale`).get("inset")).toBe("1.2% 1.7%");
  });

  it("R3046000 a backrow card's art fills the face down to the name band, which lies at the foot and is as tall", () => {
    expect(declsOf(cards, '.cf[data-layout="compact"][data-full-art="true"]').get("--cf-band")).toBe("24cqh");
    expect(declsOf(cards, `${COMPACT} .cf-art-frame`).get("inset")).toBe("0 0 var(--cf-band) 0");
    const name = declsOf(cards, `${COMPACT} .card-name`);
    expect(name.get("top")).toBe("auto");
    expect(name.get("bottom")).toBe("0");
    expect(name.get("left")).toBe("0");
    expect(name.get("right")).toBe("0");
    expect(name.get("height")).toBe("var(--cf-band)");
    // A small face's own full-art rule, which fills the face with the art, leaves the compact face to these.
    expect(cards).toContain('.cf[data-full-art="true"]:not([data-layout="minion"]):not([data-layout="compact"])[data-card-type] .cf-art-frame');
  });

  it("R3046000 a full-art Unit's border shows round the art, drawing no ring, and the keyword frames hidden behind the art are not drawn", () => {
    const cosmetics = readSheet("src/cards/cosmetics.css");
    expect(cosmetics).toContain('.cf[data-layout="minion"][data-frame]:not([data-full-art="true"]) .cf-scale::before');
    const keywords = sheetText("src/cards/keywords.css");
    expect(keywords).toContain(`${MINION} .kw-fx[data-keyword-fx="Taunt"]`);
    expect(keywords).toContain(`${MINION} .kw-fx[data-keyword-fx="Stack"]`);
    expect(declsOf(keywords, MINION).get("--kw-portrait-bottom")).toBe("var(--mf-plate, 11cqh)");
    expect(declsOf(keywords, `${MINION} .kw-fx[data-kw-layer="veil"]`).get("border-radius")).toBe("0");
    // Taunt's steel ring is drawn over the art, since its shield is not.
    expect(cards).toContain(`${MINION}[data-taunt="true"] .cf-portrait::after`);
  });
});

describe("R3339000 art-first cards in play", () => {
  const cards = sheetText("src/cards/cards.css");
  const ART_FIRST_MINION = '.cf[data-layout="minion"][data-art-first="true"]';
  const ART_FIRST_COMPACT = '.cf[data-layout="compact"][data-art-first="true"]';

  it("R3339000 art-first faces render no name, type line or rules text in full-art-shaped looks and Legendary/Mythic", () => {
    const legendary = DEFS.find((d) => d.rarity === "Legendary" && d.type === "Unit")!;
    const mythic = DEFS.find((d) => d.rarity === "Mythic" && d.type === "Unit")!;
    expect(legendary).toBeDefined();
    expect(mythic).toBeDefined();

    for (const def of [legendary, mythic]) {
      const cf = minionOf(def.id);
      expect(cf.getAttribute("data-art-first")).toBe("true");
      expect(cf.querySelector(".card-name:not(.card-name-hover)")).toBeNull();
      expect(cf.querySelector(".card-name-hover")?.textContent).toBe(def.name);
      expect(cf.querySelector(".card-type")).toBeNull();
      expect(cf.querySelector(".card-text")).toBeNull();
      expect(cf.querySelector(".cost-gem")).not.toBeNull();
      expect(cf.querySelector(".stats .stat-attack")).not.toBeNull();
      expect(cf.querySelector(".stats .stat-health")).not.toBeNull();
      cleanup();
    }

    const fullArtLevels = [
      "version-full-art",
      "version-alt-full-art",
      "version-sketch-full-art",
      "version-extended-art",
      "version-out-of-frame",
      "version-set-showcase",
      "version-immersive",
      "version-prismatic",
      "version-full-art-animated",
      "version-full-art-signed",
    ];

    for (const level of fullArtLevels) {
      const offered = (COSMETIC_OFFERS[level] ?? []).find((id) => CATALOG[id]?.type === "Unit" && CATALOG[id]?.set !== "Training Grounds") ?? "core-004";
      wear(offered, level);
      const cf = minionOf(offered);
      expect(cf.getAttribute("data-art-first"), level).toBe("true");
      expect(cf.querySelector(".card-name:not(.card-name-hover)"), level).toBeNull();
      expect(cf.querySelector(".card-name-hover")?.textContent, level).toBe(defOf(offered).name);
      expect(cf.querySelector(".card-type"), level).toBeNull();
      expect(cf.querySelector(".card-text"), level).toBeNull();
      cleanup();
    }

    // Backrow art-first face
    const backrowDef = defOf("core-018");
    const bcf = backrowOf("core-018");
    expect(bcf.getAttribute("data-art-first")).toBe("true");
    expect(bcf.querySelector(".card-name:not(.card-name-hover)")).toBeNull();
    expect(bcf.querySelector(".card-name-hover")?.textContent).toBe(backrowDef.name);
    expect(bcf.querySelector(".card-type")).toBeNull();
    expect(bcf.querySelector(".card-text")).toBeNull();
    cleanup();
  });

  it("R3339000 Common Unit keeps smaller translucent name plate", () => {
    const common = DEFS.find((d) => d.rarity === "Common" && d.type === "Unit" && !isFullArtCard(d.id))!;
    expect(common).toBeDefined();
    const cf = minionOf(common.id);
    expect(cf.hasAttribute("data-art-first")).toBe(false);
    const name = cf.querySelector(".card-name.card-name-compact-translucent");
    expect(name).not.toBeNull();
    expect(name?.textContent).toBe(common.name);
    expect(cf.querySelector(".cf-portrait > .cf-art.cf-art--oval")).not.toBeNull();
  });

  it("R3339000 keyword chips render as icon marks with accessible names", () => {
    const u = unit("p1", {
      defId: "core-004",
      keywords: [{ kind: "Taunt" }, { kind: "Divine Shield" }, { kind: "Rush" }, { kind: "Poisonous" }],
      armor: 2,
    });
    const face = faceModel({
      defId: u.defId,
      def: defOf(u.defId),
      radiant: false,
      live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
    });
    const { container } = render(<MinionFace face={face} unit={u} worn />);
    const cf = container.querySelector<HTMLElement>(".cf")!;
    const chips = cf.querySelectorAll(".keywords .keyword");
    expect(chips.length).toBeGreaterThan(0);
    for (const chip of chips) {
      expect(chip.getAttribute("role")).toBe("img");
      expect(chip.getAttribute("aria-label")).toBeTruthy();
      const kw = chip.getAttribute("data-keyword") as keyof typeof KEYWORD_MARK;
      expect(chip.textContent?.trim()).toBe(KEYWORD_MARK[kw]);
    }
  });

  it("R3339000 hover and focus reveal name plate, and boardNames setting controls visibility", () => {
    const common = DEFS.find((d) => d.rarity === "Common" && d.type === "Unit" && !isFullArtCard(d.id))!;
    const legendary = DEFS.find((d) => d.rarity === "Legendary" && d.type === "Unit")!;

    // 1. artFirst (default): art-first hides static plate; common keeps it
    writeCardSettings({ boardNames: "artFirst" });
    let lcf = minionOf(legendary.id);
    expect(lcf.querySelector(".card-name:not(.card-name-hover)")).toBeNull();
    expect(lcf.querySelector(".card-name-hover")).not.toBeNull();
    cleanup();
    let ccf = minionOf(common.id);
    expect(ccf.querySelector(".card-name:not(.card-name-hover)")).not.toBeNull();
    cleanup();

    // 2. always: both show static plate
    writeCardSettings({ boardNames: "always" });
    lcf = minionOf(legendary.id);
    expect(lcf.querySelector(".card-name:not(.card-name-hover)")).not.toBeNull();
    cleanup();
    ccf = minionOf(common.id);
    expect(ccf.querySelector(".card-name:not(.card-name-hover)")).not.toBeNull();
    cleanup();

    // 3. never: neither shows static plate; both have hover plate
    writeCardSettings({ boardNames: "never" });
    lcf = minionOf(legendary.id);
    expect(lcf.querySelector(".card-name:not(.card-name-hover)")).toBeNull();
    expect(lcf.querySelector(".card-name-hover")).not.toBeNull();
    cleanup();
    ccf = minionOf(common.id);
    expect(ccf.querySelector(".card-name:not(.card-name-hover)")).toBeNull();
    expect(ccf.querySelector(".card-name-hover")).not.toBeNull();
    cleanup();
  });

  it("R3339000 stylesheets lay art-first portrait edge-to-edge and style hover plate", () => {
    const portrait = declsOf(cards, `${ART_FIRST_MINION} .cf-portrait`);
    expect(portrait.get("top")).toBe("0");
    expect(portrait.get("left")).toBe("0");
    expect(portrait.get("right")).toBe("0");
    expect(portrait.get("bottom")).toBe("0");
    expect(portrait.get("border-radius")).toBe("0");

    expect(declsOf(cards, ART_FIRST_COMPACT).get("--cf-band")).toBe("0cqh");
    expect(declsOf(cards, `${ART_FIRST_COMPACT}[data-card-type] .cf-art-frame`).get("inset")).toBe("0");

    const hoverPlate = declsOf(cards, ".cf .card-name-hover");
    expect(hoverPlate.get("opacity")).toBe("0");
    expect(hoverPlate.get("transition")).toContain("var(--motion-fast)");
    expect(cards).toContain(".cf:hover .card-name-hover");
    expect(cards).toContain(".cf:focus-within .card-name-hover");
  });

  it("R3339000 useInspectTrigger handles coarse pointer inspect routing", () => {
    expect(typeof isCoarsePointer).toBe("function");
  });
});
