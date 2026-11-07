// R3343001 (#3343): the default looks' faces. Every Mythic's Default, a token printed Mythic among them, draws Full art,
// the picture across the whole face and no words; every Legendary's, a token printed Legendary among them, Extended art,
// its picture past its window behind the translucent frame and rules box, its words kept; a full-art card (R2655001)
// still wears Full art by the setting, and the Glitch its blank face. The Default levels name no level of their own
// (`data-level`), and the Full-art cards setting off keeps a Mythic's normal face. Prismatic is a full art at both its
// levels. R2725004 holds under each: a Radiant face keeps its Radiant marks, a base face shows none. On the board a
// Mythic Unit prints full art and a Legendary keeps its window (R3046000). Each pull request draws every Mythic and
// Legendary and a sample of the rest, two of each rarity of each set; the daily run draws every other card. The pixels
// are e2e's cypress/component/default-looks.cy.tsx.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { unit } from "../test/fixtures.ts";
import { daily } from "../test/daily.ts";
import { legacyText } from "../test/faceData.ts";
import { COSMETIC_OFFERS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests, writeCardLevel } from "./cosmetics.ts";
import { FullArtOwned, isFullArtCard } from "./fullArt.ts";
import { isGlitch } from "./glitch.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { VERSION_EXTENDED_ART_ID, VERSION_FULL_ART_ID, VERSION_PRISMATIC_ID, defaultLookOf } from "./versions.ts";

/** The cards a test draws at once, so no one test runs long. */
const CHUNK = 16;

const IDS = Object.keys(CATALOG).sort();

/** R3343000: the rarity a card's face prints: a token's printed one, any other card's own. */
function faceRarityOf(id: string): string | null {
  const def = CATALOG[id];
  if (def === undefined) return null;
  return def.rarity === "Token" ? (def.printedRarity ?? null) : def.rarity;
}

const MYTHICS = IDS.filter((id) => faceRarityOf(id) === "Mythic");
const LEGENDARIES = IDS.filter((id) => faceRarityOf(id) === "Legendary");
const BELOW = IDS.filter((id) => !MYTHICS.includes(id) && !LEGENDARIES.includes(id));

/** Two cards of each rarity of each set below Legendary, the pull request's sample; the daily run takes the rest. */
const SAMPLE: readonly string[] = (() => {
  const taken = new Map<string, number>();
  return BELOW.filter((id) => {
    const def = CATALOG[id];
    const key = `${def?.set ?? ""}/${def?.rarity ?? ""}`;
    const count = taken.get(key) ?? 0;
    taken.set(key, count + 1);
    return count < 2;
  });
})();
const REST = BELOW.filter((id) => !SAMPLE.includes(id));

/** `ids` in runs of `CHUNK`. */
function chunks(ids: readonly string[]): string[][] {
  const out: string[][] = [];
  for (let at = 0; at < ids.length; at += CHUNK) out.push(ids.slice(at, at + CHUNK));
  return out;
}

