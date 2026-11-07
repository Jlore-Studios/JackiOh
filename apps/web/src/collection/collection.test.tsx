// The collection screen's panels (#1307, SPEC §14), from fixture responses (nothing here asks the
// server anything): owned cards, wallet and tracks. R1655000: a card in context and the account's
// cosmetics as tiles. R1956000: the owned cards' grid wears their picks. R3015007 (#3015): a card's variants as
// ladders of their levels, each rung owned, next or locked, a held token offered on the next one alone.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLOBAL_PACK_NAME, PROFICIENCY_LEVELS, PROFICIENCY_LEVEL_PLAYS, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
import { SHIPPED_SETS, newestShippedSet, type CardDef } from "@jackioh/shared";

import {
  AccountCosmeticsOwned,
  CARD_PICKS_SETTINGS_KEY,
  COSMETICS_SETTINGS_KEY,
  cosmeticsOf,
  ownsAccountCosmetic,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
  subscribeCardPickWrites,
  writeCardFullArt,
  writeCardLevel,
  writeCardPick,
  writeCosmetics,
  type CardPickChange,
  type OwnedByCard,
} from "../cards/cosmetics.ts";
import { CHINESE } from "../cards/chinese.ts";
import { STAMP_LEVELS, readOwnProficiency, replaceOwnProficiency, resetProficiencyForTests, subscribeStampWrites } from "../cards/stamps.ts";
import { FullArtOwned } from "../cards/fullArt.ts";
import { ViewerFullArt, ownedCardCosmetics, resetOwnedFullArtForTests } from "../cards/ownedFullArt.ts";
import {
  AUGMENT_ANIMATED_ID,
  AUGMENT_SIGNATURE_ID,
  VERSION_ALT_ART_ID,
  VERSION_ALT_FULL_ART_ANIMATED_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_ALT_FULL_ART_SIGNED_ID,
  VERSION_ANIMATED_ID,
  VERSION_COMIC_ID,
  VERSION_EXTENDED_ART_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_FULL_ART_SIGNED_ID,
  VERSION_IMMERSIVE_ID,
  VERSION_NEON_ID,
  VERSION_NIGHT_ID,
  VERSION_OUT_OF_FRAME_ID,
  VERSION_PIXEL_ID,
  VERSION_PRISMATIC_ID,
  VERSION_SET_SHOWCASE_ID,
  VERSION_SKETCH_FULL_ART_ANIMATED_ID,
  VERSION_SKETCH_FULL_ART_ID,
  VERSION_SKETCH_FULL_ART_SIGNED_ID,
  VERSION_SKETCH_ID,
  VERSION_STAINED_GLASS_ID,
  VERSION_TAROT_ID,
  VERSION_WATERCOLOR_ID,
  VERSION_WOODCUT_ID,
  ladderOf,
  offeredOn,
} from "../cards/versions.ts";
import {
  CARD_SETTINGS_DEFAULTS,
  CardFace,
  INSPECT_CLOSE,
  INSPECT_DETAIL,
  closeInspect,
  faceModel,
  writeCardSettings,
} from "../cards/index.ts";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { packName } from "../packs/packs.ts";
import { collectionOf, cosmeticPacksInfo, tokenBars, walletInfo } from "../shop/fixtures.ts";
import { expectEmptyState } from "../test/empty.ts";
import { settle } from "../test/settle.ts";
import { stubServer } from "../test/stubServer.ts";
import AccountCosmeticTiles from "./AccountCosmeticTiles.tsx";
import OwnedCards from "./OwnedCards.tsx";
import TracksPanel from "./TracksPanel.tsx";
import WalletPanel from "./WalletPanel.tsx";
import {
  COLLECTED_DEFS,
  NO_FULL_ART_ID,
  accountTiles,
  cardLadders,
  cardNameOf,
  cardStrip,
  collectionSectionOf,
  collectionTestid as T,
  setCompletions,
} from "./collection.ts";
import { collectionOwning, fullCollection, track, tracksOf } from "./fixtures.ts";

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
});

afterEach(() => {
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  closeInspect();
  cleanup();
  localStorage.clear();
  resetCosmeticsForTests();
  resetLocaleForTests();
});

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`${id} is not in the catalog`);
  return def;
}

/** Two Core cards, one Classic card and a Core token the completion count must skip. */
const DEFS = [defOf("core-001"), defOf("core-002"), defOf("classic-001"), defOf("core-051-1")];

describe("#1307 setCompletions", () => {
  it("counts each shipped set's non-token cards and those owned", () => {
    const sets = setCompletions(collectionOwning(["core-001", "core-051-1"]), DEFS);

    expect(sets.map((entry) => entry.set)).toEqual([...SHIPPED_SETS]);
    expect(sets.map((entry) => [entry.owned, entry.total])).toEqual([
      [1, 2],
      [0, 1],
      [0, 0],
      [0, 0],
      [0, 0],
      [0, 0],
      // Training Grounds ships (#2204), so it has a row.
      [0, 0],
    ]);
    expect(sets[0]?.cards.map((card) => [card.def.id, card.owned])).toEqual([
      ["core-001", true],
      ["core-002", false],
    ]);
  });

  it("a quantity of 0 is not owned", () => {
    const sets = setCompletions({ catalogVersion: "test", entries: [{ cardId: "core-001", quantity: 0 }] }, DEFS);

    expect(sets[0]?.owned).toBe(0);
  });

  it("the launch grant owns every collected card", () => {
    const sets = setCompletions(fullCollection());

    expect(sets.every((entry) => entry.owned === entry.total)).toBe(true);
    for (const entry of sets) {
      const inCatalog = Object.values(CATALOG).filter((def) => def.set === entry.set && !def.token);
      expect(entry.total).toBe(inCatalog.length);
    }
    expect(sets.reduce((sum, entry) => sum + entry.total, 0)).toBe(COLLECTED_DEFS.length);
  });
});

describe("#1307 OwnedCards", () => {
  it("shows each set's completion count", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);

    expect(screen.getByTestId(T.completion("Core"))).toHaveTextContent("1 of 2");
    expect(screen.getByTestId(T.completion("Classic"))).toHaveTextContent("0 of 1");
    expect(screen.queryByTestId(T.card("core-001"))).toBeNull();
  });

  it("draws an unowned card as a locked silhouette", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));

    expect(screen.getByTestId(T.setToggle("Core"))).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId(T.card("core-001"))).toHaveAttribute("data-owned", "true");
    expect(screen.getByTestId(T.card("core-001"))).not.toHaveTextContent("Locked");
    expect(screen.getByTestId(T.card("core-002"))).toHaveAttribute("data-owned", "false");
    expect(screen.getByTestId(T.card("core-002"))).toHaveTextContent("Locked");
    expect(screen.queryByTestId(T.card("core-051-1"))).toBeNull();
  });

  it("opens one set at a time", () => {
    render(<OwnedCards collection={collectionOwning([])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.setToggle("Classic")));

    expect(screen.queryByTestId(T.card("core-001"))).toBeNull();
    expect(screen.getByTestId(T.card("classic-001"))).toBeInTheDocument();
  });

  it("a locked card still opens its view, and its text and meta are one click away", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-002")));

    expect(screen.getByTestId(T.cardView)).toHaveAttribute("data-card", "core-002");
    expect(screen.getByTestId(T.cardView)).toHaveTextContent("Not owned yet");
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();

    fireEvent.click(screen.getByTestId(T.cardViewText));
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-002");
    expect(detail).toHaveTextContent("Not owned yet");

    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(T.cardViewClose));
    expect(screen.queryByTestId(T.cardView)).toBeNull();
  });

  it("an owned card's view and its detail say so", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-001")));

    expect(screen.getByTestId(T.cardView)).toHaveTextContent("Owned");
    fireEvent.click(screen.getByTestId(T.cardViewText));
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveTextContent("Owned");
  });
});

describe("#2053 card names in the reader's language", () => {
  /** A locked tile, its view and its detail, opened in turn: the names each carries. */
  function openLocked(): { tile: string | null; locked: string | null; view: HTMLElement; title: string; detail: string | null } {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    const tile = screen.getByTestId(T.card("core-001")).getAttribute("aria-label");
    const locked = screen.getByTestId(T.card("core-002")).getAttribute("aria-label");
    fireEvent.click(screen.getByTestId(T.card("core-002")));
    const view = screen.getByTestId(T.cardView);
    const title = view.querySelector(".dialog__title")?.textContent ?? "";
    fireEvent.click(screen.getByTestId(T.cardViewText));
    return { tile, locked, view, title, detail: screen.getByTestId(INSPECT_DETAIL).getAttribute("aria-label") };
  }

  it("#2053 cardNameOf names a catalog card as the reader reads it, else its id", async () => {
    expect(cardNameOf("core-004")).toBe(defOf("core-004").name);
    expect(cardNameOf("not-a-card")).toBe("not-a-card");

    await setLocale("zh-CN");

    expect(cardNameOf("core-004")).toBe(CHINESE["core-004"]!.name);
    expect(cardNameOf("not-a-card")).toBe("not-a-card");
  });

  it("#2053 in English a tile, its view and its detail are labeled with the catalog's name", () => {
    const opened = openLocked();

    expect(opened.tile).toBe(defOf("core-001").name);
    expect(opened.locked).toBe(`${defOf("core-002").name}, not owned yet`);
    expect(opened.view).toHaveAccessibleName(defOf("core-002").name);
    expect(opened.title).toBe(defOf("core-002").name);
    expect(opened.detail).toBe(defOf("core-002").name);
  });

  it("#2053 for a reader of Chinese they are labeled with the Chinese table's name", async () => {
    await setLocale("zh-CN");
    const opened = openLocked();

    expect(opened.tile).toBe(CHINESE["core-001"]!.name);
    expect(opened.locked).toBe(`${CHINESE["core-002"]!.name}，尚未拥有`);
    expect(opened.view).toHaveAccessibleName(CHINESE["core-002"]!.name);
    expect(opened.title).toBe(CHINESE["core-002"]!.name);
    expect(opened.detail).toBe(CHINESE["core-002"]!.name);
  });

});

