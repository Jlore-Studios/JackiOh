// R1017001: the hero portraits' cards print full art and no words, as Hearthstone's Signature cards
// do; no other card does, a full face is the only layout that does, and the Full-art cards setting
// turns it off. The pixels are cards.css's ("full art"), seen in the Almanac.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { PORTRAITS } from "@jackioh/shared";

import { CardFace, faceText } from "./CardFace.tsx";
import { FULL_ART_NAMES, FullArtAllowed, isFullArt, showsFullArt } from "./fullArt.ts";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { INSPECT_EXTENDED, INSPECT_FACE } from "./inspect/testids.ts";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, readCardSettings, writeCardSettings } from "./settings.ts";

const ANCHOR = { left: 600, top: 200, width: 100, height: 140 };

const DEFS = Object.values(CATALOG);

function faceOf(id: string, radiant: boolean, layout: "full" | "compact" = "full"): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout={layout} />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error("no face");
  return cf;
}

const HEROES = DEFS.filter((def) => FULL_ART_NAMES.has(def.name));

afterEach(() => {
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  cleanup();
});

describe("R1017001 full-art faces", () => {
  it("R1017001 the full-art cards are the hero portraits' cards, each one in the catalog", () => {
    const names = Object.values(PORTRAITS).map((portrait) => portrait.cardName);
    expect([...FULL_ART_NAMES].sort()).toEqual([...new Set(names)].sort());
    for (const name of names) expect(DEFS.some((def) => def.name === name), name).toBe(true);
    expect(HEROES).toHaveLength(FULL_ART_NAMES.size);
    expect(isFullArt("Reach the Summit")).toBe(false);
  });

  it("R1017001 a hero's full face, base or Radiant, is its art, cost, name, rarity and a Unit's stats, and no words", () => {
    for (const def of HEROES) {
      for (const radiant of [false, true]) {
        const cf = faceOf(def.id, radiant);
        const where = `${def.id} ${radiant ? "radiant" : "base"}`;
        expect(cf.getAttribute("data-full-art"), where).toBe("true");
        expect(cf.querySelector(".cf-art-frame > .cf-art")?.classList.contains("cf-art--full"), where).toBe(true);
        for (const words of [".card-text", ".cf-tags", ".card-type"]) expect(cf.querySelector(words), `${where} ${words}`).toBeNull();
        expect(cf.querySelector(".card-name")?.textContent, where).toBe(def.name);
        expect(cf.querySelector(".cost-gem"), where).not.toBeNull();
        if (def.type === "Unit") expect(cf.querySelector(".cf-stats"), where).not.toBeNull();
        // The face prints less than its text (Mr. Vanilla has none): the preview and the detail view carry it whole.
        const model = faceModel({ defId: def.id, def, radiant });
        expect(faceText(model), where).toEqual({ shown: "", short: null, less: model.text.full !== "" });
        cleanup();
      }
    }
  });

  it("R1017001 other cards keep their windows and their words", () => {
    const others = (["Unit", "Spell", "Field Spell", "Trap", "Field Trap"] as const).flatMap((type) =>
      DEFS.filter((def) => def.type === type && !FULL_ART_NAMES.has(def.name)).slice(0, 2),
    );
    for (const def of others) {
      const cf = faceOf(def.id, false);
      expect(cf.hasAttribute("data-full-art"), def.id).toBe(false);
      expect(cf.querySelector(".cf-art--full"), def.id).toBeNull();
      expect(cf.querySelector(".card-type"), def.id).not.toBeNull();
      cleanup();
    }
  });

  it("R1017001 a compact face (a face-up backrow card) keeps its window", () => {
    const def = HEROES[0];
    if (def === undefined) throw new Error("no hero card");
    expect(faceOf(def.id, false, "compact").hasAttribute("data-full-art")).toBe(false);
  });

  it("R1017001 with the Full-art cards setting off, a hero's card prints its text like any card", () => {
    writeCardSettings({ fullArt: false });
    for (const def of HEROES) {
      const cf = faceOf(def.id, false);
      const model = faceModel({ defId: def.id, def, radiant: false });
      expect(cf.hasAttribute("data-full-art"), def.id).toBe(false);
      expect(cf.querySelector(".cf-art--full"), def.id).toBeNull();
      expect(cf.querySelector(".card-type"), def.id).not.toBeNull();
      expect(showsFullArt(model), def.id).toBe(false);
      if (model.text.full !== "") expect(cf.querySelector(".cf-text-base")?.textContent, def.id).toBe(faceText(model).shown);
      cleanup();
    }
  });
});

// R1108000: a tutorial lesson's board (practice.tsx) sits below a FullArtAllowed of false, and the
// faces and inspect overlays below it print no full art whatever the setting says.
describe("R1108000 no full art on a tutorial lesson's board", () => {
  it("R1108000 below FullArtAllowed false a hero's full face prints its text, the Full-art cards setting on", () => {
    expect(readCardSettings().fullArt).toBe(true);
    for (const def of HEROES) {
      const model = faceModel({ defId: def.id, def, radiant: false });
      const { container } = render(
        <FullArtAllowed allowed={false}>
          <CardFace face={model} layout="full" />
        </FullArtAllowed>,
      );
      const cf = container.querySelector<HTMLElement>(".cf");
      if (cf === null) throw new Error(`no face for ${def.id}`);
      expect(cf.hasAttribute("data-full-art"), def.id).toBe(false);
      expect(cf.querySelector(".cf-art--full"), def.id).toBeNull();
      expect(cf.querySelector(".card-type"), def.id).not.toBeNull();
      if (model.text.full !== "") expect(cf.querySelector(".cf-text-base")?.textContent, def.id).toBe(faceText(model, undefined, false).shown);
      cleanup();
    }
  });

  it("R1108000 below FullArtAllowed true a hero's face keeps its full art", () => {
    const def = HEROES[0];
    if (def === undefined) throw new Error("no hero card");
    const model = faceModel({ defId: def.id, def, radiant: false });
    const { container } = render(
      <FullArtAllowed allowed={true}>
        <CardFace face={model} layout="full" />
      </FullArtAllowed>,
    );
    expect(container.querySelector(".cf")?.getAttribute("data-full-art")).toBe("true");
  });

  it("R1108000 below FullArtAllowed false the hover preview carries a hero's Full text only when its face prints less", () => {
    let dropped = 0;
    for (const def of HEROES) {
      const model = faceModel({ defId: def.id, def, radiant: false });
      const less = faceText(model, undefined, false).less;
      render(
        <FullArtAllowed allowed={false}>
          <HoverPreview face={model} anchor={ANCHOR} />
        </FullArtAllowed>,
      );
      expect(screen.getByTestId(INSPECT_FACE).querySelector(".cf")?.hasAttribute("data-full-art"), def.id).toBe(false);
      expect(screen.queryByTestId(INSPECT_EXTENDED) !== null, def.id).toBe(less);
      cleanup();
      // Outside a lesson the full-art face prints no words, so its whole text is always beside it.
      render(<HoverPreview face={model} anchor={ANCHOR} />);
      expect(screen.queryByTestId(INSPECT_EXTENDED) !== null, def.id).toBe(model.text.full !== "");
      if (model.text.full !== "" && !less) dropped += 1;
      cleanup();
    }
    // The lesson drops the Full text beside a hero whose text its face prints whole.
    expect(dropped).toBeGreaterThan(0);
  });
});
