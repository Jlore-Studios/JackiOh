// R3045000 (issue #3045): a Miner or a Weapon fused onto a card leaves that card's art. The engine's Fuse joins the
// names and unions the tags (fuse.rs `build_def`); `<CardArt>` resolves the card whose art it draws, so a fused
// card wears its host's picture, seed and theme, and a Starship, which draws its own (R2249000), is unchanged.
// Every fused definition here is shaped as the engine's (test/fixtures.ts `fusedDef`): an Equip is `[equip, host]`.

import { cleanup, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, CardDefs } from "@jackioh/shared";

import { renderBoard } from "../../test/board.tsx";
import { baseView, emptySide, fusedDef, unit } from "../../test/fixtures.ts";
import { DORMANT_STARSHIP_ID } from "../fused.ts";
import { CardDefsProvider } from "../refContext.tsx";
import { CardArt, type CardArtProps } from "./CardArt.tsx";
import { artSubjectOf, MINER_IDS } from "./fusedArt.ts";
import { type ArtManifest } from "./manifest.ts";
import { starshipArtUri } from "./svg.ts";

afterEach(cleanup);

const BEAM_V3 = "stinkyscape-050";
const LAUNCHER = "stinkyscape-140";
const JADE_LAUNCHER = "stinkyscape-142";
const MINER_V2 = "stinkyscape-063";
const POINTMASTER = "core-020";
const STRATOS = "stinkyscape-068";
const OZARK = "stinkyscape-078";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** The catalog with the fused definitions the view would carry beside it (R243). */
function defsWith(...fused: readonly CardDef[]): CardDefs {
  return { ...CATALOG, ...Object.fromEntries(fused.map((fusion) => [fusion.id, fusion])) };
}

/** A card's art window as the board draws it: its definition's id, name, tags and type, under `defs`. */
function drawn(card: CardDef, defs: CardDefs | null, over: Partial<CardArtProps> = {}): HTMLElement {
  const art = (
    <CardArt defId={card.id} name={card.name} radiant={false} tags={card.tags} type={card.type} shape="oval" manifest={{}} {...over} />
  );
  const { container } = render(defs === null ? art : <CardDefsProvider defs={defs}>{art as ReactElement}</CardDefsProvider>);
  const found = container.querySelector<HTMLElement>(".cf-art");
  if (found === null) throw new Error("CardArt drew no .cf-art");
  return found;
}

/** What a window shows, for comparing two: its kind, theme, family, motif, picture and the file or painting. */
function lookOf(art: HTMLElement): Record<string, string | null> {
  return {
    art: art.getAttribute("data-art"),
    theme: art.getAttribute("data-art-theme"),
    family: art.getAttribute("data-art-family"),
    motif: art.getAttribute("data-art-motif"),
    picture: art.getAttribute("data-art-picture"),
    file: art.querySelector("img")?.getAttribute("src") ?? null,
    painting: art.style.backgroundImage === "" ? null : art.style.backgroundImage,
  };
}

/** The art of `card` alone, then of `fused` under `defs`: what the host looks like, and what the fusion draws. */
function looks(host: CardDef, fused: CardDef, defs: CardDefs, over: Partial<CardArtProps> = {}): { host: ReturnType<typeof lookOf>; fusion: ReturnType<typeof lookOf>; bare: ReturnType<typeof lookOf> } {
  const own = lookOf(drawn(host, null, over));
  cleanup();
  const fusion = lookOf(drawn(fused, defs, over));
  cleanup();
  // The same fusion with nothing to read it by, which is what every fused card drew before.
  const bare = lookOf(drawn(fused, null, over));
  cleanup();
  return { host: own, fusion, bare };
}

