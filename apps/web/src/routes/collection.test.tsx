// `/collection` (#1307, SPEC §14): the screen reads each panel's answers with the caller's token,
// shows each panel's own loading message and error, and never goes blank when a read is refused.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { newestShippedSet } from "@jackioh/shared";

import { collectionTestid as T } from "../collection/collection.ts";
import { fullCollection, questsHeld, tracksOf } from "../collection/fixtures.ts";
import {
  getCollection,
  getCosmeticPacks,
  getPacks,
  getQuests,
  getTracks,
  getWallet,
  type CollectionResponse,
} from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { cosmeticPacksInfo, walletInfo } from "../shop/fixtures.ts";
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
}

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

describe("#1307 the collection route", () => {
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
    expect(screen.getByRole("heading", { name: "Your collection" })).toBeInTheDocument();
  });

  it("names the screen Collection", () => {
    expect(screenNameFor(paths.collection)).toBe("Collection");
    expect(documentTitleFor(paths.collection)).toMatch(/^Collection · /u);
  });
});
