// The collection screen's panels (#1307, SPEC §14), from fixture responses: the completion counts
// and locked cards of the owned-cards panel, the wallet, and the tracks' levels, XP bars, rewards
// and locks. The server's answers are the fixtures; nothing here asks it anything.
// R1655000: a card in context (the card view's strip, its previews, its equips and its locks) and the
// account's cosmetics as tiles.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLOBAL_PACK_NAME, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
import { SHIPPED_SETS, newestShippedSet, type CardDef } from "@jackioh/shared";

import {
  CARD_PICKS_SETTINGS_KEY,
  COSMETICS_SETTINGS_KEY,
  cosmeticsOf,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
  writeCardPick,
  writeCosmetics,
} from "../cards/cosmetics.ts";
import { FullArtOwned } from "../cards/fullArt.ts";
import {
  CARD_SETTINGS_DEFAULTS,
  INSPECT_CLOSE,
  INSPECT_DETAIL,
  closeInspect,
  writeCardSettings,
} from "../cards/index.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { packName } from "../packs/packs.ts";
import { cosmeticPacksInfo, ownedCosmeticsOf, walletInfo } from "../shop/fixtures.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import AccountCosmeticTiles from "./AccountCosmeticTiles.tsx";
import OwnedCards from "./OwnedCards.tsx";
import TracksPanel from "./TracksPanel.tsx";
import WalletPanel from "./WalletPanel.tsx";
import {
  COLLECTED_DEFS,
  NO_FULL_ART_ID,
  accountTiles,
  cardStrip,
  collectionSectionOf,
  collectionTestid as T,
  ownedCosmetics,
  setCompletions,
} from "./collection.ts";
import { collectionOwning, fullCollection, track, tracksOf } from "./fixtures.ts";

// R1655000: the account's cosmetics, a table past the registry's that no account owns yet, so a locked
// tile has something to show. The registry's own account cosmetics are every one a starter (§14.2).
vi.mock("../cards/cosmetics.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../cards/cosmetics.ts")>();
  const future = {
    id: "table-future",
    kind: "table",
    name: "Future table",
    rarity: "Mythic",
    default: false,
    starter: false,
    chase: true,
  } as const;
  return {
    ...actual,
    cosmeticsOf: (kind: Parameters<typeof actual.cosmeticsOf>[0]) =>
      kind === "table" ? [...actual.cosmeticsOf(kind), future] : actual.cosmeticsOf(kind),
  };
});

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

describe("R1655000 a card in context", () => {
  const BIG_D = "core-001"; // a Common unit with a full art of its own
  const PLAIN = "core-002"; // a Common unit with none
  const GARY = "core-004";
  const MYTHIC = "core-096";
  const BIG_D_ART = "full-art-big-d-fender";
  const CARDS = [BIG_D, PLAIN, GARY, MYTHIC].map(defOf);
  /** The cosmetics the ledger lists for Big D-fender's owner: its full art. */
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

  it("R1655000 a card opened in the Collection wears its own picks while another card keeps the account's", () => {
    writeCosmetics({ finish: "finish-linen" });
    writeCardPick(BIG_D, "border", "border-wood");
    writeCardPick(BIG_D, "foil", "foil-rainbow");
    open(BIG_D);

    expect(large().getAttribute("data-frame")).toBe("border-wood");
    expect(large().getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    expect(large().getAttribute("data-finish")).toBe("finish-linen");
    expect(item("border-wood")).toHaveAttribute("data-equipped", "true");
    expect(item("border-standard")).toHaveAttribute("data-equipped", "false");

    // Another card, in the same view: the account's picks, and none of Big D-fender's.
    fireEvent.click(screen.getByTestId(T.card(PLAIN)));
    expect(screen.getByTestId(T.cardView)).toHaveAttribute("data-card", PLAIN);
    expect(large().getAttribute("data-finish")).toBe("finish-linen");
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
    // The card's other picks stay on the shown copy, and the card itself is not changed.
    expect(shown()?.getAttribute("data-finish")).toBe("finish-linen");
    expect(large().getAttribute("data-frame")).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ finish: "finish-linen" });

    fireEvent.mouseLeave(item("border-bone"));
    expect(shown()).toBeNull();

    fireEvent.focus(item("foil-holo-sweep"));
    expect(shown()?.getAttribute("data-cosmetic-foil")).toBe("foil-holo-sweep");
    // A preview's foil holds still: the cap on moving foils belongs to a game's faces.
    expect(shown()?.getAttribute("data-cosmetic-motion")).toBe("static");
    fireEvent.mouseEnter(item("finish-gilded-edge"));
    fireEvent.blur(item("foil-holo-sweep"));
    expect(shown()?.getAttribute("data-finish")).toBe("finish-gilded-edge");
    expect(shown()?.getAttribute("data-cosmetic-foil")).toBeNull();

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
    writeCosmetics({ border: "border-bone" });
    open(BIG_D);
    expect(large().getAttribute("data-frame")).toBe("border-bone");

    fireEvent.click(item("border-wood"));
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-wood" });
    expect(large().getAttribute("data-frame")).toBe("border-wood");
    expect(item("border-wood")).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.stripDefault("border"))).toHaveAttribute("aria-pressed", "false");
    expect(readCosmetics().border).toBe("border-bone");

    // A pick of the kind's default is a pick: it beats the account's pick that is not the default.
    fireEvent.click(item("border-standard"));
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-standard" });
    expect(large().getAttribute("data-frame")).toBeNull();

    fireEvent.click(screen.getByTestId(T.stripDefault("border")));
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBe("{}");
    expect(large().getAttribute("data-frame")).toBe("border-bone");
    expect(screen.getByTestId(T.stripDefault("border"))).toHaveAttribute("aria-pressed", "true");

    // Another card has none of it.
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
    // What every account owns is not locked.
    expect(item("border-wood")).toHaveAttribute("data-owned", "true");
    expect(item(NO_FULL_ART_ID)).toHaveAttribute("data-owned", "true");

    fireEvent.click(art);
    fireEvent.mouseEnter(art);
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBeNull();
    expect(art).toHaveAttribute("data-equipped", "false");
    expect(shown()).toBeNull();

    // The ledger lists it, and the same item equips.
    cleanup();
    open(BIG_D, OWNS_BIG_D_ART);
    expect(item(BIG_D_ART)).toHaveAttribute("data-owned", "true");
    expect(item(BIG_D_ART)).not.toHaveTextContent("Locked");
    fireEvent.click(item(BIG_D_ART));
    expect(readCardPicks(BIG_D)).toEqual({ fullArt: true });
    expect(item(BIG_D_ART)).toHaveAttribute("data-equipped", "true");
  });

  it("R1655000 a full art is shown by holding its pick, which is given back when it stops being shown", () => {
    writeCardSettings({ fullArt: false });
    open(BIG_D, OWNS_BIG_D_ART);
    expect(large().getAttribute("data-full-art")).toBeNull();

    fireEvent.mouseEnter(item(BIG_D_ART));
    expect(large().getAttribute("data-full-art")).toBe("true");
    fireEvent.mouseLeave(item(BIG_D_ART));
    expect(large().getAttribute("data-full-art")).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBe("{}");

    // Clicked, the pick stands past the pointer leaving.
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

  it("R1655000 a shown full art is given back when the view closes or the page is left", () => {
    writeCardSettings({ fullArt: false });
    writeCardPick(BIG_D, "border", "border-bone");
    open(BIG_D, OWNS_BIG_D_ART);

    fireEvent.focus(item(BIG_D_ART));
    expect(readCardPicks(BIG_D).fullArt).toBe(true);
    fireEvent.click(screen.getByTestId(T.cardViewClose));
    expect(screen.queryByTestId(T.cardView)).toBeNull();
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });

    fireEvent.click(screen.getByTestId(T.card(BIG_D)));
    fireEvent.focus(item(BIG_D_ART));
    expect(readCardPicks(BIG_D).fullArt).toBe(true);
    act(() => {
      window.dispatchEvent(new Event("pagehide"));
    });
    expect(readCardPicks(BIG_D)).toEqual({ border: "border-bone" });
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
    // A card's own full art is its own: Big D-fender's is not on another card.
    expect(cardStrip(PLAIN, () => true).map((group) => group.kind)).toEqual(["foil", "finish", "border"]);
  });
});