function defOf(id: string) {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

/** The viewer's own full face of `id`. */
function faceIn(id: string, radiant: boolean): HTMLElement {
  const { container } = render(<CardFace face={faceModel({ defId: id, def: defOf(id), radiant })} layout="full" worn />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error(`no face for ${id}`);
  return cf;
}

/** The art window: the art frame's own picture. */
function windowOf(cf: HTMLElement): HTMLElement | null {
  return cf.querySelector<HTMLElement>(".cf-art-frame > .cf-art:not(.cf-art--cutout)");
}

/** R2725004: a Radiant face keeps its marks and paints its picture Radiant; a base face shows neither. */
function keepsRadiantMarks(cf: HTMLElement, radiant: boolean, where: string, glitch = false): void {
  expect(cf.getAttribute("data-radiant-face"), where).toBe(radiant ? "true" : null);
  if (!glitch) expect(windowOf(cf)?.getAttribute("data-art-variant"), where).toBe(radiant ? "radiant" : "base");
}

/** What a card below Legendary draws, as before #3343: a full-art card its Full art by the setting, any other its normal face. */
function drawsAsBefore(ids: readonly string[]): void {
  for (const id of ids) {
    for (const radiant of [false, true]) {
      const where = `${id} ${radiant ? "Radiant" : "base"}`;
      expect(defaultLookOf(id), where).toBeNull();
      const cf = faceIn(id, radiant);
      if (isFullArtCard(id)) {
        expect(cf.getAttribute("data-full-art"), where).toBe("true");
        expect(cf.getAttribute("data-version"), where).toBe(VERSION_FULL_ART_ID);
        expect(cf.getAttribute("data-level"), where).toBe(VERSION_FULL_ART_ID);
      } else {
        expect(cf.getAttribute("data-full-art"), where).toBeNull();
        expect(cf.getAttribute("data-version"), where).toBeNull();
        expect(cf.getAttribute("data-level"), where).toBeNull();
        expect(cf.querySelector(".card-text"), where).not.toBeNull();
      }
      keepsRadiantMarks(cf, radiant, where, isGlitch(id));
      cleanup();
    }
  }
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

describe("R3343001 a Mythic's Default is full art", () => {
  it("R3343001 the catalog holds Mythics, tokens printed Mythic among them", () => {
    expect(MYTHICS.length).toBeGreaterThan(CHUNK);
    expect(MYTHICS.some((id) => CATALOG[id]?.rarity === "Token")).toBe(true);
  });

  for (const [at, ids] of chunks(MYTHICS).entries()) {
    it(`R3343001 Mythics ${String(at + 1)}: the Default draws Full art, no level named and no words, base and Radiant`, () => {
      for (const id of ids) {
        for (const radiant of [false, true]) {
          const where = `${id} ${radiant ? "Radiant" : "base"}`;
          const cf = faceIn(id, radiant);
          expect(cf.getAttribute("data-full-art"), where).toBe("true");
          expect(cf.getAttribute("data-version"), where).toBe(VERSION_FULL_ART_ID);
          expect(cf.getAttribute("data-level"), where).toBeNull();
          expect(cf.querySelector(".card-text"), where).toBeNull();
          expect(windowOf(cf)?.classList.contains("cf-art--full"), where).toBe(true);
          keepsRadiantMarks(cf, radiant, where);
          cleanup();
        }
      }
    });
  }

  it("R3343001 with the Full-art cards setting off a Mythic shows its normal face and its words", () => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, fullArt: false });
    for (const id of MYTHICS.slice(0, CHUNK)) {
      const cf = faceIn(id, false);
      expect(cf.getAttribute("data-full-art"), id).toBeNull();
      expect(cf.getAttribute("data-version"), id).toBeNull();
      expect(cf.querySelector(".card-text"), id).not.toBeNull();
      cleanup();
    }
  });
});

describe("R3343001 a Legendary's Default is Extended art", () => {
  it("R3343001 the catalog holds Legendaries, tokens printed Legendary among them", () => {
    expect(LEGENDARIES.length).toBeGreaterThan(CHUNK);
    expect(LEGENDARIES.some((id) => CATALOG[id]?.rarity === "Token")).toBe(true);
  });

  for (const [at, ids] of chunks(LEGENDARIES).entries()) {
    it(`R3343001 Legendaries ${String(at + 1)}: the Default draws Extended art with its words, a full-art card its Full art, base and Radiant`, () => {
      for (const id of ids) {
        for (const radiant of [false, true]) {
          const where = `${id} ${radiant ? "Radiant" : "base"}`;
          const cf = faceIn(id, radiant);
          if (isFullArtCard(id)) {
            // R2725000: the setting's Full art, its own level, worn over the Default's look.
            expect(cf.getAttribute("data-full-art"), where).toBe("true");
            expect(cf.getAttribute("data-version"), where).toBe(VERSION_FULL_ART_ID);
          } else {
            expect(cf.getAttribute("data-full-art"), where).toBeNull();
            expect(cf.getAttribute("data-version"), where).toBe(VERSION_EXTENDED_ART_ID);
            expect(cf.getAttribute("data-level"), where).toBeNull();
            expect(cf.querySelector(".card-text"), where).not.toBeNull();
            expect(windowOf(cf)?.classList.contains("cf-art--full"), where).toBe(true);
            // R277's marks wherever its words show: on a Radiant face whose words differ, never on a base face.
            const marked = legacyText(faceModel({ defId: id, def: defOf(id), radiant: true }).text).marks.length > 0;
            const marks = cf.querySelectorAll('.cf-mark[data-mark="radiant"]').length;
            if (radiant) expect(marks > 0, where).toBe(marked);
            else expect(marks, where).toBe(0);
          }
          keepsRadiantMarks(cf, radiant, where);
          cleanup();
        }
      }
    });
  }

  it("R3343001 with the Full-art cards setting off a Legendary keeps its Extended art", () => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS, fullArt: false });
    for (const id of LEGENDARIES.slice(0, CHUNK)) {
      const cf = faceIn(id, false);
      expect(cf.getAttribute("data-full-art"), id).toBeNull();
      expect(cf.getAttribute("data-version"), id).toBe(VERSION_EXTENDED_ART_ID);
      expect(cf.querySelector(".card-text"), id).not.toBeNull();
      cleanup();
    }
  });
});

