// `/collection` (#1307, SPEC §14): the screen reads each panel's answers with the caller's token,
// shows each panel's own loading message and error, and never goes blank when a read is refused.
// R1579000: it is the hub of four doors, its Cards and Cosmetics sections open in place by
// `?section=`, and Cosmetics is where a cosmetic is equipped.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { newestShippedSet } from "@jackioh/shared";

import {
  COSMETICS_SETTINGS_KEY,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
  writeCardPick,
} from "../cards/cosmetics.ts";
import { collectionTestid as T } from "../collection/collection.ts";
import { fullCollection, questsHeld, tracksOf } from "../collection/fixtures.ts";
import {
  getCollection,
  getCosmeticPacks,
  getOwnedCosmetics,
  getPacks,
  getQuests,
  getTracks,
  getWallet,
  type CollectionResponse,
} from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { SETTINGS_GROUPS } from "../settings/groups.ts";
import { cosmeticPacksInfo, ownedCosmeticsOf, walletInfo } from "../shop/fixtures.ts";
import { navTestid } from "./nav.tsx";
import CollectionRoute from "./collection.tsx";
import { questsTestid } from "./QuestsPanel.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getCollection: vi.fn(),
    getWallet: vi.fn(),
    getPacks: vi.fn(),
    getCosmeticPacks: vi.fn(),
    getOwnedCosmetics: vi.fn(),
    getTracks: vi.fn(),
    getQuests: vi.fn(),
  };
});

const TOKEN = "token-1";

/** Every read answers its fixture. */
function answerAll(): void {
  vi.mocked(getCollection).mockResolvedValue(fullCollection());
  vi.mocked(getWallet).mockResolvedValue(walletInfo());
  vi.mocked(getPacks).mockResolvedValue(packsInfo());
  vi.mocked(getCosmeticPacks).mockResolvedValue(cosmeticPacksInfo());
  vi.mocked(getTracks).mockResolvedValue(tracksOf());
  vi.mocked(getQuests).mockResolvedValue(questsHeld());
  vi.mocked(getOwnedCosmetics).mockResolvedValue(ownedCosmeticsOf());
}

beforeEach(() => {
  window.history.replaceState(null, "", paths.collection);
});

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("#1307 the collection route", () => {
  beforeEach(() => {
    window.history.replaceState(null, "", paths.collectionSection("cards"));
  });

  it("reads every panel with the caller's token", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(await screen.findByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      expect(read).toHaveBeenCalledWith(TOKEN);
    }
    expect(screen.getByTestId(T.screen)).toBeInTheDocument();
  });

  it("shows each panel's loading message until it answers", async () => {
    answerAll();
    let answerCards: (collection: CollectionResponse) => void = () => undefined;
    vi.mocked(getCollection).mockReturnValue(
      new Promise<CollectionResponse>((resolve) => {
        answerCards = resolve;
      }),
    );
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.loading("cards"))).toHaveTextContent("Loading your cards…");
    expect(await screen.findByTestId(T.jlore)).toBeInTheDocument();
    expect(screen.queryByTestId(T.loading("wallet"))).toBeNull();
    expect(screen.queryByTestId(T.loading("tracks"))).toBeNull();
    expect(screen.getByTestId(T.loading("cards"))).toBeInTheDocument();

    await act(async () => {
      answerCards(fullCollection());
      await Promise.resolve();
    });
    expect(screen.queryByTestId(T.loading("cards"))).toBeNull();
    expect(screen.getByTestId(T.completion("Core"))).toBeInTheDocument();
  });

  it("shows the wallet and tracks loading messages before they answer", () => {
    vi.mocked(getWallet).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getPacks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getCosmeticPacks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getTracks).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getCollection).mockReturnValue(new Promise(() => undefined));
    vi.mocked(getQuests).mockReturnValue(new Promise(() => undefined));
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.loading("wallet"))).toHaveTextContent("Loading your wallet…");
    expect(screen.getByTestId(T.loading("tracks"))).toHaveTextContent("Loading your tracks…");
  });

  it("a refused read says why in its panel; the others still show", async () => {
    answerAll();
    vi.mocked(getPacks).mockRejectedValue(new Error("the packs are unavailable"));
    render(<CollectionRoute token={TOKEN} />);

    expect(await screen.findByTestId(T.error("wallet"))).toHaveTextContent("the packs are unavailable");
    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(await screen.findByTestId(T.track(newestShippedSet()))).toBeInTheDocument();
    expect(screen.queryByTestId(T.error("cards"))).toBeNull();
    expect(screen.queryByTestId(T.error("tracks"))).toBeNull();
    expect(screen.queryByTestId(T.jlore)).toBeNull();
  });

  it("every read refused shows messages, not a blank screen", async () => {
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      vi.mocked(read).mockRejectedValue(new Error("the server refused"));
    }
    render(<CollectionRoute token={TOKEN} />);

    for (const part of ["cards", "wallet", "tracks"] as const) {
      expect(await screen.findByTestId(T.error(part))).toHaveTextContent("the server refused");
    }
    expect(await screen.findByTestId(questsTestid.error)).toHaveTextContent("the server refused");
    expect(screen.getByRole("heading", { name: "Cards", level: 2 })).toBeInTheDocument();
  });

  it("names the screen Collection", () => {
    expect(screenNameFor(paths.collection)).toBe("Collection");
    expect(documentTitleFor(paths.collection)).toMatch(/^Collection · /u);
  });
});