describe("#1307 WalletPanel", () => {
  it("shows the balance and every unopened pack", () => {
    render(<WalletPanel wallet={walletInfo()} packs={packsInfo()} cosmeticPacks={cosmeticPacksInfo()} />);

    expect(screen.getByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(screen.getByTestId(T.packs(GLOBAL_PACK_NAME))).toHaveTextContent("Global pack: 1 unopened");
    expect(screen.getByTestId(T.packs("Core"))).toHaveTextContent("Core pack: 2 unopened");
    expect(screen.getByTestId(T.cosmeticPacks)).toHaveTextContent("Cosmetic packs: 2 unopened");
    expect(screen.queryByTestId(T.noPacks)).toBeNull();
  });

  it("says when no pack is held", () => {
    render(
      <WalletPanel
        wallet={walletInfo()}
        packs={{ ...packsInfo(), unopened: [] }}
        cosmeticPacks={{ ...cosmeticPacksInfo(), unopened: 0 }}
      />,
    );

    expect(screen.getByTestId(T.noPacks)).toHaveTextContent("No unopened packs.");
    const noPacks = expectEmptyState(screen.getByTestId(T.noPacks));
    expect(within(noPacks).getByText("Get packs in the shop")).toHaveAttribute("href", "/shop");
    expect(screen.queryByTestId(T.cosmeticPacks)).toBeNull();
  });
});

describe("#1307 TracksPanel", () => {
  it("shows an unlocked track's level, XP bar and next rewards", () => {
    const newest = newestShippedSet();
    render(<TracksPanel tracks={tracksOf().tracks} />);

    expect(screen.getByTestId(T.track(newest))).toHaveAttribute("data-unlocked", "true");
    expect(screen.getByTestId(T.trackLevel(newest))).toHaveTextContent(`Level 7 of ${String(SET_TRACK_LEVELS)}`);
    const bar = screen.getByRole("progressbar", { name: `${newest} track XP` });
    expect(bar).toHaveAttribute("max", "700");
    expect(bar).toHaveAttribute("value", "50");
    expect(screen.getByTestId(T.trackXp(newest))).toHaveTextContent("650 XP to the next level");
    expect(screen.getByTestId(T.trackRewards(newest))).toHaveTextContent(
      `Next rewards: 25 Jlore, 2 × ${packName(newest)}, 3 × card token`,
    );
    expect(screen.queryByTestId(T.trackLock(newest))).toBeNull();
  });

  it("marks a locked track and says how it unlocks", () => {
    render(<TracksPanel tracks={tracksOf().tracks} />);

    expect(screen.getByTestId(T.track("Core"))).toHaveAttribute("data-unlocked", "false");
    expect(screen.getByTestId(T.trackLock("Core"))).toHaveTextContent(
      `Locked: unlocks once the ${newestShippedSet()} track reaches level ${String(SET_TRACK_UNLOCK_LEVEL)}.`,
    );
    expect(screen.getByTestId(T.trackRewards("Core"))).toHaveTextContent("Next rewards: 40 Jlore");
    expect(screen.getAllByRole("listitem").map((row) => row.getAttribute("data-testid"))).toEqual(
      SHIPPED_SETS.map((set) => T.track(set)),
    );
  });

  it("a track on its last level reads no next level", () => {
    render(
      <TracksPanel
        tracks={[
          track({
            level: SET_TRACK_LEVELS,
            levelXp: null,
            xpToNext: null,
            nextRewards: null,
            rewardsClaimed: SET_TRACK_LEVELS,
            unlocked: true,
          }),
        ]}
      />,
    );

    expect(screen.getByTestId(T.trackXp("Core"))).toHaveTextContent("Last level reached.");
    expect(screen.getByTestId(T.trackRewards("Core"))).toHaveTextContent("All rewards claimed.");
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
});

describe("R1579000 the Collection's cosmetics and sections", () => {
  it("R1579000 collectionSectionOf reads ?section= and gives the hub otherwise", () => {
    expect(collectionSectionOf("?section=cards")).toBe("cards");
    expect(collectionSectionOf("?section=cosmetics")).toBe("cosmetics");
    expect(collectionSectionOf("?x=1&section=cards")).toBe("cards");
    expect(collectionSectionOf("")).toBeNull();
    expect(collectionSectionOf("?section=")).toBeNull();
    expect(collectionSectionOf("?section=shop")).toBeNull();
    expect(collectionSectionOf("?section=Cards")).toBeNull();
    expect(collectionSectionOf("?other=cards")).toBeNull();
  });
});

describe("R1655000 a card in context", () => {
  const BIG_D = "core-001"; // a Common unit with a full art of its own
  const PLAIN = "core-002"; // a Common unit with none
  const GARY = "core-004";
  const MYTHIC = "core-096";
  const BIG_D_ART = "full-art-big-d-fender";
  /** R2715000: Big D-fender's Animated art, its normal face with its art moving. */
  const BIG_D_MOVING = "animated-big-d-fender";
  /** R2725001: Carnivorous Cube, an Epic Unit offered the versions and augments, with no retired full art of its own. */
  const CUBE = "core-022";
  const CARDS = [BIG_D, PLAIN, GARY, MYTHIC, CUBE].map(defOf);
  /** What a ledger holding `ids` on Big D-fender owns, as the client reads it (R2725002: what a retired full art implies). */
  const owning = (...ids: string[]) => ownedCardCosmetics({ cards: ids.map((cosmeticId) => ({ cardId: BIG_D, cosmeticId })), account: [] });
  const OWNS_BIG_D_ART = owning(BIG_D_ART);

  /** Opens `id` in its set, with every cosmetic owned unless `owned` says what the ledger holds. */
  function open(id: string, owned: "every" | ReadonlyMap<string, ReadonlySet<string>> = "every"): void {
    render(
      <FullArtOwned owned={owned}>
        <OwnedCards collection={collectionOwning(CARDS.map((def) => def.id))} defs={CARDS} />
      </FullArtOwned>,
    );
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card(id)));
  }
  const large = (): Element => {
    const face = screen.getByTestId(T.cardViewFace).querySelector(".cf");
    if (face === null) throw new Error("the large face is not drawn");
    return face;
  };
  const shown = (): Element | null => screen.queryByTestId(T.cardViewPreview)?.querySelector(".cf") ?? null;
  const item = (id: string): HTMLElement => screen.getByTestId(T.stripItem(id));
  /** R3015007: a ladder's rung, by its level's id (the normal face's `NO_FULL_ART_ID`). */
  const rung = (id: string): HTMLElement => screen.getByTestId(T.rung(id));
  /** R3015007: the open view's ladders, by variant, in order. */
  const ladderIds = (): (string | undefined)[] => screen.getAllByTestId(/^collection-ladder-/u).map((section) => section.dataset.variant);
  /** R3015007: each variant `defId` is offered, with its first rung and that rung's rarity. */
  const firstLevels = (defId: string): (string | undefined)[][] =>
    cardLadders(defId, () => true).map((ladder) => [ladder.variant, ladder.rungs[0]?.id, ladder.rungs[0]?.rarity]);
  /** R3015007: every level `defId`'s ladders list. */
  const listed = (defId: string): string[] => cardLadders(defId, () => true).flatMap((ladder) => ladder.rungs.map((each) => each.id));

  it("R1655000 a card opened in the Collection wears its own picks while another card draws the defaults", () => {
    // R1953001: the account's synced finish is worn by no card.
    writeCosmetics({ finish: "finish-linen" });
    writeCardPick(BIG_D, "border", "border-wood");
    writeCardPick(BIG_D, "foil", "foil-rainbow");
    open(BIG_D);

    expect(large().getAttribute("data-frame")).toBe("border-wood");
    expect(large().getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    expect(large().getAttribute("data-finish")).toBeNull();
    expect(item("border-wood")).toHaveAttribute("data-equipped", "true");
    expect(item("border-standard")).toHaveAttribute("data-equipped", "false");

    fireEvent.click(screen.getByTestId(T.card(PLAIN)));
    expect(screen.getByTestId(T.cardView)).toHaveAttribute("data-card", PLAIN);
    expect(large().getAttribute("data-finish")).toBeNull();
    expect(large().getAttribute("data-frame")).toBeNull();
    expect(large().getAttribute("data-cosmetic-foil")).toBeNull();
  });

  it("R1655000 R3015007 the strip holds each kind's registry entries, and the ladders each variant the card is offered", () => {
    open(BIG_D);
    for (const kind of ["foil", "finish", "border"] as const) {
      const strip = screen.getByTestId(T.strip(kind));
      for (const cosmetic of cosmeticsOf(kind)) {
        expect(within(strip).getByTestId(T.stripItem(cosmetic.id))).toHaveTextContent(cosmetic.name);
      }
    }
    // R3015007: the Default first, the normal face then its animated level, then Full art and the Sketch, which Big
    // D-fender is offered, and R3017001 its side of each part 6 pair; no strip of versions or augments.
    expect(ladderIds()).toEqual(["default", "full-art", "sketch", "night", "stained-glass", "woodcut"]);
    expect(within(screen.getByTestId(T.ladder("full-art"))).getByTestId(T.rung(VERSION_FULL_ART_ID))).toHaveTextContent("Legendary");
    expect(within(screen.getByTestId(T.ladder("default"))).getByTestId(T.rung(NO_FULL_ART_ID))).toHaveTextContent("Normal");
    expect(within(screen.getByTestId(T.ladder("full-art"))).getByTestId(T.rung(VERSION_FULL_ART_SIGNED_ID))).toHaveTextContent("Mythic");
    expect(screen.queryByTestId(T.strip("version"))).toBeNull();
    expect(screen.queryByTestId(T.strip("augment"))).toBeNull();
    expect(screen.getAllByRole("button", { name: /^Use default/u })).toHaveLength(3);

    // Speciesist is offered no variant but its Default and, R3017001, its side of each part 6 pair.
    fireEvent.click(screen.getByTestId(T.card(PLAIN)));
    expect(ladderIds()).toEqual(["default", "neon", "watercolor", "woodcut"]);
    expect(screen.queryByTestId(T.ladder("full-art"))).toBeNull();
    expect(screen.getAllByRole("button", { name: /^Use default/u })).toHaveLength(3);
  });

  it("R1655000 hovering or focusing a strip item shows it on the large card and equips nothing", () => {
    writeCardPick(BIG_D, "finish", "finish-linen");
    open(BIG_D);
    expect(shown()).toBeNull();

    fireEvent.mouseEnter(item("border-bone"));
    expect(shown()?.getAttribute("data-frame")).toBe("border-bone");
    expect(shown()?.getAttribute("data-finish")).toBe("finish-linen");
    expect(large().getAttribute("data-frame")).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ finish: "finish-linen" });

    fireEvent.mouseLeave(item("border-bone"));
    expect(shown()).toBeNull();

    fireEvent.focus(item("foil-holo-sweep"));
    expect(shown()?.getAttribute("data-cosmetic-foil")).toBe("foil-holo-sweep");
    // A preview of a foil that is no chase holds still; a chase moves (R1953001).
    expect(shown()?.getAttribute("data-cosmetic-motion")).toBe("static");
    fireEvent.mouseEnter(item("finish-gilded-edge"));
    fireEvent.blur(item("foil-holo-sweep"));
    expect(shown()?.getAttribute("data-finish")).toBe("finish-gilded-edge");
    expect(shown()?.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(shown()?.getAttribute("data-cosmetic-motion")).toBe("animated");

    fireEvent.mouseLeave(item("finish-gilded-edge"));
    expect(shown()).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ finish: "finish-linen" });
    expect(JSON.parse(localStorage.getItem(CARD_PICKS_SETTINGS_KEY) ?? "null")).toEqual({ [BIG_D]: { finish: "finish-linen" } });
  });

  it("R1655000 a shown foil keeps the rarity rules: a Mythic card wearing its rarity foil wears none", () => {
    writeCardSettings({ rarityFoil: true });
    open(MYTHIC);
    fireEvent.mouseEnter(item("foil-rainbow"));

    expect(shown()).not.toBeNull();
    expect(shown()?.getAttribute("data-cosmetic-foil")).toBeNull();
    fireEvent.mouseEnter(item("border-circuit"));
    expect(shown()?.getAttribute("data-frame")).toBe("border-circuit");
  });

  it("R1655000 clicking an owned item equips it on that card only, and Use default clears it", () => {
    // R1953001: the account's synced border is worn by no card, before a pick or after Use default.
    writeCosmetics({ border: "border-bone" });
    open(BIG_D);
    expect(large().getAttribute("data-frame")).toBeNull();

    fireEvent.click(item("border-wood"));
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-wood" });
    expect(large().getAttribute("data-frame")).toBe("border-wood");
    expect(item("border-wood")).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.stripDefault("border"))).toHaveAttribute("aria-pressed", "false");
    expect(readCosmetics().border).toBe("border-bone");

    // A pick of the kind's default is a pick of its own.
    fireEvent.click(item("border-standard"));
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-standard" });
    expect(large().getAttribute("data-frame")).toBeNull();

    fireEvent.click(screen.getByTestId(T.stripDefault("border")));
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBe("{}");
    expect(large().getAttribute("data-frame")).toBeNull();
    expect(screen.getByTestId(T.stripDefault("border"))).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(item("foil-rainbow"));
    fireEvent.click(screen.getByTestId(T.card(PLAIN)));
    expect(readCardPicks(PLAIN)).toEqual({});
    expect(large().getAttribute("data-cosmetic-foil")).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ foil: "foil-rainbow" });
    // The account's own picks are the account's: a card's pick never writes them.
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "{}")).toMatchObject({ border: "border-bone" });
  });

  it("R1655000 R3015007 a locked rung shows its rarity and equips nothing", () => {
    open(BIG_D, new Map());

    const art = rung(VERSION_FULL_ART_ID);
    expect(art).toHaveAttribute("data-owned", "false");
    expect(art).toHaveTextContent("Legendary");
    // R3015004: Full art's first level is next, every level below it (none) owned; the one above it needs it.
    expect(art).toHaveAttribute("data-next", "true");
    expect(art).toHaveTextContent("Next level");
    expect(rung(VERSION_FULL_ART_ANIMATED_ID)).toHaveTextContent("Needs Full art");
    expect(item("border-wood")).toHaveAttribute("data-owned", "true");
    expect(rung(NO_FULL_ART_ID)).toHaveAttribute("data-owned", "true");

    fireEvent.mouseEnter(art);
    expect(large().getAttribute("data-full-art")).toBeNull();
    fireEvent.mouseLeave(art);
    // A click opens its preview and writes nothing.
    fireEvent.click(art);
    expect(screen.getByTestId(T.preview)).toBeInTheDocument();
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBeNull();
    expect(art).toHaveAttribute("data-equipped", "false");
    expect(shown()).toBeNull();

    // R3015006: the card's retired still Epic owns Full art's first level, which is then picked by its id.
    cleanup();
    open(BIG_D, OWNS_BIG_D_ART);
    expect(rung(VERSION_FULL_ART_ID)).toHaveAttribute("data-owned", "true");
    expect(rung(VERSION_FULL_ART_ID)).not.toHaveTextContent("Next level");
    expect(rung(VERSION_ANIMATED_ID)).toHaveAttribute("data-owned", "false");
    expect(rung(VERSION_FULL_ART_ANIMATED_ID)).toHaveAttribute("data-next", "true");
    fireEvent.click(rung(VERSION_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_FULL_ART_ID });
    expect(rung(VERSION_FULL_ART_ID)).toHaveAttribute("data-equipped", "true");
  });

  it("R2655000 R3015005 R3015006 a full-art card lists none of its retired tiers, and a retired pick wears the level it maps to until it is edited into one level", () => {
    const SIGNED = `${BIG_D_ART}-signed`;
    open(BIG_D, owning(BIG_D_ART, SIGNED));
    const rungsOf = (variant: string): (string | undefined)[][] =>
      within(screen.getByTestId(T.ladder(variant)))
        .getAllByRole("button")
        .map((button) => [button.dataset.level, button.dataset.rarity, button.dataset.owned]);
    // R3015006: the Mythic owns every level of Full art and the Default's animated one; the Sketch is owned nowhere.
    expect(rungsOf("default")).toEqual([
      [NO_FULL_ART_ID, "Common", "true"],
      [VERSION_ANIMATED_ID, "Epic", "true"],
    ]);
    expect(rungsOf("full-art")).toEqual([
      [VERSION_FULL_ART_ID, "Legendary", "true"],
      [VERSION_FULL_ART_ANIMATED_ID, "Mythic", "true"],
      [VERSION_FULL_ART_SIGNED_ID, "Mythic", "true"],
    ]);
    expect(rungsOf("sketch").map(([, , owned]) => owned)).toEqual(["false", "false", "false", "false"]);
    for (const retired of [BIG_D_ART, `${BIG_D_ART}-animated`, SIGNED, BIG_D_MOVING, AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]) {
      expect(screen.queryByTestId(T.stripItem(retired)), retired).toBeNull();
      expect(screen.queryByTestId(T.rung(retired)), retired).toBeNull();
    }
    cleanup();

    // A pick made before #2725 of the signed full art: Full art's signed level, the Mythic's look.
    writeCardFullArt(BIG_D, SIGNED);
    open(BIG_D, owning(BIG_D_ART, SIGNED));
    expect(large().getAttribute("data-full-art-tier")).toBe("Mythic");
    expect(large().getAttribute("data-level")).toBe(VERSION_FULL_ART_SIGNED_ID);
    expect(large().querySelector(".cf-signature")).not.toBeNull();
    expect(rung(VERSION_FULL_ART_SIGNED_ID)).toHaveAttribute("data-equipped", "true");
    for (const id of [NO_FULL_ART_ID, VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID]) expect(rung(id), id).toHaveAttribute("data-equipped", "false");

    // Its animated level: the pick becomes that one level, the Legendary's look.
    fireEvent.click(rung(VERSION_FULL_ART_ANIMATED_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_FULL_ART_ANIMATED_ID });
    fireEvent.mouseLeave(rung(VERSION_FULL_ART_ANIMATED_ID));
    expect(large().getAttribute("data-full-art-tier")).toBe("Legendary");
    expect(large().querySelector(".cf-signature")).toBeNull();
    // The Default's animated level: the Animated art's look.
    fireEvent.click(rung(VERSION_ANIMATED_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_ANIMATED_ID });
    fireEvent.mouseLeave(rung(VERSION_ANIMATED_ID));
    expect(large().getAttribute("data-full-art-tier")).toBe("Animated");
    expect(large().getAttribute("data-full-art")).toBeNull();
    // The normal face: no level at all.
    fireEvent.click(rung(NO_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: false });
    fireEvent.mouseLeave(rung(NO_FULL_ART_ID));
    expect(large().getAttribute("data-full-art-tier")).toBeNull();
    cleanup();

    // R3015005: a pick of #2725's shape, the Full art version and Animated, wears Full art's animated level, and its
    // first edit writes the one level it chose and clears the augments.
    localStorage.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify({ [BIG_D]: { fullArt: VERSION_FULL_ART_ID, augments: [AUGMENT_ANIMATED_ID] } }));
    resetCosmeticsForTests();
    open(BIG_D, owning(BIG_D_ART, SIGNED));
    expect(rung(VERSION_FULL_ART_ANIMATED_ID)).toHaveAttribute("data-equipped", "true");
    expect(large().getAttribute("data-full-art-tier")).toBe("Legendary");
    fireEvent.click(rung(VERSION_FULL_ART_SIGNED_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_FULL_ART_SIGNED_ID });
  });

  it("R2715000 R3015007 Animated is the Default's second level, locked until owned and next above the normal face, and moves the face the card wears", () => {
    // Not owned: locked but next, it previews and equips nothing.
    open(BIG_D, OWNS_BIG_D_ART);
    const animated = (): HTMLElement => rung(VERSION_ANIMATED_ID);
    expect(animated()).toHaveTextContent("Animated");
    expect(animated()).toHaveAttribute("data-rarity", "Epic");
    expect(animated()).toHaveAttribute("data-owned", "false");
    expect(animated()).toHaveAttribute("data-next", "true");
    expect(animated()).toHaveTextContent("Next level");
    fireEvent.click(animated());
    expect(readCardPicks(BIG_D)).toEqual({});
    cleanup();

    // R3015006: the retired Animated art owns it. Shown on the large card, the normal face moving, then chosen.
    open(BIG_D, owning(BIG_D_MOVING));
    expect(animated()).toHaveAttribute("data-owned", "true");
    fireEvent.mouseEnter(animated());
    expect(large().getAttribute("data-full-art-tier")).toBe("Animated");
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(large().getAttribute("data-art-motion")).not.toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({});
    fireEvent.click(animated());
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_ANIMATED_ID });
    expect(animated()).toHaveAttribute("data-equipped", "true");
    fireEvent.mouseLeave(animated());
    expect(large().getAttribute("data-full-art-tier")).toBe("Animated");
    expect(large().getAttribute("data-level")).toBe(VERSION_ANIMATED_ID);
    expect(large().querySelector(".card-type")).not.toBeNull();
    // The normal face's rung takes it off.
    fireEvent.click(rung(NO_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: false });
    expect(large().getAttribute("data-art-motion")).toBeNull();
  });

  it("R1655000 R3015007 a level is shown while it is shown, and stands once chosen", () => {
    writeCardSettings({ fullArt: false });
    open(BIG_D, OWNS_BIG_D_ART);
    expect(large().getAttribute("data-full-art")).toBeNull();

    fireEvent.mouseEnter(rung(VERSION_FULL_ART_ID));
    expect(large().getAttribute("data-full-art")).toBe("true");
    fireEvent.mouseLeave(rung(VERSION_FULL_ART_ID));
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBeNull();

    fireEvent.mouseEnter(rung(VERSION_FULL_ART_ID));
    fireEvent.click(rung(VERSION_FULL_ART_ID));
    fireEvent.mouseLeave(rung(VERSION_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_FULL_ART_ID });
    expect(large().getAttribute("data-full-art")).toBe("true");

    // The normal face is a pick of its own, and it beats the setting being on; Full art's level, chosen again, wears it.
    writeCardSettings({ fullArt: true });
    fireEvent.click(rung(NO_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: false });
    expect(large().getAttribute("data-full-art")).toBeNull();
    fireEvent.click(rung(VERSION_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_FULL_ART_ID });
    expect(large().getAttribute("data-full-art")).toBe("true");
  });

  it("R1655000 R3015007 a level shown, not chosen, is not marked as worn: the ladders say what the card wears", () => {
    open(BIG_D, OWNS_BIG_D_ART);
    // With no pick of its own the setting decides, and no rung is marked.
    const equipped = (): (string | undefined)[] =>
      screen
        .getAllByTestId(/^collection-rung-/u)
        .filter((button) => button.dataset.equipped === "true")
        .map((button) => button.dataset.level);
    expect(equipped()).toEqual([]);

    fireEvent.focus(rung(VERSION_FULL_ART_ID));
    expect(large().getAttribute("data-full-art"), "the large card shows it").toBe("true");
    expect(rung(VERSION_FULL_ART_ID)).toHaveAttribute("aria-pressed", "false");
    expect(rung(VERSION_FULL_ART_ID)).toHaveAttribute("data-equipped", "false");
    expect(equipped()).toEqual([]);

    fireEvent.click(rung(VERSION_FULL_ART_ID));
    expect(rung(VERSION_FULL_ART_ID)).toHaveAttribute("aria-pressed", "true");
    expect(equipped()).toEqual([VERSION_FULL_ART_ID]);
    fireEvent.blur(rung(VERSION_FULL_ART_ID));
    expect(rung(VERSION_FULL_ART_ID)).toHaveAttribute("aria-pressed", "true");
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_FULL_ART_ID });
  });

  it("R1834000 R3015007 a shown level writes no pick, so none is sent to the account, whether the view closes or the page is left", () => {
    writeCardSettings({ fullArt: false });
    writeCardPick(BIG_D, "border", "border-bone");
    const stored = localStorage.getItem(CARD_PICKS_SETTINGS_KEY);
    const writes: [string, CardPickChange][] = [];
    const unsubscribe = subscribeCardPickWrites((defId, change) => {
      writes.push([defId, change]);
    });
    open(BIG_D, OWNS_BIG_D_ART);

    fireEvent.focus(rung(VERSION_FULL_ART_ID));
    expect(large().getAttribute("data-full-art"), "the large card shows it").toBe("true");
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });
    fireEvent.click(screen.getByTestId(T.cardViewClose));
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });

    fireEvent.click(screen.getByTestId(T.card(BIG_D)));
    fireEvent.focus(rung(NO_FULL_ART_ID));
    fireEvent.focus(rung(VERSION_FULL_ART_ID));
    act(() => {
      window.dispatchEvent(new Event("pagehide"));
    });
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBe(stored);
    expect(writes, "no preview reaches the account's sync").toEqual([]);

    fireEvent.click(rung(VERSION_FULL_ART_ID));
    expect(writes).toEqual([[BIG_D, { fullArt: VERSION_FULL_ART_ID }]]);
    unsubscribe();
  });

  it("R2725001 R3015007 a card's ladders list only its offered levels", () => {
    // Carnivorous Cube, offered Full art's ladder with no full art of its own, and R2940000 Alt art's; Speciesist, its
    // Default; a Legacy card's ladders stop before animated (R2724000). R3017001: and each its side of the part 6 pairs.
    const ladders = (defId: string) => cardLadders(defId, () => true).map((ladder) => [ladder.variant, ladder.rungs.map((each) => each.id)]);
    const partSix = (variant: string, moves = true): [string, string[]] => [variant, [`version-${variant}`, ...(moves ? [`version-${variant}-animated`] : [])]];
    expect(ladders(CUBE)).toEqual([
      ["default", [NO_FULL_ART_ID, VERSION_ANIMATED_ID]],
      ["full-art", [VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID]],
      ["alt-art", [VERSION_ALT_ART_ID, VERSION_ALT_FULL_ART_ID, VERSION_ALT_FULL_ART_ANIMATED_ID, VERSION_ALT_FULL_ART_SIGNED_ID]],
      partSix("night"),
      partSix("watercolor"),
      partSix("tarot"),
    ]);
    expect(ladders(PLAIN)).toEqual([["default", [NO_FULL_ART_ID, VERSION_ANIMATED_ID]], partSix("neon"), partSix("watercolor"), partSix("woodcut")]);
    expect(ladders("traininggrounds-044")).toEqual([
      ["default", [NO_FULL_ART_ID]],
      ["full-art", [VERSION_FULL_ART_ID]],
      ["sketch", [VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID]],
      partSix("night", false),
      partSix("watercolor", false),
      partSix("tarot", false),
    ]);
    expect(cardStrip(CUBE, () => true).map((group) => group.kind)).toEqual(["foil", "finish", "border"]);
    // Owned on Speciesist by a grant no shop makes, Full art is listed nowhere, and a face there wears none.
    writeCardLevel(PLAIN, VERSION_FULL_ART_ID);
    open(PLAIN, new Map([[PLAIN, new Set([VERSION_FULL_ART_ID])]]));
    expect(screen.queryByTestId(T.ladder("full-art"))).toBeNull();
    expect(screen.queryByTestId(T.rung(VERSION_FULL_ART_ID))).toBeNull();
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(large().getAttribute("data-version")).toBeNull();
    cleanup();
    // On the Cube, Full art's first level is picked, worn on the large card, and needs no setting; the level above it is
    // next, the one past that locked.
    writeCardSettings({ fullArt: false });
    open(CUBE, new Map([[CUBE, new Set([VERSION_FULL_ART_ID])]]));
    expect(large().getAttribute("data-full-art")).toBeNull();
    fireEvent.click(rung(VERSION_FULL_ART_ID));
    fireEvent.mouseLeave(rung(VERSION_FULL_ART_ID));
    expect(readCardPicks(CUBE)).toEqual({ fullArt: VERSION_FULL_ART_ID });
    expect(large().getAttribute("data-full-art")).toBe("true");
    expect(large().getAttribute("data-version")).toBe(VERSION_FULL_ART_ID);
    expect(large().getAttribute("data-level")).toBe(VERSION_FULL_ART_ID);
    expect(rung(VERSION_FULL_ART_ANIMATED_ID)).toHaveAttribute("data-next", "true");
    expect(rung(VERSION_FULL_ART_SIGNED_ID)).toHaveTextContent("Needs Full art, animated");
  });

  it("R2949000 R3015007 Full art's signed level is its third rung: locked until the two below it are owned, it signs over the card's own picture", () => {
    open(CUBE, new Map([[CUBE, new Set([VERSION_FULL_ART_ID])]]));
    const signed = (): HTMLElement => rung(VERSION_FULL_ART_SIGNED_ID);
    // Past the next level it says what it needs, and a click opens its preview and writes nothing.
    expect(signed()).toHaveAttribute("data-owned", "false");
    expect(signed()).toHaveAttribute("data-next", "false");
    expect(signed()).toHaveTextContent("Needs Full art, animated");
    fireEvent.click(signed());
    expect(readCardPicks(CUBE)).toEqual({});
    cleanup();
    // With Full art's animated level owned too, it is next.
    open(CUBE, new Map([[CUBE, new Set([VERSION_FULL_ART_ID, VERSION_FULL_ART_ANIMATED_ID])]]));
    expect(signed()).toHaveAttribute("data-next", "true");
    expect(signed()).toHaveTextContent("Next level");
    expect(signed()).not.toHaveTextContent("Needs");
    cleanup();
    // Owned, it is a rung like any other, and the large card signs over its animated full art, drawing no picture of its own.
    open(CUBE);
    fireEvent.click(signed());
    fireEvent.mouseLeave(signed());
    expect(readCardPicks(CUBE)).toEqual({ fullArt: VERSION_FULL_ART_SIGNED_ID });
    expect(signed()).toHaveAttribute("data-equipped", "true");
    expect(large().querySelector(".cf-signature")).not.toBeNull();
    expect(large().getAttribute("data-full-art-tier")).toBe("Mythic");
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBeNull();
    // A level below it takes the signature off, and the normal face every level.
    fireEvent.click(rung(VERSION_FULL_ART_ANIMATED_ID));
    expect(readCardPicks(CUBE)).toEqual({ fullArt: VERSION_FULL_ART_ANIMATED_ID });
    expect(signed()).toHaveAttribute("data-equipped", "false");
    expect(large().querySelector(".cf-signature")).toBeNull();
    fireEvent.click(rung(NO_FULL_ART_ID));
    expect(readCardPicks(CUBE)).toEqual({ fullArt: false });
    expect(large().querySelector(".cf-signature")).toBeNull();
  });

  it("R1655000 Escape closes the view, but the card's text open over it takes the Escape first", () => {
    open(BIG_D);
    fireEvent.click(screen.getByTestId(T.cardViewText));
    expect(screen.getByTestId(INSPECT_DETAIL)).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    expect(screen.getByTestId(T.cardView)).toBeInTheDocument();

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(T.cardView)).toBeNull();
  });

  it("R1655000 Tab stays inside the view, and focus goes back to the card that opened it", () => {
    open(BIG_D);
    const close = screen.getByTestId(T.cardViewClose);
    expect(close).toHaveFocus();

    fireEvent.keyDown(close, { key: "Tab" });
    expect(screen.getByTestId(T.stripDefault("foil"))).toHaveFocus();
    fireEvent.keyDown(screen.getByTestId(T.stripDefault("foil")), { key: "Tab", shiftKey: true });
    expect(close).toHaveFocus();

    screen.getByTestId(T.card(BIG_D)).focus();
    fireEvent.click(close);
    expect(screen.getByTestId(T.card(BIG_D))).toHaveFocus();
  });

  it("R1655000 R3015007 cardStrip offers each kind's registry entries and says which are locked, and cardLadders each level of each variant", () => {
    const strip = cardStrip(BIG_D, (_defId, cosmeticId) => cosmeticId !== "foil-cracked-ice");

    expect(strip.map((group) => group.kind)).toEqual(["foil", "finish", "border"]);
    expect(strip[0]?.items.map((entry) => entry.id)).toEqual(cosmeticsOf("foil").map((cosmetic) => cosmetic.id));
    expect(strip.flatMap((group) => group.items).filter((entry) => !entry.owned).map((entry) => entry.id)).toEqual(["foil-cracked-ice"]);
    expect(strip[0]?.items[0]).toMatchObject({ id: "foil-none", name: "No foil", rarity: "Common", owned: true });
    // R3015007: the Default (the normal face, then its animated level), then Full art and R2940000 the Sketch, which Big
    // D-fender is offered, and R3017001 its side of each part 6 pair, each level a rung, the retired full arts and
    // augments nowhere; Full art owned nowhere.
    const ladders = cardLadders(BIG_D, (_defId, id) => !ladderOf("full-art").includes(id));
    expect(ladders.map((ladder) => [ladder.variant, ladder.name])).toEqual([
      ["default", "Default"],
      ["full-art", "Full art"],
      ["sketch", "Sketch"],
      ["night", "Night"],
      ["stained-glass", "Stained-glass window"],
      ["woodcut", "Woodcut"],
    ]);
    expect(ladders.map((ladder) => ladder.rungs.map((each) => [each.id, each.rarity]))).toEqual([
      [
        [NO_FULL_ART_ID, "Common"],
        [VERSION_ANIMATED_ID, "Epic"],
      ],
      [
        [VERSION_FULL_ART_ID, "Legendary"],
        [VERSION_FULL_ART_ANIMATED_ID, "Mythic"],
        [VERSION_FULL_ART_SIGNED_ID, "Mythic"],
      ],
      [
        [VERSION_SKETCH_ID, "Rare"],
        [VERSION_SKETCH_FULL_ART_ID, "Epic"],
        [VERSION_SKETCH_FULL_ART_ANIMATED_ID, "Legendary"],
        [VERSION_SKETCH_FULL_ART_SIGNED_ID, "Mythic"],
      ],
      [
        [VERSION_NIGHT_ID, "Rare"],
        [`${VERSION_NIGHT_ID}-animated`, "Epic"],
      ],
      [
        [VERSION_STAINED_GLASS_ID, "Epic"],
        [`${VERSION_STAINED_GLASS_ID}-animated`, "Legendary"],
      ],
      [
        [VERSION_WOODCUT_ID, "Rare"],
        [`${VERSION_WOODCUT_ID}-animated`, "Epic"],
      ],
    ]);
    // The first unowned level is next; each past it needs the highest level below it that is not owned.
    expect(ladders[1]?.rungs.map(({ owned, next, needs }) => ({ owned, next, needs }))).toEqual([
      { owned: false, next: true, needs: null },
      { owned: false, next: false, needs: "Full art" },
      { owned: false, next: false, needs: "Full art, animated" },
    ]);
    expect(ladders[0]?.rungs[0]).toMatchObject({ id: NO_FULL_ART_ID, name: "Normal", owned: true, next: false, needs: null });
    expect(cardStrip(PLAIN, () => true).map((group) => group.kind)).toEqual(["foil", "finish", "border"]);
    expect(cardLadders(PLAIN, () => true).map((ladder) => ladder.variant)).toEqual(["default", "neon", "watercolor", "woodcut"]);
  });

  it("R2940000 R3015007 the ladders list each new variant only where it is offered, each rung drawing its variant's picture", () => {
    // Transmogulate is offered Full art, Alt art and the Sketch (and R2942000 Comic, R2943000 Set showcase, R2944000
    // Immersive); Carnivorous Cube Alt art alone of the two new ones, Big D-fender the Sketch alone; Call to Chaos (Classic+
    // Edition) both and not Full art (and R2942000 Pixel and Comic, R2943000 Set showcase, R2944000 Immersive); Speciesist
    // neither. R3015000: Alt full art is Alt art's second level, no variant of its own. R3017001: each card its side of
    // each part 6 pair, last.
    const ALL = "core-083";
    const NOT_FULL = "classicplus-073";
    expect(firstLevels(ALL)).toEqual([
      ["default", NO_FULL_ART_ID, "Common"],
      ["full-art", VERSION_FULL_ART_ID, "Legendary"],
      ["alt-art", VERSION_ALT_ART_ID, "Rare"],
      ["sketch", VERSION_SKETCH_ID, "Rare"],
      ["comic", VERSION_COMIC_ID, "Epic"],
      ["set-showcase", VERSION_SET_SHOWCASE_ID, "Legendary"],
      ["immersive", VERSION_IMMERSIVE_ID, "Mythic"],
      ["neon", VERSION_NEON_ID, "Epic"],
      ["stained-glass", VERSION_STAINED_GLASS_ID, "Epic"],
      ["woodcut", VERSION_WOODCUT_ID, "Rare"],
    ]);
    expect(firstLevels(CUBE).map(([variant]) => variant)).toEqual(["default", "full-art", "alt-art", "night", "watercolor", "tarot"]);
    expect(firstLevels(BIG_D).map(([variant]) => variant)).toEqual(["default", "full-art", "sketch", "night", "stained-glass", "woodcut"]);
    expect(firstLevels(NOT_FULL).map(([variant]) => variant)).toEqual([
      "default",
      "alt-art",
      "sketch",
      "pixel",
      "comic",
      "set-showcase",
      "immersive",
      "neon",
      "stained-glass",
      "tarot",
    ]);
    expect(firstLevels(PLAIN).map(([variant]) => variant)).toEqual(["default", "neon", "watercolor", "woodcut"]);
    // Whatever the ladders list is offered there, and nothing offered is left out.
    for (const defId of [ALL, CUBE, BIG_D, NOT_FULL, PLAIN]) {
      for (const id of [...ladderOf("alt-art"), ...ladderOf("sketch")]) {
        expect(listed(defId).includes(id), `${id} on ${defId}`).toBe(offeredOn(id, defId));
      }
    }

    // R3015002: each rung draws its variant's picture: Alt art's at every level of Alt art, the normal one sketched at
    // every level of the Sketch.
    const all = defOf(ALL);
    render(<OwnedCards collection={collectionOwning([ALL])} defs={[all]} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card(ALL)));
    const picture = (id: string): string | null => rung(id).querySelector(".cf-art")?.getAttribute("data-art-picture") ?? null;
    expect(picture(VERSION_FULL_ART_ID)).toBeNull();
    for (const id of ladderOf("alt-art")) expect(picture(id), id).toBe("alt");
    for (const id of ladderOf("sketch")) expect(picture(id), id).toBe("sketch");
    // Choosing Alt full art wears it, full bleed, with Alt art's picture.
    fireEvent.click(rung(VERSION_ALT_FULL_ART_ID));
    expect(readCardPicks(ALL)).toEqual({ fullArt: VERSION_ALT_FULL_ART_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_ALT_FULL_ART_ID);
    expect(large().getAttribute("data-full-art")).toBe("true");
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBe("alt");
    expect(large().querySelector(".cf-signature")).toBeNull();
    // R3015000: its signed level, two rungs up, signs over that same picture.
    fireEvent.click(rung(VERSION_ALT_FULL_ART_SIGNED_ID));
    expect(readCardPicks(ALL)).toEqual({ fullArt: VERSION_ALT_FULL_ART_SIGNED_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_ALT_FULL_ART_ID);
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBe("alt");
    expect(large().querySelector(".cf-signature")).not.toBeNull();
  });

  it("R2942000 R3015007 the ladders list each part 3 variant only where it is offered, each rung drawing its own look", () => {
    // Spell Tyrant, a Legendary Unit, is offered the Sketch and all three, not Full art (and R2944000 Prismatic); Scratch
    // Pad, a Mythic Spell, Full art, Alt art (R3015000: which took Alt full art's cards) and all three (and R2944000
    // Immersive and Prismatic); Carnivorous Cube and Big D-fender none of the three; Speciesist nothing but, R3017001, its
    // side of each part 6 pair, as each card has.
    const TYRANT = "classic-056";
    const PAD = "meditativeplus-026";
    expect(firstLevels(TYRANT)).toEqual([
      ["default", NO_FULL_ART_ID, "Common"],
      ["sketch", VERSION_SKETCH_ID, "Rare"],
      ["extended-art", VERSION_EXTENDED_ART_ID, "Epic"],
      ["pixel", VERSION_PIXEL_ID, "Epic"],
      ["comic", VERSION_COMIC_ID, "Epic"],
      ["prismatic", VERSION_PRISMATIC_ID, "Mythic"],
      ["neon", VERSION_NEON_ID, "Epic"],
      ["watercolor", VERSION_WATERCOLOR_ID, "Rare"],
      ["tarot", VERSION_TAROT_ID, "Epic"],
    ]);
    expect(firstLevels(PAD).map(([variant]) => variant)).toEqual([
      "default",
      "full-art",
      "alt-art",
      "extended-art",
      "pixel",
      "comic",
      "immersive",
      "prismatic",
      "night",
      "stained-glass",
      "tarot",
    ]);
    expect(firstLevels(PLAIN).map(([variant]) => variant)).toEqual(["default", "neon", "watercolor", "woodcut"]);
    // Whatever the ladders list is offered there, and nothing offered is left out; each a ladder of two, art then animated.
    for (const defId of [TYRANT, PAD, CUBE, BIG_D, "core-083", "classicplus-073", PLAIN]) {
      for (const first of [VERSION_EXTENDED_ART_ID, VERSION_PIXEL_ID, VERSION_COMIC_ID]) {
        for (const id of [first, `${first}-animated`]) expect(listed(defId).includes(id), `${id} on ${defId}`).toBe(offeredOn(id, defId));
      }
    }
    expect(cardLadders(PAD, () => true).find((ladder) => ladder.variant === "pixel")?.rungs.map((each) => [each.id, each.rarity])).toEqual([
      [VERSION_PIXEL_ID, "Epic"],
      [`${VERSION_PIXEL_ID}-animated`, "Legendary"],
    ]);

    // Each rung draws its look across its tile: Extended art the normal picture, Pixel and Comic restyled, at both levels.
    const pad = defOf(PAD);
    render(<OwnedCards collection={collectionOwning([PAD])} defs={[pad]} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Meditative+")));
    fireEvent.click(screen.getByTestId(T.card(PAD)));
    const art = (id: string): Element | null => rung(id).querySelector(".cf-art");
    expect(art(VERSION_EXTENDED_ART_ID)?.getAttribute("data-art-picture")).toBeNull();
    expect(art(VERSION_EXTENDED_ART_ID)?.classList.contains("cf-art--full")).toBe(true);
    for (const id of [VERSION_PIXEL_ID, `${VERSION_PIXEL_ID}-animated`]) expect(art(id)?.getAttribute("data-art-picture"), id).toBe("pixel");
    for (const id of [VERSION_COMIC_ID, `${VERSION_COMIC_ID}-animated`]) expect(art(id)?.getAttribute("data-art-picture"), id).toBe("comic");
    // Choosing Pixel wears it, its words kept, with Pixel's picture.
    fireEvent.click(rung(VERSION_PIXEL_ID));
    expect(readCardPicks(PAD)).toEqual({ fullArt: VERSION_PIXEL_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_PIXEL_ID);
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(large().querySelector(".card-text")).not.toBeNull();
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBe("pixel");
    // R2942002, R3015000: its animated level moves that picture, and no rung signs it.
    fireEvent.click(rung(`${VERSION_PIXEL_ID}-animated`));
    expect(readCardPicks(PAD)).toEqual({ fullArt: `${VERSION_PIXEL_ID}-animated` });
    expect(large().getAttribute("data-version")).toBe(VERSION_PIXEL_ID);
    expect(large().getAttribute("data-art-motion")).not.toBeNull();
    expect(large().querySelector(".cf-signature")).toBeNull();
  });

  it("R2943000 R3015007 the ladders list each part 4 variant only where it is offered, each rung drawing its own look", () => {
    // BOOM! Big Max, a Legendary Unit, is offered both, beside Full art, Alt art and Pixel (and R2944000 Immersive);
    // Transmogulate, a Legendary Spell, Set showcase alone of the two; Carnivorous Cube, Big D-fender and Speciesist neither.
    // R3017001: each card its side of each part 6 pair, last.
    const MAX = "classic-080";
    expect(firstLevels(MAX)).toEqual([
      ["default", NO_FULL_ART_ID, "Common"],
      ["full-art", VERSION_FULL_ART_ID, "Legendary"],
      ["alt-art", VERSION_ALT_ART_ID, "Rare"],
      ["pixel", VERSION_PIXEL_ID, "Epic"],
      ["set-showcase", VERSION_SET_SHOWCASE_ID, "Legendary"],
      ["out-of-frame", VERSION_OUT_OF_FRAME_ID, "Legendary"],
      ["immersive", VERSION_IMMERSIVE_ID, "Mythic"],
      ["night", VERSION_NIGHT_ID, "Rare"],
      ["watercolor", VERSION_WATERCOLOR_ID, "Rare"],
      ["tarot", VERSION_TAROT_ID, "Epic"],
    ]);
    expect(firstLevels(PLAIN).map(([variant]) => variant)).toEqual(["default", "neon", "watercolor", "woodcut"]);
    // Whatever the ladders list is offered there, and nothing offered is left out.
    for (const defId of [MAX, "core-083", "classicplus-073", "classic-056", CUBE, BIG_D, PLAIN]) {
      for (const first of [VERSION_SET_SHOWCASE_ID, VERSION_OUT_OF_FRAME_ID]) {
        for (const id of [first, `${first}-animated`]) expect(listed(defId).includes(id), `${id} on ${defId}`).toBe(offeredOn(id, defId));
      }
    }

    // Each rung draws its look: Set showcase the picture in Classic's stone, Out of frame its subject alone, at both levels.
    const max = defOf(MAX);
    render(<OwnedCards collection={collectionOwning([MAX])} defs={[max]} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Classic")));
    fireEvent.click(screen.getByTestId(T.card(MAX)));
    const art = (id: string): Element | null => rung(id).querySelector(".cf-art");
    for (const id of [VERSION_SET_SHOWCASE_ID, `${VERSION_SET_SHOWCASE_ID}-animated`]) {
      expect(art(id)?.getAttribute("data-art-picture"), id).toBe("showcase-classic");
      expect(art(id)?.getAttribute("data-art-part"), id).toBeNull();
    }
    for (const id of [VERSION_OUT_OF_FRAME_ID, `${VERSION_OUT_OF_FRAME_ID}-animated`]) {
      expect(art(id)?.getAttribute("data-art-picture"), id).toBeNull();
      expect(art(id)?.getAttribute("data-art-part"), id).toBe("subject");
    }
    // Choosing Set showcase wears Classic's look, its words kept.
    fireEvent.click(rung(VERSION_SET_SHOWCASE_ID));
    expect(readCardPicks(MAX)).toEqual({ fullArt: VERSION_SET_SHOWCASE_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_SET_SHOWCASE_ID);
    expect(large().getAttribute("data-showcase")).toBe("classic");
    expect(large().querySelector(".card-text")).not.toBeNull();
    // Choosing Out of frame cuts its subject out over the frame, its words kept.
    fireEvent.click(rung(VERSION_OUT_OF_FRAME_ID));
    expect(readCardPicks(MAX)).toEqual({ fullArt: VERSION_OUT_OF_FRAME_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_OUT_OF_FRAME_ID);
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(large().querySelector(".card-text")).not.toBeNull();
    expect(large().querySelector(".cf-art-frame > .cf-art--cutout")?.getAttribute("data-art-part")).toBe("subject");
    // R2943002, R3015000: its animated level moves the cut-out picture, and no rung signs it.
    fireEvent.click(rung(`${VERSION_OUT_OF_FRAME_ID}-animated`));
    expect(readCardPicks(MAX)).toEqual({ fullArt: `${VERSION_OUT_OF_FRAME_ID}-animated` });
    expect(large().getAttribute("data-version")).toBe(VERSION_OUT_OF_FRAME_ID);
    expect(large().getAttribute("data-art-motion")).not.toBeNull();
    expect(large().querySelector(".cf-art-frame > .cf-art--cutout")?.getAttribute("data-art-part")).toBe("subject");
    expect(large().querySelector(".cf-signature")).toBeNull();
  });

  it("R2944000 R3015007 the ladders list each part 5 variant only where it is offered, each rung drawing its own look", () => {
    // Ceaseless Void, a Mythic Unit, is offered both, last, beside Full art, Alt art (R3015000: which took Alt full art's
    // cards), the Sketch, Extended art, Set showcase and Out of frame; Spell Tyrant, a Legendary Unit, Prismatic alone of
    // the two, and BOOM! Big Max Immersive alone; Carnivorous Cube, Big D-fender and Speciesist neither. R3017001: each card
    // its side of each part 6 pair, last.
    const VOID = "core-100";
    expect(firstLevels(VOID)).toEqual([
      ["default", NO_FULL_ART_ID, "Common"],
      ["full-art", VERSION_FULL_ART_ID, "Legendary"],
      ["alt-art", VERSION_ALT_ART_ID, "Rare"],
      ["sketch", VERSION_SKETCH_ID, "Rare"],
      ["extended-art", VERSION_EXTENDED_ART_ID, "Epic"],
      ["set-showcase", VERSION_SET_SHOWCASE_ID, "Legendary"],
      ["out-of-frame", VERSION_OUT_OF_FRAME_ID, "Legendary"],
      ["immersive", VERSION_IMMERSIVE_ID, "Mythic"],
      ["prismatic", VERSION_PRISMATIC_ID, "Mythic"],
      ["night", VERSION_NIGHT_ID, "Rare"],
      ["stained-glass", VERSION_STAINED_GLASS_ID, "Epic"],
      ["woodcut", VERSION_WOODCUT_ID, "Rare"],
    ]);
    expect(firstLevels(PLAIN).map(([variant]) => variant)).toEqual(["default", "neon", "watercolor", "woodcut"]);
    // Whatever the ladders list is offered there, and nothing offered is left out.
    for (const defId of [VOID, "classic-056", "classic-080", "core-083", "classicplus-073", "meditativeplus-026", CUBE, BIG_D, PLAIN]) {
      for (const first of [VERSION_IMMERSIVE_ID, VERSION_PRISMATIC_ID]) {
        for (const id of [first, `${first}-animated`]) expect(listed(defId).includes(id), `${id} on ${defId}`).toBe(offeredOn(id, defId));
      }
    }

    // Each rung draws its look: Immersive the whole picture hazed, Prismatic in chrome, at both levels.
    const voidDef = defOf(VOID);
    render(<OwnedCards collection={collectionOwning([VOID])} defs={[voidDef]} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card(VOID)));
    const art = (id: string): Element | null => rung(id).querySelector(".cf-art");
    for (const id of [VERSION_IMMERSIVE_ID, `${VERSION_IMMERSIVE_ID}-animated`]) {
      expect(art(id)?.getAttribute("data-art-picture"), id).toBe("depth");
      expect(art(id)?.getAttribute("data-art-part"), id).toBeNull();
    }
    for (const id of [VERSION_PRISMATIC_ID, `${VERSION_PRISMATIC_ID}-animated`]) expect(art(id)?.getAttribute("data-art-picture"), id).toBe("prismatic");
    // Choosing Immersive lays the subject on its near layer over the hazed backdrop, its words kept.
    fireEvent.click(rung(VERSION_IMMERSIVE_ID));
    expect(readCardPicks(VOID)).toEqual({ fullArt: VERSION_IMMERSIVE_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_IMMERSIVE_ID);
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(large().querySelector(".card-text")).not.toBeNull();
    expect(large().querySelector(".cf-art-frame > .cf-art")?.getAttribute("data-art-part")).toBe("backdrop");
    expect(large().querySelector(".cf-art-frame > .cf-immersive > .cf-art--near")?.getAttribute("data-art-part")).toBe("subject");
    // Choosing Prismatic draws it in chrome, its words kept.
    fireEvent.click(rung(VERSION_PRISMATIC_ID));
    expect(readCardPicks(VOID)).toEqual({ fullArt: VERSION_PRISMATIC_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_PRISMATIC_ID);
    expect(large().querySelector(".card-text")).not.toBeNull();
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBe("prismatic");
    expect(large().querySelector(".cf-immersive")).toBeNull();
    // R2944002, R3015000: its animated level moves the chrome, and no rung signs it.
    fireEvent.click(rung(`${VERSION_PRISMATIC_ID}-animated`));
    expect(readCardPicks(VOID)).toEqual({ fullArt: `${VERSION_PRISMATIC_ID}-animated` });
    expect(large().getAttribute("data-version")).toBe(VERSION_PRISMATIC_ID);
    expect(large().getAttribute("data-art-motion")).not.toBeNull();
    expect(large().querySelector(".cf-signature")).toBeNull();
  });

  it("R3017000 the Collection lists only the part 6 variants a card is offered", () => {
    // R3017001: Speciesist, a Common Unit, is offered Neon sign, Watercolor and Woodcut, and Scratch Pad, a Mythic Spell,
    // their partners, Night, Stained-glass window and Tarot card; each its two levels, art then animated.
    const PAD = "meditativeplus-026";
    const PART_SIX = [VERSION_NIGHT_ID, VERSION_NEON_ID, VERSION_WATERCOLOR_ID, VERSION_STAINED_GLASS_ID, VERSION_WOODCUT_ID, VERSION_TAROT_ID];
    const sides: readonly (readonly [string, readonly string[]])[] = [
      [PLAIN, [VERSION_NEON_ID, VERSION_WATERCOLOR_ID, VERSION_WOODCUT_ID]],
      [PAD, [VERSION_NIGHT_ID, VERSION_STAINED_GLASS_ID, VERSION_TAROT_ID]],
    ];
    for (const [defId, mine] of sides) {
      expect(firstLevels(defId).filter(([, id]) => PART_SIX.includes(id ?? "")).map(([, id]) => id), defId).toEqual(mine);
      for (const first of PART_SIX) {
        for (const id of [first, `${first}-animated`]) expect(listed(defId).includes(id), `${id} on ${defId}`).toBe(mine.includes(first));
      }
    }
    // The open view shows those ladders and no other part 6 one, each rung drawing its variant's picture.
    const pictures: Readonly<Record<string, string>> = {
      [VERSION_NEON_ID]: "neon",
      [VERSION_WATERCOLOR_ID]: "watercolor",
      [VERSION_WOODCUT_ID]: "woodcut",
    };
    open(PLAIN);
    for (const first of PART_SIX) {
      const variant = first.slice("version-".length);
      const shown = Object.hasOwn(pictures, first);
      expect(screen.queryByTestId(T.ladder(variant)) !== null, variant).toBe(shown);
      if (!shown) continue;
      for (const id of [first, `${first}-animated`]) expect(rung(id).querySelector(".cf-art")?.getAttribute("data-art-picture"), id).toBe(pictures[first]);
    }
    // Choosing one wears it in the normal frame, its words kept.
    fireEvent.click(rung(VERSION_WOODCUT_ID));
    expect(readCardPicks(PLAIN)).toEqual({ fullArt: VERSION_WOODCUT_ID });
    expect(large().getAttribute("data-version")).toBe(VERSION_WOODCUT_ID);
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(large().querySelector(".card-text")).not.toBeNull();
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBe("woodcut");
  });

  it("R3015007 each variant is a ladder: owned, next and locked", () => {
    // Big D-fender, owning Full art's first level (its retired still Epic, R3015006) and the Sketch's first two.
    open(BIG_D, owning(BIG_D_ART, VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID));
    // One section per variant, the Default first, each named.
    const sections = screen.getAllByTestId(/^collection-ladder-/u);
    expect(sections.map((section) => [section.dataset.variant, section.getAttribute("aria-label")])).toEqual([
      ["default", "Default"],
      ["full-art", "Full art"],
      ["sketch", "Sketch"],
      ["night", "Night"],
      ["stained-glass", "Stained-glass window"],
      ["woodcut", "Woodcut"],
    ]);
    // Each an ordered list of its levels, lowest first, each rung its level's state: owned, next or locked.
    const state = (variant: string): (string | undefined)[][] =>
      within(screen.getByTestId(T.ladder(variant)))
        .getAllByRole("listitem")
        .map((each) => {
          const button = within(each).getByRole("button");
          return [button.dataset.level, button.dataset.owned, button.dataset.next];
        });
    for (const section of sections) expect(section.querySelector(":scope > ol"), section.dataset.variant).not.toBeNull();
    expect(state("default")).toEqual([
      [NO_FULL_ART_ID, "true", "false"],
      [VERSION_ANIMATED_ID, "false", "true"],
    ]);
    expect(state("full-art")).toEqual([
      [VERSION_FULL_ART_ID, "true", "false"],
      [VERSION_FULL_ART_ANIMATED_ID, "false", "true"],
      [VERSION_FULL_ART_SIGNED_ID, "false", "false"],
    ]);
    expect(state("sketch")).toEqual([
      [VERSION_SKETCH_ID, "true", "false"],
      [VERSION_SKETCH_FULL_ART_ID, "true", "false"],
      [VERSION_SKETCH_FULL_ART_ANIMATED_ID, "false", "true"],
      [VERSION_SKETCH_FULL_ART_SIGNED_ID, "false", "false"],
    ]);
    // A next rung says so; a locked one past it names the level it needs first; an owned one says neither.
    expect(rung(VERSION_SKETCH_FULL_ART_ANIMATED_ID)).toHaveTextContent("Next level");
    expect(rung(VERSION_SKETCH_FULL_ART_SIGNED_ID)).toHaveTextContent("Needs Sketch full art, animated");
    expect(rung(VERSION_FULL_ART_SIGNED_ID)).toHaveTextContent("Needs Full art, animated");
    for (const id of [VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID, VERSION_FULL_ART_ID]) {
      expect(rung(id), id).not.toHaveTextContent("Next level");
      expect(rung(id), id).not.toHaveTextContent("Needs");
    }
    // An owned rung is worn on a click, the one worn marked; a locked one opens its preview and wears nothing.
    fireEvent.click(rung(VERSION_SKETCH_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_SKETCH_FULL_ART_ID });
    expect(rung(VERSION_SKETCH_FULL_ART_ID)).toHaveAttribute("data-equipped", "true");
    expect(rung(VERSION_SKETCH_FULL_ART_ID)).toHaveAttribute("aria-pressed", "true");
    expect(large().getAttribute("data-level")).toBe(VERSION_SKETCH_FULL_ART_ID);
    expect(large().getAttribute("data-full-art")).toBe("true");
    expect(large().querySelector(".cf-art")?.getAttribute("data-art-picture")).toBe("sketch");
    fireEvent.click(rung(VERSION_SKETCH_FULL_ART_SIGNED_ID));
    expect(screen.getByTestId(T.preview)).toBeInTheDocument();
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: VERSION_SKETCH_FULL_ART_ID });
    expect(rung(VERSION_SKETCH_FULL_ART_SIGNED_ID)).toHaveAttribute("data-equipped", "false");
  });
});

describe("R3015007 a held token on a ladder", () => {
  const TOKEN = "token-1";
  const BIG_D = "core-001";
  const HELD = { card: 0, cosmeticRare: 9, cosmeticEpic: 9, cosmeticLegendary: 9, cosmeticPack: 0 };

  afterEach(() => {
    vi.unstubAllGlobals();
    resetOwnedFullArtForTests();
  });

  it("R3015007 a token is offered on the next level alone", async () => {
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: { ...tokenBars(), tokens: HELD } };
      if (call.method === "POST" && call.path === "/api/tokens/redeem-cosmetic") {
        return { status: 200, body: { ...tokenBars(), tokens: { ...HELD, cosmeticRare: HELD.cosmeticRare - 1 } } };
      }
      return undefined;
    });
    // Nothing owned on Big D-fender, then, once the token is spent, the Sketch's first level.
    const read = vi
      .fn<(token: string) => Promise<OwnedByCard>>()
      .mockResolvedValueOnce(new Map())
      .mockResolvedValue(new Map([[BIG_D, new Set([VERSION_SKETCH_ID])]]));
    render(
      <ViewerFullArt token={TOKEN} gated read={read}>
        <OwnedCards collection={collectionOf()} defs={[defOf(BIG_D)]} token={TOKEN} />
      </ViewerFullArt>,
    );
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card(BIG_D)));
    const previewOf = (id: string): HTMLElement => {
      fireEvent.keyDown(screen.getByTestId(T.rung(id)), { key: "i" });
      return screen.getByTestId(T.preview);
    };
    const closePreview = (): void => {
      fireEvent.keyDown(document.body, { key: "Escape" });
      expect(screen.queryByTestId(T.preview)).toBeNull();
    };

    // Each variant's next level offers the cheapest held token covering it: the Default's animated level an Epic, Full
    // art's first a Legendary, the Sketch's first a Rare.
    for (const [id, words] of [
      [VERSION_ANIMATED_ID, "an Epic token"],
      [VERSION_FULL_ART_ID, "a Legendary token"],
      [VERSION_SKETCH_ID, "a Rare token"],
    ] as const) {
      expect(screen.getByTestId(T.rung(id)), id).toHaveAttribute("data-next", "true");
      expect(await within(previewOf(id)).findByTestId(T.tokenOffer(id)), id).toHaveTextContent(`Unlock with ${words} (9 held)`);
      closePreview();
    }
    // R3015004: a level past the next offers none, though a held token covers it: the Sketch's full art, an Epic.
    expect(screen.getByTestId(T.rung(VERSION_SKETCH_FULL_ART_ID))).toHaveAttribute("data-next", "false");
    previewOf(VERSION_SKETCH_FULL_ART_ID);
    await settle();
    expect(within(screen.getByTestId(T.preview)).queryByTestId(T.tokenOffer(VERSION_SKETCH_FULL_ART_ID))).toBeNull();
    closePreview();

    // Spending one on the Sketch's first level owns it, and the level above it is next and offered in turn.
    fireEvent.click(await within(previewOf(VERSION_SKETCH_ID)).findByTestId(T.tokenOffer(VERSION_SKETCH_ID)));
    fireEvent.click(screen.getByTestId(T.tokenOfferConfirm(VERSION_SKETCH_ID)));
    await waitFor(() => {
      expect(screen.getByTestId(T.rung(VERSION_SKETCH_ID))).toHaveAttribute("data-owned", "true");
    });
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")).toHaveLength(1);
    expect(server.to("POST", "/api/tokens/redeem-cosmetic")[0]?.body).toEqual({ token: "Rare", cosmeticId: VERSION_SKETCH_ID, cardId: BIG_D });
    closePreview();
    expect(screen.getByTestId(T.rung(VERSION_SKETCH_FULL_ART_ID))).toHaveAttribute("data-next", "true");
    expect(await within(previewOf(VERSION_SKETCH_FULL_ART_ID)).findByTestId(T.tokenOffer(VERSION_SKETCH_FULL_ART_ID))).toHaveTextContent(
      "Unlock with an Epic token (9 held)",
    );
  });
});