describe("R3343001 every card below Legendary is unchanged", () => {
  it("R3343001 a sample, two of each rarity of each set", () => {
    expect(SAMPLE.length).toBeGreaterThan(CHUNK);
    drawsAsBefore(SAMPLE);
  });

  for (const ids of chunks(REST)) {
    daily(`R3343001 cards ${ids[0] ?? ""} to ${ids.at(-1) ?? ""} below Legendary draw as before`, () => {
      drawsAsBefore(ids);
    });
  }
});

describe("R3343001 Prismatic is a full art at both its levels", () => {
  const offered = COSMETIC_OFFERS[VERSION_PRISMATIC_ID] ?? [];
  for (const [at, ids] of chunks(offered).entries()) {
    it(`R3343001 Prismatic ${String(at + 1)}: chrome across the whole face, no words, at both levels, base and Radiant`, () => {
      for (const id of ids) {
        for (const level of [VERSION_PRISMATIC_ID, `${VERSION_PRISMATIC_ID}-animated`]) {
          if (!(COSMETIC_OFFERS[level] ?? []).includes(id)) continue;
          for (const radiant of [false, true]) {
            const where = `${id} ${level} ${radiant ? "Radiant" : "base"}`;
            writeCardLevel(id, level);
            const cf = faceIn(id, radiant);
            expect(cf.getAttribute("data-full-art"), where).toBe("true");
            expect(cf.getAttribute("data-version"), where).toBe(VERSION_PRISMATIC_ID);
            expect(cf.getAttribute("data-level"), where).toBe(level);
            expect(cf.querySelector(".card-text"), where).toBeNull();
            expect(windowOf(cf)?.getAttribute("data-art-picture"), where).toBe("prismatic");
            keepsRadiantMarks(cf, radiant, where);
            cleanup();
          }
        }
      }
    });
  }
});

describe("R3343001 on the board", () => {
  /** A Unit `id` on the board, the viewer's own. */
  function minionOf(id: string, radiant = false): HTMLElement {
    const u = unit("p1", { defId: id, radiant });
    const face = faceModel({
      defId: id,
      def: defOf(id),
      radiant,
      live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
    });
    const { container } = render(
      <FullArtOwned owned="every">
        <MinionFace face={face} unit={u} worn />
      </FullArtOwned>,
    );
    const cf = container.querySelector<HTMLElement>(".cf");
    if (cf === null) throw new Error(`no minion for ${id}`);
    return cf;
  }

  it("R3343001 a Mythic Unit prints full art, a Legendary Unit keeps its window", () => {
    const mythic = MYTHICS.find((id) => CATALOG[id]?.type === "Unit" && CATALOG[id]?.rarity !== "Token");
    const legendary = LEGENDARIES.find((id) => CATALOG[id]?.type === "Unit" && CATALOG[id]?.rarity !== "Token" && !isFullArtCard(id));
    if (mythic === undefined || legendary === undefined) throw new Error("no Mythic or Legendary Unit");
    for (const radiant of [false, true]) {
      const full = minionOf(mythic, radiant);
      expect(full.getAttribute("data-full-art"), mythic).toBe("true");
      expect(full.getAttribute("data-version"), mythic).toBe(VERSION_FULL_ART_ID);
      expect(full.querySelector(".cf-portrait > .cf-art")?.classList.contains("cf-art--full"), mythic).toBe(true);
      cleanup();
      const windowed = minionOf(legendary, radiant);
      expect(windowed.getAttribute("data-full-art"), legendary).toBeNull();
      expect(windowed.getAttribute("data-version"), legendary).toBeNull();
      expect(windowed.querySelector(".cf-art--oval"), legendary).not.toBeNull();
      cleanup();
    }
  });
});