describe("R1655000 the account's cosmetics as tiles", () => {
  it("R1655000 accountTiles lists a kind's entries, a default and a starter owned and the rest as the account's list says", () => {
    const none = accountTiles("table", null);
    expect(none.map((tile) => [tile.cosmetic.id, tile.owned])).toEqual([
      ["table-tavern", true],
      ["table-wood", true],
      ["table-felt", true],
      ["table-neon-grid", true],
      ["table-future", false],
    ]);
    expect(accountTiles("table", { cards: [], account: [] }).at(-1)?.owned).toBe(false);
    expect(accountTiles("table", { cards: [], account: ["table-future"] }).at(-1)?.owned).toBe(true);
    expect(accountTiles("cardBack", null).map((tile) => tile.cosmetic.id)).toEqual(cosmeticsOf("cardBack").map((cosmetic) => cosmetic.id));
  });

  it("R1655000 a playmat and a backdrop are drawn together as a mini board, a card back as a back", () => {
    render(<AccountCosmeticTiles owned={null} />);

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

  it("R1655000 clicking an owned tile equips it through the cosmetics store; a locked one shows its rarity and equips nothing", () => {
    const { rerender } = render(<AccountCosmeticTiles owned={null} />);

    fireEvent.click(within(screen.getByTestId(T.tile("card-back-lacquer"))).getByRole("button"));
    expect(readCosmetics().cardBack).toBe("card-back-lacquer");
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "null")).toMatchObject({ cardBack: "card-back-lacquer" });
    expect(screen.getByTestId(T.tile("card-back-lacquer"))).toHaveAttribute("data-equipped", "true");
    expect(screen.getByTestId(T.tile("card-back-standard"))).toHaveAttribute("data-equipped", "false");

    const locked = screen.getByTestId(T.tile("table-future"));
    expect(locked).toHaveAttribute("data-owned", "false");
    expect(locked).toHaveTextContent("Mythic");
    expect(locked).toHaveTextContent("Locked");
    expect(within(locked).getByRole("button")).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(within(locked).getByRole("button"));
    expect(readCosmetics().table).toBe("table-tavern");

    rerender(<AccountCosmeticTiles owned={{ cards: [], account: ["table-future"] }} />);
    expect(screen.getByTestId(T.tile("table-future"))).toHaveAttribute("data-owned", "true");
    fireEvent.click(within(screen.getByTestId(T.tile("table-future"))).getByRole("button"));
    expect(readCosmetics().table).toBe("table-future");
  });
});