describe("R3045000: a fused Weapon keeps its host's art", () => {
  it("R3045000 Beam v3 equipped on a Unit draws the Unit's picture and theme, not a beam's", () => {
    for (const host of [POINTMASTER, STRATOS]) {
      const equipped = fusedDef([def(BEAM_V3), def(host)]);
      expect(equipped.name).toBe(`Beam v3 + ${def(host).name}`);
      expect(equipped.tags).toContain("Weapon");
      const seen = looks(def(host), equipped, defsWith(equipped));
      expect(seen.fusion, host).toEqual(seen.host);
      expect(seen.fusion.painting, host).not.toBeNull();
      // Before: the fusion was drawn from its own id, joined name and union of tags, so a different picture.
      expect(seen.bare.painting, host).not.toBe(seen.host.painting);
    }
  });

  it("R3045000 a Weapon-tagged card that names no weapon word is a Weapon too", () => {
    const equipped = fusedDef([def(LAUNCHER), def(POINTMASTER)]);
    const seen = looks(def(POINTMASTER), equipped, defsWith(equipped));
    expect(seen.fusion).toEqual(seen.host);
    expect(seen.bare.painting).not.toBe(seen.host.painting);
  });

  it("R3045000 a weapon's own tribe does not recolor the host", () => {
    // Jade Launcher is a CN card: fused onto a Human, the union of tags reads as CN's theme, which the Human's is not.
    const equipped = fusedDef([def(JADE_LAUNCHER), def(POINTMASTER)]);
    expect(equipped.tags).toEqual(expect.arrayContaining(["CN", "Human", "Weapon"]));
    const seen = looks(def(POINTMASTER), equipped, defsWith(equipped));
    expect(seen.bare.theme).toBe("cn");
    expect(seen.fusion.theme).toBe("human");
    expect(seen.fusion).toEqual(seen.host);
  });

  it("R3045000 it keeps the host's seed, whichever picture the card is drawn in", () => {
    const equipped = fusedDef([def(BEAM_V3), def(POINTMASTER)]);
    for (const picture of ["normal", "alt", "signature"] as const) {
      const seen = looks(def(POINTMASTER), equipped, defsWith(equipped), { picture });
      expect(seen.fusion, picture).toEqual(seen.host);
    }
    const seen = looks(def(POINTMASTER), equipped, defsWith(equipped), { radiant: true });
    expect(seen.fusion).toEqual(seen.host);
    expect(seen.fusion.painting).not.toBeNull();
  });

  it("R3045000 a delivered file is the host's, not the fusion's", () => {
    const equipped = fusedDef([def(BEAM_V3), def(POINTMASTER)]);
    const manifest: ArtManifest = { [POINTMASTER]: { base: true }, [equipped.id]: { base: true } };
    const seen = looks(def(POINTMASTER), equipped, defsWith(equipped), { manifest });
    expect(seen.host.art).toBe("real");
    expect(seen.fusion).toEqual(seen.host);
    expect(seen.fusion.file).toContain(POINTMASTER);
  });
});

describe("R3045000: on the board", () => {
  it("R3045000 a Unit with a Weapon or a Miner equipped is drawn in its host's art, from the definitions its view carries", () => {
    const paintings = (defId: string, equipped: CardDef | null): string => {
      const card = equipped ?? def(defId);
      const view = baseView({
        you: emptySide("p1", {
          units: [null, null, unit("p1", { instanceId: "u1", defId: card.id, tags: [...card.tags] }), null, null],
        }),
        ...(equipped === null ? {} : { defs: { [equipped.id]: equipped } }),
      });
      const art = renderBoard(view).querySelector<HTMLElement>('[data-testid="card-u1"] .cf-art');
      const painting = art?.style.backgroundImage ?? "";
      cleanup();
      return painting;
    };
    const own = paintings(POINTMASTER, null);
    expect(own).toContain("data:image/svg+xml");
    expect(paintings(POINTMASTER, fusedDef([def(BEAM_V3), def(POINTMASTER)], 1))).toBe(own);
    expect(paintings(POINTMASTER, fusedDef([def(MINER_V2), def(POINTMASTER)], 2))).toBe(own);
    // Two cards that are no gear are a different card, with a picture of its own.
    expect(paintings(POINTMASTER, fusedDef([def(STRATOS), def(POINTMASTER)], 3))).not.toBe(own);
  });
});

describe("R3045000: a fused Miner keeps its host's art", () => {
  it("R3045000 Miner v2 equipped on a Unit draws the Unit's picture and theme, not a drill's", () => {
    const equipped = fusedDef([def(MINER_V2), def(POINTMASTER)]);
    expect(equipped.tags).not.toContain("Weapon");
    const seen = looks(def(POINTMASTER), equipped, defsWith(equipped));
    expect(seen.fusion).toEqual(seen.host);
    expect(seen.bare.painting).not.toBe(seen.host.painting);
  });

  it("R3045000 the Miners are the catalog's Miner v1 to v5", () => {
    const named = Object.values(CATALOG)
      .filter((card) => /^Miner v\d$/.test(card.name))
      .map((card) => card.id)
      .sort();
    expect([...MINER_IDS].sort()).toEqual(named);
    expect(named).toHaveLength(5);
  });
});