describe("R1956000 the owned cards' grid wears their picks", () => {
  const PLAIN = "core-002";
  const MYTHIC = "core-096";
  const LOCKED = "core-005";
  const DEFS_HERE = [PLAIN, MYTHIC, LOCKED].map(defOf);
  const ATTRIBUTES = ["data-cosmetic-foil", "data-cosmetic-motion", "data-finish", "data-frame"] as const;

  const tile = (id: string): Element => {
    const face = screen.getByTestId(T.card(id)).querySelector(".cf");
    if (face === null) throw new Error(`${id}'s tile draws no face`);
    return face;
  };

  it("R1956000 an owned grid tile wears its card's picks, never the account's; a locked tile and a Radiant face do not, and a Mythic tile's foil follows Rarity foil (R2721000)", () => {
    writeCosmetics({ finish: "finish-linen" });
    writeCardPick(PLAIN, "border", "border-bone");
    writeCardPick(PLAIN, "foil", "foil-rainbow");
    writeCardPick(MYTHIC, "border", "border-bone");
    writeCardPick(MYTHIC, "foil", "foil-rainbow");
    writeCardPick(LOCKED, "border", "border-bone");
    render(<OwnedCards collection={collectionOwning([PLAIN, MYTHIC])} defs={DEFS_HERE} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));

    const plain = tile(PLAIN);
    expect(plain.getAttribute("data-frame")).toBe("border-bone");
    // R1953001: a kind the card has no pick of wears its default, not the account's pick.
    expect(plain.getAttribute("data-finish")).toBeNull();
    expect(plain.getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    // A grid's foil holds still: the moving slots stay with the card a player opens.
    expect(plain.getAttribute("data-cosmetic-motion")).toBe("static");
    // A Mythic face wears its cosmetic foil while no rarity foil shows (R2721000), and its other picks.
    expect(tile(MYTHIC).getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    expect(tile(MYTHIC).getAttribute("data-frame")).toBe("border-bone");
    act(() => {
      writeCardSettings({ rarityFoil: true });
    });
    expect(tile(MYTHIC).getAttribute("data-cosmetic-foil")).toBeNull();
    expect(tile(MYTHIC).getAttribute("data-frame")).toBe("border-bone");
    // A locked silhouette is no copy of the viewer's.
    for (const attribute of ATTRIBUTES) expect(tile(LOCKED).getAttribute(attribute), attribute).toBeNull();

    // A pick made in this tab reaches the same tile on the same render.
    act(() => {
      writeCardPick(PLAIN, "border", "border-wood");
    });
    expect(tile(PLAIN)).toBe(plain);
    expect(plain.getAttribute("data-frame")).toBe("border-wood");

    // The grid draws base faces; a Radiant face of the same card wears nothing.
    cleanup();
    const { container } = render(<CardFace face={faceModel({ defId: PLAIN, def: defOf(PLAIN), radiant: true })} worn />);
    const radiant = container.querySelector(".cf");
    for (const attribute of ATTRIBUTES) expect(radiant?.getAttribute(attribute), attribute).toBeNull();
  });

  it("R1956000 a grid tile wears only what the ledger owns on its card", () => {
    // A starter every account owns on every card, and a pick the ledger does not list on this one.
    writeCardPick(PLAIN, "border", "border-bone");
    writeCardPick(PLAIN, "finish", "finish-parchment");
    render(
      <FullArtOwned owned={new Map()}>
        <OwnedCards collection={collectionOwning([PLAIN])} defs={DEFS_HERE} />
      </FullArtOwned>,
    );
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    expect(tile(PLAIN).getAttribute("data-frame")).toBe("border-bone");
    expect(tile(PLAIN).getAttribute("data-finish")).toBeNull();
  });
});

describe("R1655000 the account's cosmetics as tiles", () => {
  it("R1655000 R1833000 accountTiles lists a kind's entries, a default and a starter owned and the rest as the account's list says", () => {
    const none = accountTiles("table", (id) => ownsAccountCosmetic(new Set(), id));
    expect(none.map((tile) => [tile.cosmetic.id, tile.owned])).toEqual([
      ["table-tavern", true],
      ["table-wood", true],
      ["table-felt", true],
      ["table-neon-grid", true],
      ["table-marble", false],
      ["table-chessboard", false],
      ["table-desert-sand", false],
      ["table-obsidian-slab", false],
      ["table-ancient-map", false],
      ["table-astral", false],
    ]);
    expect(accountTiles("table", (id) => ownsAccountCosmetic(new Set(["table-astral"]), id)).at(-1)?.owned).toBe(true);
    expect(accountTiles("table", (id) => ownsAccountCosmetic("every", id)).every((tile) => tile.owned)).toBe(true);
    expect(accountTiles("cardBack", () => false).map((tile) => tile.cosmetic.id)).toEqual(cosmeticsOf("cardBack").map((cosmetic) => cosmetic.id));
  });

  it("R1655000 a playmat and a backdrop are drawn together as a mini board, a card back as a back", () => {
    render(<AccountCosmeticTiles />);

    // A playmat tile is its playmat over the equipped backdrop; a backdrop tile, its backdrop over the equipped playmat.
    const felt = screen.getByTestId(T.tile("table-felt"));
    expect(felt.querySelector(".collection-miniboard")).toHaveAttribute("data-table", "table-felt");
    expect(felt.querySelector(".board-backdrop")).toBeNull();
    const rain = screen.getByTestId(T.tile("field-rain"));
    expect(rain.querySelector(".collection-miniboard")).toHaveAttribute("data-table", "table-tavern");
    expect(rain.querySelector(".board-backdrop")).toHaveAttribute("data-backdrop", "field-rain");
    expect(screen.getByTestId(T.tile("field-none")).querySelector(".board-backdrop")).toBeNull();

    expect(screen.getByTestId(T.tile("card-back-starfield")).querySelector(".cf-back")).toHaveAttribute("data-card-back", "card-back-starfield");
    expect(screen.getByTestId(T.tile("card-back-standard")).querySelector(".cf-back")).not.toHaveAttribute("data-card-back");
    expect(screen.getByTestId(T.tile("table-tavern"))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.tile("table-felt"))).toHaveAttribute("data-equipped", "false");

    act(() => {
      writeCosmetics({ table: "table-wood", field: "field-rain" });
    });
    expect(screen.getByTestId(T.tile("table-wood"))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.tile("field-rain"))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.tile("field-embers")).querySelector(".collection-miniboard")).toHaveAttribute("data-table", "table-wood");
    expect(screen.getByTestId(T.tile("table-felt")).querySelector(".board-backdrop")).toHaveAttribute("data-backdrop", "field-rain");
  });

  it("R1655000 R1833000 clicking an owned tile equips it through the cosmetics store; a locked one shows its rarity and equips nothing", () => {
    const { rerender } = render(
      <AccountCosmeticsOwned owned={new Set()}>
        <AccountCosmeticTiles />
      </AccountCosmeticsOwned>,
    );

    fireEvent.click(within(screen.getByTestId(T.tile("card-back-lacquer"))).getByRole("button"));
    expect(readCosmetics().cardBack).toBe("card-back-lacquer");
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "null")).toMatchObject({ cardBack: "card-back-lacquer" });
    expect(screen.getByTestId(T.tile("card-back-lacquer"))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.tile("card-back-standard"))).toHaveAttribute("data-equipped", "false");

    const locked = screen.getByTestId(T.tile("table-astral"));
    expect(locked).toHaveAttribute("data-owned", "false");
    expect(locked).toHaveTextContent("Mythic");
    expect(locked).toHaveTextContent("Locked");
    expect(within(locked).getByRole("button")).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(within(locked).getByRole("button"));
    expect(readCosmetics().table).toBe("table-tavern");

    rerender(
      <AccountCosmeticsOwned owned={new Set(["table-astral"])}>
        <AccountCosmeticTiles />
      </AccountCosmeticsOwned>,
    );
    expect(screen.getByTestId(T.tile("table-astral"))).toHaveAttribute("data-owned", "true");
    fireEvent.click(within(screen.getByTestId(T.tile("table-astral"))).getByRole("button"));
    expect(readCosmetics().table).toBe("table-astral");
    expect(screen.getByTestId(T.tile("table-astral"))).toHaveAttribute("data-equipped", "true");
  });
});