describe("R1579000 the Collection hub", () => {
  beforeEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  afterEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  const READS = [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests, getOwnedCosmetics];

  it("R1579000 offers four doors to the decks, the cards, the cosmetics and the card builder, and reads nothing", () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(screen.getByRole("heading", { name: "Your collection" })).toBeInTheDocument();
    expect(screen.getByTestId(T.door("decks"))).toHaveAttribute("href", paths.decks);
    expect(screen.getByTestId(T.door("cards"))).toHaveAttribute("href", paths.collectionSection("cards"));
    expect(screen.getByTestId(T.door("cosmetics"))).toHaveAttribute("href", paths.collectionSection("cosmetics"));
    expect(screen.getByTestId(T.door("builder"))).toHaveAttribute("href", paths.creator);
    for (const read of READS) expect(read).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(T.door("builder")));
    expect(window.location.pathname).toBe(paths.creator);

    window.history.replaceState(null, "", paths.collection);
    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("R1579000 the Cards door opens the cards, wallet, tracks and quests in place, and Back returns to the hub", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(screen.getByTestId(T.door("cards")));
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cards");
    expect(await screen.findByTestId(T.completion("Core"))).toBeInTheDocument();
    expect(await screen.findByTestId(T.jlore)).toBeInTheDocument();
    expect(await screen.findByTestId(questsTestid.panel)).toHaveTextContent("Win 3 games");
    expect(screen.getByTestId(T.body)).toBeInTheDocument();
    expect(screen.queryByTestId(T.door("cards"))).toBeNull();

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(screen.getByTestId(T.door("cards"))).toBeInTheDocument();
    expect(window.location.search).toBe("");
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R1579000 the Cosmetics door lists what the account owns and equips through the same store, with no request", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(screen.getByTestId(T.door("cosmetics")));
    expect(await screen.findByTestId(T.ownedCosmetic("full-art-gary-the-gambler", "core-004"))).toBeInTheDocument();
    expect(screen.getAllByTestId(T.ownedCosmetic("table-felt", null))).toHaveLength(1);
    expect(screen.queryByTestId(T.ownedCosmetic("foil-none", null))).toBeNull();

    const group = SETTINGS_GROUPS.find((candidate) => candidate.id === "cosmetics");
    if (group === undefined) throw new Error("no cosmetics settings group");
    const changed = vi.fn();
    const unsubscribe = group.subscribe(changed);
    // R1655000: the playmat, backdrop and card back are tiles, each a click on an owned one; the defaults of every card stay selects.
    expect(screen.queryByTestId(T.equip("table"))).toBeNull();
    fireEvent.click(within(screen.getByTestId(T.tile("table-felt"))).getByRole("button"));
    fireEvent.click(within(screen.getByTestId(T.tile("card-back-starfield"))).getByRole("button"));
    fireEvent.change(screen.getByTestId(T.equip("foil")), { target: { value: "foil-rainbow" } });
    unsubscribe();

    expect(screen.getByTestId(T.tile("table-felt"))).toHaveAttribute("data-equipped", "true");
    expect(readCosmetics()).toMatchObject({ table: "table-felt", cardBack: "card-back-starfield", foil: "foil-rainbow" });
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "null")).toMatchObject({
      table: "table-felt",
      cardBack: "card-back-starfield",
      foil: "foil-rainbow",
    });
    expect(group.read()["table"]).toBe("table-felt");
    expect(group.read()["foil"]).toBe("foil-rainbow");
    expect(changed).toHaveBeenCalled();
    expect(getOwnedCosmetics).toHaveBeenCalledTimes(1);
    expect(getOwnedCosmetics).toHaveBeenCalledWith(TOKEN);
    for (const read of [getCollection, getWallet, getPacks, getCosmeticPacks, getTracks, getQuests]) {
      expect(read).not.toHaveBeenCalled();
    }
  });

  it("R1579000 the browser's Back and Forward move between the hub and a section", async () => {
    answerAll();
    render(<CollectionRoute token={TOKEN} />);
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");

    act(() => {
      window.history.pushState(null, "", paths.collectionSection("cosmetics"));
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "cosmetics");
    expect(await screen.findByTestId(T.equip("foil"))).toBeInTheDocument();

    act(() => {
      window.history.replaceState(null, "", paths.collection);
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
  });

  it("R1579000 an unknown section opens the hub", () => {
    answerAll();
    window.history.replaceState(null, "", `${paths.collection}?section=shop`);
    render(<CollectionRoute token={TOKEN} />);

    expect(screen.getByTestId(T.screen)).toHaveAttribute("data-section", "hub");
    expect(screen.getByTestId(T.door("decks"))).toBeInTheDocument();
    for (const read of READS) expect(read).not.toHaveBeenCalled();
  });
});

describe("R1655000 the Collection's card view", () => {
  beforeEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
    window.history.replaceState(null, "", paths.collectionSection("cards"));
  });

  afterEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  it("R1655000 a card of the Cards section opens large wearing its own picks, and equipping on it asks the server nothing", async () => {
    answerAll();
    writeCardPick("core-001", "border", "border-wood");
    render(<CollectionRoute token={TOKEN} />);

    fireEvent.click(await screen.findByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-001")));
    const face = screen.getByTestId(T.cardViewFace).querySelector(".cf");
    expect(face?.getAttribute("data-frame")).toBe("border-wood");

    fireEvent.click(screen.getByTestId(T.stripItem("foil-rainbow")));
    expect(readCardPicks("core-001")).toEqual({ border: "border-wood", foil: "foil-rainbow" });
    expect(face?.getAttribute("data-cosmetic-foil")).toBe("foil-rainbow");
    expect(readCosmetics().foil).toBe("foil-none");
    // Who owns what is the session's (`useCanEquip`, from the screen's `ViewerFullArt`): the section reads nothing more for it.
    expect(getOwnedCosmetics).not.toHaveBeenCalled();
  });
});