describe("R3045000: the host is the card the gear was fused onto", () => {
  it("R3045000 a host equipped twice, a Weapon and then a Miner, still draws its own art", () => {
    const inner = fusedDef([def(BEAM_V3), def(POINTMASTER)], 1);
    const outer = fusedDef([def(MINER_V2), inner], 2);
    expect(outer.ingredients).toEqual([{ defId: MINER_V2 }, { defId: inner.id }]);
    // The view carries the outer definition and maybe not the inner one (R243): the id spells it.
    for (const defs of [defsWith(inner, outer), defsWith(outer)]) {
      const seen = looks(def(POINTMASTER), outer, defs);
      expect(seen.fusion).toEqual(seen.host);
    }
  });

  it("R3045000 fusing two cards that are no gear changes nothing", () => {
    const pair = fusedDef([def(POINTMASTER), def(STRATOS)]);
    const seen = looks(def(STRATOS), pair, defsWith(pair));
    expect(seen.fusion).toEqual(seen.bare);
    expect(seen.fusion).not.toEqual(seen.host);
  });

  it("R3045000 gear fused onto gear has no host, so it draws as it did", () => {
    const rigs = fusedDef([def(BEAM_V3), def(MINER_V2)]);
    const seen = looks(def(MINER_V2), rigs, defsWith(rigs));
    expect(seen.fusion).toEqual(seen.bare);
  });

  it("R3045000 a digest id (R468) that hides the ingredients leaves the art as it is", () => {
    const hidden: CardDef = {
      ...fusedDef([def(BEAM_V3), def(POINTMASTER)], 3),
      id: "t-3:#0123456789abcdef",
      ingredients: [{ defId: BEAM_V3 }, { defId: "t-2:#fedcba9876543210" }],
    };
    const seen = looks(def(POINTMASTER), hidden, defsWith(hidden));
    expect(seen.fusion).toEqual(seen.bare);
    // A digest whose definition is held lists its ingredients all the same.
    const digest: CardDef = { ...fusedDef([def(BEAM_V3), def(POINTMASTER)], 4), id: "t-4:#0123456789abcdef" };
    const held = looks(def(POINTMASTER), digest, defsWith(digest));
    expect(held.fusion).toEqual(held.host);
  });

  it("R3045000 with no catalog to read the ingredients from, the art is as it was", () => {
    const equipped = fusedDef([def(BEAM_V3), def(POINTMASTER)]);
    const subject = { defId: equipped.id, name: equipped.name, tags: equipped.tags };
    expect(artSubjectOf(subject, null)).toBe(subject);
    expect(artSubjectOf(subject, () => undefined)).toBe(subject);
  });

  it("R3045000 a host the catalog lacks leaves the art as it was", () => {
    const equipped: CardDef = { ...fusedDef([def(BEAM_V3), def(POINTMASTER)]), ingredients: [{ defId: BEAM_V3 }, { defId: "core-999" }] };
    const subject = { defId: equipped.id, name: equipped.name, tags: equipped.tags };
    expect(artSubjectOf(subject, (id) => defsWith(equipped)[id])).toBe(subject);
  });

  it("R3045000 a Stinkyscape ship named like a miner is a host, not a Miner", () => {
    // Ozark is drawn as a miner ship from its name, but it is no Equip: gear is the five Miners and the Weapon tag.
    const equipped = fusedDef([def(BEAM_V3), def(OZARK)]);
    const seen = looks(def(OZARK), equipped, defsWith(equipped));
    expect(seen.fusion).toEqual(seen.host);
  });
});

describe("R3045000: a Starship keeps its own art", () => {
  const hull = def(DORMANT_STARSHIP_ID);

  it("R3045000 a Starship in play wears the Starship's art, however it was fused", () => {
    const fused = fusedDef([hull, def(BEAM_V3)]);
    const starship = { pieces: 2, launched: false };
    const own = lookOf(drawn(hull, null, { starship }));
    cleanup();
    const art = drawn(fused, defsWith(fused), { starship });
    expect(art.getAttribute("data-art-family")).toBe("starship");
    expect(lookOf(art)).toEqual(own);
    expect(art.style.backgroundImage).toContain(starshipArtUri(2, false, false));
  });

  it("R3045000 a fused hull outside play draws the hull, the card the pieces were fused onto", () => {
    const fused = fusedDef([hull, def(BEAM_V3), def(MINER_V2)]);
    const seen = looks(hull, fused, defsWith(fused));
    expect(seen.fusion).toEqual(seen.host);
  });
});