describe("R3013001 a card's proficiency in its view", () => {
  const PLAYED = "core-002";
  const UNPLAYED = "core-004";
  const TOPPED = "core-001";
  const LOCKED = "core-022";
  const CARDS = [PLAYED, UNPLAYED, TOPPED, LOCKED].map(defOf);
  const TOP = PROFICIENCY_LEVEL_PLAYS[PROFICIENCY_LEVELS - 1] ?? 0;

  beforeEach(() => {
    resetProficiencyForTests();
    replaceOwnProficiency([
      { cardId: PLAYED, plays: 20, level: 2, stamp: 1 },
      { cardId: TOPPED, plays: TOP + 7, level: PROFICIENCY_LEVELS, stamp: null },
    ]);
  });

  afterEach(() => {
    resetProficiencyForTests();
  });

  /** Opens `id`, owning every card but LOCKED. */
  function open(id: string): void {
    render(<OwnedCards collection={collectionOwning(CARDS.filter((def) => def.id !== LOCKED).map((def) => def.id))} defs={CARDS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card(id)));
  }
  const stamp = (level: number): HTMLElement => screen.getByTestId(T.stamp(level));
  const largeStamp = (): string | null =>
    screen.getByTestId(T.cardViewFace).querySelector(".cf .cf-stamp")?.getAttribute("data-stamp") ?? null;

  it("R3013001 the card view shows the level, the progress and the stamps, locked ones with their level", () => {
    open(PLAYED);
    const panel = screen.getByTestId(T.proficiency);
    expect(within(panel).getByTestId(T.proficiencyLevel)).toHaveTextContent(`Level 2 of ${String(PROFICIENCY_LEVELS)}`);
    const next = PROFICIENCY_LEVEL_PLAYS[2] ?? 0;
    const reached = PROFICIENCY_LEVEL_PLAYS[1] ?? 0;
    const plays = within(panel).getByTestId(T.proficiencyPlays);
    expect(plays).toHaveTextContent(`20 of ${String(next)} plays to the next level`);
    // The bar runs between the level's plays and the next's.
    const progress = plays.querySelector("progress");
    expect(progress?.getAttribute("max")).toBe(String(next - reached));
    expect(progress?.getAttribute("value")).toBe(String(20 - reached));
    // Every level's stamp is listed; those past the card's level are locked with the level each needs.
    for (const level of STAMP_LEVELS) {
      const button = stamp(level);
      expect(button.querySelector(`.cf-stamp[data-stamp="${String(level)}"]`), String(level)).not.toBeNull();
      if (level <= 2) {
        expect(button, String(level)).not.toHaveAttribute("aria-disabled");
        expect(button, String(level)).not.toHaveTextContent("Unlocks");
      } else {
        expect(button, String(level)).toHaveAttribute("aria-disabled", "true");
        expect(button, String(level)).toHaveTextContent(`Unlocks at level ${String(level)}`);
      }
    }
    expect(stamp(1)).toHaveAttribute("aria-pressed", "true");
    expect(stamp(2)).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByTestId(T.stampNone)).toHaveAttribute("aria-pressed", "false");
    // The large face wears the card's stamp.
    expect(largeStamp()).toBe("1");

    // A card never played is at level 0, every stamp locked, the first level PROFICIENCY_LEVEL_PLAYS[0] plays away.
    fireEvent.click(screen.getByTestId(T.card(UNPLAYED)));
    expect(screen.getByTestId(T.proficiencyLevel)).toHaveTextContent(`Level 0 of ${String(PROFICIENCY_LEVELS)}`);
    expect(screen.getByTestId(T.proficiencyPlays)).toHaveTextContent(`0 of ${String(PROFICIENCY_LEVEL_PLAYS[0] ?? 0)} plays to the next level`);
    for (const level of STAMP_LEVELS) expect(stamp(level)).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByTestId(T.stampNone)).toHaveAttribute("aria-pressed", "true");
    expect(largeStamp()).toBeNull();

    // A card at the top has nothing more to reach, and every stamp unlocked.
    fireEvent.click(screen.getByTestId(T.card(TOPPED)));
    expect(screen.getByTestId(T.proficiencyLevel)).toHaveTextContent(`Level ${String(PROFICIENCY_LEVELS)} of ${String(PROFICIENCY_LEVELS)}`);
    expect(screen.getByTestId(T.proficiencyPlays)).toHaveTextContent("Top level reached.");
    for (const level of STAMP_LEVELS) expect(stamp(level)).not.toHaveAttribute("aria-disabled");

    // A card not owned shows no proficiency.
    fireEvent.click(screen.getByTestId(T.card(LOCKED)));
    expect(screen.queryByTestId(T.proficiency)).toBeNull();
  });

  it("R3013001 an unlocked stamp is picked and a locked one writes nothing", () => {
    const writes: [string, number | null][] = [];
    subscribeStampWrites((defId, picked) => {
      writes.push([defId, picked]);
    });
    open(PLAYED);
    fireEvent.click(stamp(2));
    expect(writes).toEqual([[PLAYED, 2]]);
    expect(readOwnProficiency(PLAYED)?.stamp).toBe(2);
    expect(stamp(2)).toHaveAttribute("aria-pressed", "true");
    expect(stamp(1)).toHaveAttribute("aria-pressed", "false");
    expect(largeStamp()).toBe("2");

    fireEvent.click(stamp(3));
    expect(writes).toHaveLength(1);
    expect(readOwnProficiency(PLAYED)?.stamp).toBe(2);

    fireEvent.click(screen.getByTestId(T.stampNone));
    expect(writes).toEqual([
      [PLAYED, 2],
      [PLAYED, null],
    ]);
    expect(screen.getByTestId(T.stampNone)).toHaveAttribute("aria-pressed", "true");
    expect(largeStamp()).toBeNull();
  });
});
