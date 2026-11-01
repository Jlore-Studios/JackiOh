// The collection screen's panels (#1307, SPEC §14), from fixture responses (nothing here asks the
// server anything): owned cards, wallet and tracks. R1655000: a card in context and the account's
// cosmetics as tiles. R1956000: the owned cards' grid wears their picks.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLOBAL_PACK_NAME, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
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
  writeCardPick,
  writeCosmetics,
  type CardPickChange,
} from "../cards/cosmetics.ts";
import { CHINESE } from "../cards/chinese.ts";
import { FullArtOwned } from "../cards/fullArt.ts";
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
import { cosmeticPacksInfo, ownedCosmeticsOf, walletInfo } from "../shop/fixtures.ts";
import { expectEmptyState } from "../test/empty.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import AccountCosmeticTiles from "./AccountCosmeticTiles.tsx";
import OwnedCosmetics from "./CosmeticsPanel.tsx";
import OwnedCards from "./OwnedCards.tsx";
import TracksPanel from "./TracksPanel.tsx";
import WalletPanel from "./WalletPanel.tsx";
import {
  COLLECTED_DEFS,
  NO_FULL_ART_ID,
  accountTiles,
  cardNameOf,
  cardStrip,
  collectionSectionOf,
  collectionTestid as T,
  cosmeticKindWords,
  ownedCosmetics,
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
    const title = view.querySelector(".collection-cardview__title")?.textContent ?? "";
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
    expect(opened.view).toHaveAttribute("aria-label", defOf("core-002").name);
    expect(opened.title).toBe(defOf("core-002").name);
    expect(opened.detail).toBe(defOf("core-002").name);
  });

  it("#2053 for a reader of Chinese they are labeled with the Chinese table's name", async () => {
    await setLocale("zh-CN");
    const opened = openLocked();

    expect(opened.tile).toBe(CHINESE["core-001"]!.name);
    expect(opened.locked).toBe(`${CHINESE["core-002"]!.name}，尚未拥有`);
    expect(opened.view).toHaveAttribute("aria-label", CHINESE["core-002"]!.name);
    expect(opened.title).toBe(CHINESE["core-002"]!.name);
    expect(opened.detail).toBe(CHINESE["core-002"]!.name);
  });

  it("#2053 an owned per-card cosmetic names its card in the reader's language", async () => {
    await setLocale("zh-CN");
    render(<OwnedCosmetics owned={ownedCosmeticsOf()} />);

    expect(screen.getByTestId(T.ownedCosmetic("full-art-gary-the-gambler", "core-004"))).toHaveTextContent(
      `用于${CHINESE["core-004"]!.name}`,
    );
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
  const starters = COSMETICS.filter((cosmetic) => cosmetic.starter && !cosmetic.default);

  it("R1579000 ownedCosmetics lists every starter but the defaults once, then each per-card cosmetic on its card", () => {
    const none = ownedCosmetics({ cards: [], account: [] });
    expect(none.map((entry) => entry.cosmetic.id)).toEqual(starters.map((cosmetic) => cosmetic.id));
    expect(none.every((entry) => entry.cardId === null)).toBe(true);
    expect(none.some((entry) => entry.cosmetic.default)).toBe(false);

    const some = ownedCosmetics(
      ownedCosmeticsOf({
        cards: [
          { cardId: "core-004", cosmeticId: "full-art-gary-the-gambler" },
          { cardId: "core-001", cosmeticId: "full-art-big-d-fender" },
          { cardId: "core-001", cosmeticId: "full-art-unknown-to-this-client" },
        ],
        account: ["table-felt", "table-unknown-to-this-client"],
      }),
    );
    // The registry's order: the starters, once each (table-felt is in `account` too), then the full arts.
    expect(some.map((entry) => [entry.cosmetic.id, entry.cardId])).toEqual([
      ...starters.map((cosmetic) => [cosmetic.id, null]),
      ["full-art-big-d-fender", "core-001"],
      ["full-art-gary-the-gambler", "core-004"],
    ]);
  });

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

describe("R1827000 the owned portraits", () => {
  it("R1827000 ownedCosmetics lists owned portraits on no card, never vanilla, as Portrait", () => {
    const portraitsOf = (owned: Parameters<typeof ownedCosmetics>[0]) =>
      ownedCosmetics(owned)
        .filter((entry) => entry.cosmetic.kind === "portrait")
        .map((entry) => [entry.cosmetic.id, entry.cardId]);
    // `vanilla` is a default every account owns with no row, so it is never listed, even when a row names it.
    expect(portraitsOf({ cards: [], account: [] })).toEqual([]);
    const owned = ownedCosmeticsOf({ account: ["portrait-shredder", "portrait-vanilla", "portrait-gary", "portrait-ulfric"] });
    // The registry's order, on no card; an id this client does not know is not shown.
    expect(portraitsOf(owned)).toEqual([
      ["portrait-gary", null],
      ["portrait-shredder", null],
    ]);

    expect(cosmeticKindWords("portrait")).toBe("Portrait");
    render(<OwnedCosmetics owned={owned} />);
    expect(screen.getByTestId(T.ownedCosmetic("portrait-gary", null))).toHaveTextContent("Gary the Gambler portrait · Portrait · Epic");
    expect(screen.getByTestId(T.ownedCosmetic("portrait-shredder", null))).toHaveTextContent(
      "Jlockeed Shredder-10 portrait · Portrait · Epic",
    );
    expect(screen.queryByTestId(T.ownedCosmetic("portrait-vanilla", null))).toBeNull();
    expect(screen.queryByTestId(T.ownedCosmetic("portrait-timmy", null))).toBeNull();
  });
});

describe("R1655000 a card in context", () => {
  const BIG_D = "core-001"; // a Common unit with a full art of its own
  const PLAIN = "core-002"; // a Common unit with none
  const GARY = "core-004";
  const MYTHIC = "core-096";
  const BIG_D_ART = "full-art-big-d-fender";
  const CARDS = [BIG_D, PLAIN, GARY, MYTHIC].map(defOf);
  const OWNS_BIG_D_ART = new Map([[BIG_D, new Set([BIG_D_ART])]]);

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

  it("R1655000 the strip holds each kind's registry entries and a full art only on a card that has one", () => {
    open(BIG_D);
    for (const kind of ["foil", "finish", "border"] as const) {
      const strip = screen.getByTestId(T.strip(kind));
      for (const cosmetic of cosmeticsOf(kind)) {
        expect(within(strip).getByTestId(T.stripItem(cosmetic.id))).toHaveTextContent(cosmetic.name);
      }
    }
    expect(within(screen.getByTestId(T.strip("fullArt"))).getByTestId(T.stripItem(BIG_D_ART))).toHaveTextContent("Epic");
    expect(item(NO_FULL_ART_ID)).toHaveTextContent("No full art");
    expect(screen.getAllByRole("button", { name: /^Use default/u })).toHaveLength(4);

    fireEvent.click(screen.getByTestId(T.card(PLAIN)));
    expect(screen.queryByTestId(T.strip("fullArt"))).toBeNull();
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

  it("R1655000 a shown foil keeps the rarity rules: a Mythic card wears none", () => {
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

  it("R1655000 a locked item shows its rarity and equips nothing", () => {
    open(BIG_D, new Map());

    const art = item(BIG_D_ART);
    expect(art).toHaveAttribute("data-owned", "false");
    expect(art).toHaveAttribute("aria-disabled", "true");
    expect(art).toHaveTextContent("Epic");
    expect(art).toHaveTextContent("Locked");
    expect(item("border-wood")).toHaveAttribute("data-owned", "true");
    expect(item(NO_FULL_ART_ID)).toHaveAttribute("data-owned", "true");

    fireEvent.click(art);
    fireEvent.mouseEnter(art);
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBeNull();
    expect(art).toHaveAttribute("data-equipped", "false");
    expect(shown()).toBeNull();

    cleanup();
    open(BIG_D, OWNS_BIG_D_ART);
    expect(item(BIG_D_ART)).toHaveAttribute("data-owned", "true");
    expect(item(BIG_D_ART)).not.toHaveTextContent("Locked");
    fireEvent.click(item(BIG_D_ART));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: true });
    expect(item(BIG_D_ART)).toHaveAttribute("data-equipped", "true");
  });

  it("R1655000 a full art is shown while it is shown, and stands once chosen", () => {
    writeCardSettings({ fullArt: false });
    open(BIG_D, OWNS_BIG_D_ART);
    expect(large().getAttribute("data-full-art")).toBeNull();

    fireEvent.mouseEnter(item(BIG_D_ART));
    expect(large().getAttribute("data-full-art")).toBe("true");
    fireEvent.mouseLeave(item(BIG_D_ART));
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBeNull();

    fireEvent.mouseEnter(item(BIG_D_ART));
    fireEvent.click(item(BIG_D_ART));
    fireEvent.mouseLeave(item(BIG_D_ART));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: true });
    expect(large().getAttribute("data-full-art")).toBe("true");

    // A face without one is a pick of its own, and it beats the setting being on.
    writeCardSettings({ fullArt: true });
    fireEvent.click(item(NO_FULL_ART_ID));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: false });
    expect(large().getAttribute("data-full-art")).toBeNull();
    fireEvent.click(screen.getByTestId(T.stripDefault("fullArt")));
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(large().getAttribute("data-full-art")).toBe("true");
  });

  it("R1655000 a full art shown, not chosen, is not marked as worn: the strip says what the card wears", () => {
    open(BIG_D, OWNS_BIG_D_ART);
    const useDefault = screen.getByTestId(T.stripDefault("fullArt"));
    expect(useDefault).toHaveAttribute("aria-pressed", "true");

    fireEvent.focus(item(BIG_D_ART));
    expect(large().getAttribute("data-full-art"), "the large card shows it").toBe("true");
    expect(item(BIG_D_ART)).toHaveAttribute("aria-pressed", "false");
    expect(item(BIG_D_ART)).toHaveAttribute("data-equipped", "false");
    expect(useDefault).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(item(BIG_D_ART));
    expect(item(BIG_D_ART)).toHaveAttribute("aria-pressed", "true");
    expect(useDefault).toHaveAttribute("aria-pressed", "false");
    fireEvent.blur(item(BIG_D_ART));
    expect(item(BIG_D_ART)).toHaveAttribute("aria-pressed", "true");
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: true });
  });

  it("R1834000 a shown full art writes no pick, so none is sent to the account, whether the view closes or the page is left", () => {
    writeCardSettings({ fullArt: false });
    writeCardPick(BIG_D, "border", "border-bone");
    const stored = localStorage.getItem(CARD_PICKS_SETTINGS_KEY);
    const writes: [string, CardPickChange][] = [];
    const unsubscribe = subscribeCardPickWrites((defId, change) => {
      writes.push([defId, change]);
    });
    open(BIG_D, OWNS_BIG_D_ART);

    fireEvent.focus(item(BIG_D_ART));
    expect(large().getAttribute("data-full-art"), "the large card shows it").toBe("true");
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });
    fireEvent.click(screen.getByTestId(T.cardViewClose));
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });

    fireEvent.click(screen.getByTestId(T.card(BIG_D)));
    fireEvent.focus(item(NO_FULL_ART_ID));
    fireEvent.focus(item(BIG_D_ART));
    act(() => {
      window.dispatchEvent(new Event("pagehide"));
    });
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBe(stored);
    expect(writes, "no preview reaches the account's sync").toEqual([]);

    fireEvent.click(item(BIG_D_ART));
    expect(writes).toEqual([[BIG_D, { fullArt: true }]]);
    unsubscribe();
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

  it("R1655000 cardStrip offers each kind's registry entries, the full art of a card that has one, and says which are locked", () => {
    const strip = cardStrip(BIG_D, (_defId, cosmeticId) => cosmeticId !== BIG_D_ART && cosmeticId !== "foil-cracked-ice");

    expect(strip.map((group) => group.kind)).toEqual(["foil", "finish", "border", "fullArt"]);
    expect(strip[0]?.items.map((entry) => entry.id)).toEqual(cosmeticsOf("foil").map((cosmetic) => cosmetic.id));
    expect(strip.flatMap((group) => group.items).filter((entry) => !entry.owned).map((entry) => entry.id)).toEqual([
      "foil-cracked-ice",
      BIG_D_ART,
    ]);
    expect(strip[0]?.items[0]).toMatchObject({ id: "foil-none", name: "No foil", rarity: "Common", owned: true });
    expect(strip[3]?.items.map((entry) => [entry.id, entry.rarity])).toEqual([
      [NO_FULL_ART_ID, "Common"],
      [BIG_D_ART, "Epic"],
    ]);
    expect(cardStrip(PLAIN, () => true).map((group) => group.kind)).toEqual(["foil", "finish", "border"]);
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

  it("R1956000 an owned grid tile wears its card's picks, never the account's; a locked tile, a Mythic tile's foil and a Radiant face do not", () => {
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
