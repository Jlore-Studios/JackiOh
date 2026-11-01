// Deck workshop: decks and trios, comparison (R251, R252, R253), codes (R255, R339–R341), autosave
// (R256) and a deck's history (R2100000) (SPEC §9.4, R250–R256).
//
// Validator sentences are computed, never copied (messages.test.ts).

import type { CardDef } from "@jackioh/shared";
import { checkImportRoom, validateDeck, validateTrio, type Collection } from "@jackioh/validator";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EARLIER_DECK_SIZE } from "@jackioh/engine/config";
import { DECK_AUTOSAVE_DEBOUNCE_MS, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "@jackioh/server-config";
import { cosmeticsOf } from "../../cards/cosmetics.ts";
import { PORTRAIT_DEFS } from "../../emotes/portraits.ts";
import { formatDateTime } from "../../i18n/i18n.ts";
import { ApiRequestError, type SavedDeck, type SavedTrio } from "../../net/api.ts";
import { navigate, newDeckPath } from "../../net/navigate.ts";
import { expectEmptyState } from "../../test/empty.ts";
import { pressBack } from "../../test/history.ts";
import { at } from "../../test/stubServer.ts";
import { UNDO_WINDOW_MS } from "../../undo/hold.ts";
import { UNDO_TOAST, UNDO_TOAST_UNDO } from "../../undo/UndoToast.tsx";
import { DECK_COMPLETE_SAVED } from "./DeckEditor.tsx";
import { decodeDeckCode, encodeDeckCode } from "./deckCode.ts";
import { DECK_SIZE } from "./deckSize.ts";
import DeckWorkshop, { syncWords, type WorkshopOpen } from "./DeckWorkshop.tsx";
import { TOKEN_ID, fixtureCardId, fixtureCardName, fixtureCatalog, fixtureCollection, legalDecks } from "./fixtures.ts";
import { droppedLines } from "./ImportPanel.tsx";
import { OFFLINE_MESSAGE, UNTITLED_DECK, mirrorKey } from "./sync.ts";
import { TRIO_CODE_MESSAGES, decodeTrioCode, encodeTrioCode } from "./trioCode.ts";
import {
  TEST_PROFILE,
  decksResponse,
  fakeDeckServer,
  manualClock,
  memoryStorage,
  savedDeck,
  savedTrio,
  settle,
} from "./testkit.ts";
import {
  DECK_CAP,
  DECK_CAP_REASON,
  DECK_CODE_OUTPUT,
  DECK_COMPARE_SELECT,
  DECK_CONFLICTS,
  DB_SEARCH,
  DECK_COPY_CODE,
  DECK_COSMETICS_BUTTON,
  DECK_COUNT,
  DECK_DUPLICATE,
  DECK_DUPLICATE_REASON,
  DECK_DELETE,
  DECK_EDITOR,
  DECK_HISTORY_BUTTON,
  DECK_HISTORY_PANEL,
  DECK_HISTORY_ROW,
  DECK_IMPORT,
  DECK_IMPORT_CAP_REASON,
  DECK_IMPORT_INPUT,
  DECK_IMPORT_OPEN,
  DECK_IMPORT_PREVIEW,
  DECK_IMPORT_SUBMIT,
  DECK_LIST,
  DECK_NAME_INPUT,
  DECK_NEW,
  DECK_REDO,
  DECK_SAVE_ERROR,
  DECK_STATUS,
  DECK_UNDO,
  DECK_TO_PRACTICE,
  DECK_VERDICT,
  SYNC_STATUS,
  TRIO_CAP_REASON,
  TRIO_CODE_OUTPUT,
  TRIO_COMPARE,
  TRIO_COPY_CODE,
  TRIO_DELETE,
  TRIO_EDITOR,
  TRIO_IMPORT,
  TRIO_IMPORT_CAP_REASON,
  TRIO_IMPORT_ERROR,
  TRIO_IMPORT_INPUT,
  TRIO_IMPORT_OPEN,
  TRIO_IMPORT_PREVIEW,
  TRIO_IMPORT_SHARED,
  TRIO_IMPORT_SUBMIT,
  TRIO_LIST,
  TRIO_NAME_INPUT,
  TRIO_NEW,
  TRIO_VERDICT,
  WORKSHOP,
  WORKSHOP_BACK,
  WORKSHOP_EMPTY,
  addPoolId,
  deckCardId,
  deckCompareChipId,
  deckDuplicateId,
  deckHistoryRestoreId,
  deckRowId,
  loadoutErrorId,
  poolCardId,
  trioCardId,
  trioImportSlotId,
  trioOpenDeckId,
  trioRowId,
  trioSlotId,
} from "./testids.ts";

const catalog = fixtureCatalog();
const collection = fixtureCollection();
const [ONE = [], TWO = [], THREE = []] = legalDecks();

type Mounted = {
  server: ReturnType<typeof fakeDeckServer>;
  storage: ReturnType<typeof memoryStorage>;
  tick: (ms: number) => Promise<void>;
};

type MountOptions = {
  decks?: SavedDeck[];
  trios?: SavedTrio[];
  collection?: Collection | null;
  initialOpen?: WorkshopOpen;
  server?: ReturnType<typeof fakeDeckServer>;
  storage?: ReturnType<typeof memoryStorage>;
  catalogOverride?: typeof catalog;
};

function mount(options: MountOptions = {}): Mounted {
  const server = options.server ?? fakeDeckServer();
  const data = decksResponse(options.decks ?? [], options.trios ?? [], catalog.version);
  server.seed(data);
  const storage = options.storage ?? memoryStorage();
  const clock = manualClock();
  let minted = 0;
  render(
    <DeckWorkshop
      catalog={options.catalogOverride ?? catalog}
      collection={options.collection === undefined ? collection : options.collection}
      data={data}
      profileId={TEST_PROFILE}
      api={server.api}
      storage={storage}
      clock={clock}
      newId={() => {
        minted += 1;
        return `new-${String(minted)}`;
      }}
      {...(options.initialOpen === undefined ? {} : { initialOpen: options.initialOpen })}
    />,
  );
  return {
    server,
    storage,
    tick: async (ms) => {
      await act(async () => {
        await clock.advance(ms);
      });
    },
  };
}

function syncState(): string | null {
  return screen.getByTestId(SYNC_STATUS).getAttribute("data-state");
}

function nameOf(cardId: string): string {
  return catalog.cards[cardId]?.name ?? cardId;
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("the rail", () => {
  it("R250 lists every saved deck with its count and status chip, and the caps", () => {
    mount({
      decks: [savedDeck("full", "Aggro", ONE, 1), savedDeck("part", "Tempo", TWO.slice(0, 7), 2)],
      trios: [savedTrio("t", "Ladder", ["full", null, null], 3)],
    });
    expect(screen.getByTestId(DECK_CAP)).toHaveTextContent(`2/${String(MAX_SAVED_DECKS)}`);
    const full = screen.getByTestId(deckRowId("full"));
    expect(full).toHaveAttribute("data-count", String(DECK_SIZE));
    expect(full).toHaveAttribute("data-status", "ready");
    expect(full).toHaveTextContent("Ready");
    const part = screen.getByTestId(deckRowId("part"));
    expect(part).toHaveAttribute("data-count", "7");
    expect(part).toHaveAttribute("data-status", "incomplete");
    expect(part).toHaveTextContent(`7/${String(DECK_SIZE)}`);
    expect(screen.getByTestId(trioRowId("t"))).toHaveAttribute("data-ready", "false");
    expect(syncState()).toBe("saved");
  });

  it("R1705001 shows a saved EARLIER_DECK_SIZE-card deck as incomplete", () => {
    // A deck saved complete before #1705 is a draft now: it keeps its cards and counts toward DECK_SIZE.
    const earlier = ONE.slice(0, EARLIER_DECK_SIZE);
    expect(earlier).toHaveLength(EARLIER_DECK_SIZE);
    mount({ decks: [savedDeck("earlier", "Old Aggro", earlier, 1)] });
    const row = screen.getByTestId(deckRowId("earlier"));
    expect(row).toHaveAttribute("data-count", String(EARLIER_DECK_SIZE));
    expect(row).toHaveAttribute("data-status", "incomplete");
    expect(row).toHaveTextContent(`${String(EARLIER_DECK_SIZE)}/${String(DECK_SIZE)}`);
    expect(row).not.toHaveTextContent("Ready");
    expect(screen.getByRole("button", { name: new RegExp(`^Old Aggro, ${String(EARLIER_DECK_SIZE)} of ${String(DECK_SIZE)} cards`) })).toBe(row);
  });

  it("each row is named in words, with no aria-label to hide its drawn count from voice control", () => {
    mount({ decks: [savedDeck("full", "Aggro", ONE, 1)], trios: [savedTrio("t", "Ladder", ["full", null, null], 3)] });
    const deck = screen.getByRole("button", { name: `Aggro, ${String(DECK_SIZE)} of ${String(DECK_SIZE)} cards, Ready` });
    expect(deck).toBe(screen.getByTestId(deckRowId("full")));
    expect(deck).not.toHaveAttribute("aria-label");
    const trio = screen.getByRole("button", { name: "Ladder, 1 of 3 decks, not ready" });
    expect(trio).toBe(screen.getByTestId(trioRowId("t")));
    expect(trio).not.toHaveAttribute("aria-label");
  });

  it("R250 a deck holding cards the player does not own says how many", () => {
    const missing: Record<string, number> = { ...collection };
    delete missing[ONE[0] ?? ""];
    mount({ decks: [savedDeck("full", "Aggro", ONE, 1)], collection: missing });
    const row = screen.getByTestId(deckRowId("full"));
    expect(row).toHaveAttribute("data-status", "unowned");
    expect(row).toHaveTextContent("1 not owned");
  });

  it("R250 New deck is off at the cap and says why; New trio likewise", () => {
    const decks = Array.from({ length: MAX_SAVED_DECKS }, (_unused, index) => savedDeck(`d${String(index)}`, `Deck ${String(index)}`, [], index));
    const trios = Array.from({ length: MAX_SAVED_TRIOS }, (_unused, index) => savedTrio(`t${String(index)}`, `Trio ${String(index)}`, [null, null, null], index));
    mount({ decks, trios });
    expect(screen.getByTestId(DECK_NEW)).toBeDisabled();
    expect(screen.getByTestId(DECK_CAP_REASON)).toHaveTextContent(String(MAX_SAVED_DECKS));
    expect(screen.getByTestId(DECK_NEW)).toHaveAttribute("aria-describedby", screen.getByTestId(DECK_CAP_REASON).id);
    expect(screen.getByTestId(TRIO_NEW)).toBeDisabled();
    expect(screen.getByTestId(TRIO_CAP_REASON)).toHaveTextContent(String(MAX_SAVED_TRIOS));
  });

  it("a phone shows the list first, an opened deck next, and the way back", () => {
    mount({ decks: [savedDeck("a", "Aggro", ONE.slice(0, 3), 1)] });
    const root = screen.getByTestId(WORKSHOP);
    expect(root).toHaveAttribute("data-view", "list");
    fireEvent.click(screen.getByTestId(deckRowId("a")));
    expect(root).toHaveAttribute("data-view", "editor");
    expect(screen.getByTestId(deckRowId("a"))).toHaveAttribute("aria-current", "true");
    fireEvent.click(screen.getByTestId(WORKSHOP_BACK));
    expect(root).toHaveAttribute("data-view", "list");
  });
});

describe("R2108000 the workshop's empty state", () => {
  it("R2108000 with no deck the workshop's empty state offers Make a deck, which makes and opens one", () => {
    mount();
    const root = expectEmptyState(
      within(screen.getByTestId(WORKSHOP_EMPTY)).getByText("Make your first deck, or import a code someone shared."),
    );
    fireEvent.click(within(root).getByText("Make a deck"));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
  });
});

describe("a deck", () => {
  it("R256 New deck makes a named deck, opens it, and saves it after the debounce", async () => {
    const { server, tick } = mount();
    fireEvent.click(screen.getByTestId(DECK_NEW));
    const editor = screen.getByTestId(DECK_EDITOR);
    expect(editor).toHaveAttribute("data-deck", "new-1");
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Deck 1");
    expect(screen.getByTestId(deckRowId("new-1"))).toHaveAttribute("data-unsynced", "true");
    expect(syncState()).toBe("saving");
    expect(screen.getByTestId(SYNC_STATUS)).toHaveTextContent(syncWords({ state: "saving", message: null }));

    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("new-1")).toMatchObject({ name: "Deck 1", cards: [] });
    expect(syncState()).toBe("saved");
    expect(screen.getByTestId(deckRowId("new-1"))).not.toHaveAttribute("data-unsynced");
  });

  it("R256 renaming saves the new name; an emptied name saves as Untitled deck and settles there on blur", async () => {
    const { server, tick } = mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    const input = screen.getByTestId(DECK_NAME_INPUT);
    fireEvent.change(input, { target: { value: "Big Tempo" } });
    expect(screen.getByTestId(deckRowId("a"))).toHaveTextContent("Big Tempo");
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.name).toBe("Big Tempo");

    fireEvent.change(input, { target: { value: "   " } });
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.name).toBe(UNTITLED_DECK);
    fireEvent.blur(input);
    expect(input).toHaveValue(UNTITLED_DECK);
  });

  it("R250 cards added and taken out move the deck's count and its row", async () => {
    const { server, tick } = mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(1))));
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(2))));
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", "2");
    expect(screen.getByTestId(deckRowId("a"))).toHaveAttribute("data-count", "2");
    fireEvent.click(screen.getByTestId(deckCardId(fixtureCardId(1))));
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", "1");
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.cards).toEqual([fixtureCardId(2)]);
  });

  it("R250 a full deck takes no more, and says so", () => {
    mount({ decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    const spare = TWO[0] ?? "";
    fireEvent.click(screen.getByTestId(addPoolId(spare)));
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", String(DECK_SIZE));
    expect(screen.queryByTestId(deckCardId(spare))).toBeNull();
    expect(screen.getByTestId(DECK_STATUS)).toHaveTextContent(`Aggro has ${String(DECK_SIZE)} cards`);
  });

  it("R253 the verdict is validateDeck's list, verbatim, under 'Before you can queue this deck'", () => {
    const cards = ONE.slice(0, 4);
    mount({ decks: [savedDeck("a", "Aggro", cards, 1)], initialOpen: { kind: "deck", id: "a" } });
    const verdict = screen.getByTestId(DECK_VERDICT);
    expect(verdict).toHaveAttribute("data-ready", "false");
    expect(verdict).toHaveTextContent("Before you can queue this deck");
    const want = validateDeck({ deck: { name: "Aggro", cards }, catalog, collection });
    const messages = want.ok ? [] : want.errors.map((issue) => issue.message);
    expect(messages.length).toBeGreaterThan(0);
    expect(within(verdict).getAllByTestId(loadoutErrorId("L2")).map((node) => node.textContent)).toEqual(messages);
    expect(within(verdict).getByTestId(loadoutErrorId("L2"))).toHaveAttribute("data-rule", "L2");
  });

  it("R256 the twentieth card saves at once and says 'Deck complete and saved' once the server has it", async () => {
    const { server } = mount({ decks: [savedDeck("a", "Aggro", ONE.slice(0, DECK_SIZE - 1), 1)], initialOpen: { kind: "deck", id: "a" } });
    const last = ONE[DECK_SIZE - 1] ?? "";
    await act(async () => {
      fireEvent.click(screen.getByTestId(addPoolId(last)));
      await Promise.resolve();
    });
    // No debounce: completing a deck saves at once.
    await act(async () => {
      for (let turn = 0; turn < 50; turn += 1) await Promise.resolve();
    });
    expect(server.decks.get("a")?.cards).toHaveLength(DECK_SIZE);
    expect(screen.getByTestId(DECK_STATUS)).toHaveTextContent(DECK_COMPLETE_SAVED);
    expect(screen.getByTestId(DECK_VERDICT)).toHaveAttribute("data-ready", "true");
  });

  it("R250 Delete removes the deck at once and sends the DELETE once the undo window has passed", async () => {
    const { server, tick } = mount({
      decks: [savedDeck("a", "Aggro", [], 1), savedDeck("b", "Brawl", [], 2)],
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    expect(screen.queryByTestId(deckRowId("a"))).toBeNull();
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "b");
    await tick(UNDO_WINDOW_MS);
    expect(server.calls.some((call) => call.op === "deleteDeck" && call.id === "a")).toBe(true);
    expect(server.decks.has("a")).toBe(false);
  });
});

// Practice with this deck (#1991)
describe("issue #1991 Practice with this deck", () => {
  afterEach(() => {
    window.history.replaceState(null, "", "/");
  });

  /** Presses the open deck's "Practice with this deck" and says where it went. */
  function practiceTarget(): string {
    fireEvent.click(screen.getByTestId(DECK_TO_PRACTICE));
    return window.location.pathname + window.location.search;
  }

  it("issue #1991 a complete, saved third deck links to practice with it picked, by its place in the list", () => {
    mount({
      decks: [savedDeck("a", "Aggro", ONE, 1), savedDeck("b", "Brawl", TWO, 2), savedDeck("c", "Control", THREE, 3)],
      initialOpen: { kind: "deck", id: "c" },
    });
    const button = screen.getByTestId(DECK_TO_PRACTICE);
    expect(button).toHaveTextContent("Practice");
    expect(button).toHaveAccessibleName("Practice with this deck");
    expect(practiceTarget()).toBe("/practice?deck=saved:3");
    fireEvent.click(screen.getByTestId(deckRowId("a")));
    expect(practiceTarget()).toBe("/practice?deck=saved:1");
  });

  it("issue #1991 an incomplete deck offers no practice until its last card is saved", async () => {
    mount({ decks: [savedDeck("a", "Aggro", ONE.slice(0, DECK_SIZE - 1), 1)], initialOpen: { kind: "deck", id: "a" } });
    expect(screen.queryByTestId(DECK_TO_PRACTICE)).toBeNull();
    const last = ONE[DECK_SIZE - 1] ?? "";
    await act(async () => {
      fireEvent.click(screen.getByTestId(addPoolId(last)));
      await Promise.resolve();
    });
    await act(async () => {
      for (let turn = 0; turn < 50; turn += 1) await Promise.resolve();
    });
    expect(practiceTarget()).toBe("/practice?deck=saved:1");
  });

  it("issue #1991 no deck offers practice while another deck is not saved yet", async () => {
    const { tick } = mount({ decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    expect(screen.getByTestId(DECK_TO_PRACTICE)).toBeInTheDocument();
    fireEvent.click(screen.getByTestId(DECK_NEW));
    fireEvent.click(screen.getByTestId(deckRowId("a")));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "a");
    expect(screen.getByTestId(deckRowId("new-1"))).toHaveAttribute("data-unsynced", "true");
    expect(screen.queryByTestId(DECK_TO_PRACTICE)).toBeNull();
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(practiceTarget()).toBe("/practice?deck=saved:1");
  });

  it("issue #1991 a deletion the server has not had yet hides it, and the place moves up once it has", async () => {
    const { tick } = mount({
      decks: [savedDeck("a", "Aggro", ONE, 1), savedDeck("b", "Brawl", TWO, 2)],
      initialOpen: { kind: "deck", id: "b" },
    });
    expect(practiceTarget()).toBe("/practice?deck=saved:2");
    fireEvent.click(screen.getByTestId(deckRowId("a")));
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "b");
    expect(screen.queryByTestId(DECK_TO_PRACTICE)).toBeNull();
    await tick(UNDO_WINDOW_MS);
    expect(practiceTarget()).toBe("/practice?deck=saved:1");
  });
});

describe("comparing a deck with others (R251)", () => {
  const clash = TWO[0] ?? "";

  it("R251 cards a compared deck holds are unavailable in the pool, marked 'In <deck name>', and adding one is refused naming it", () => {
    mount({
      decks: [savedDeck("a", "Aggro", ONE.slice(0, 3), 1), savedDeck("c", "Control", TWO, 2)],
      initialOpen: { kind: "deck", id: "a" },
    });
    const before = screen.getByTestId(poolCardId(clash));
    expect(before).not.toHaveAttribute("data-unavailable");

    fireEvent.change(screen.getByTestId(DECK_COMPARE_SELECT), { target: { value: "deck:c" } });
    expect(screen.getByTestId(deckCompareChipId("c"))).toHaveTextContent("Control");
    const held = screen.getByTestId(poolCardId(clash));
    expect(held).toHaveAttribute("data-unavailable", "true");
    expect(held).toHaveAttribute("data-held-by", "Control");
    expect(held).toHaveAttribute("data-legal", "false");
    expect(held.querySelector(".db-held")).toHaveTextContent("In Control");
    expect(screen.getByTestId(poolCardId(THREE[0] ?? ""))).not.toHaveAttribute("data-unavailable");

    fireEvent.click(screen.getByTestId(addPoolId(clash)));
    expect(screen.queryByTestId(deckCardId(clash))).toBeNull();
    expect(screen.getByTestId(DECK_STATUS)).toHaveTextContent(`${nameOf(clash)} is in Control.`);
    expect(held).toHaveAttribute("data-refused", "true");

    fireEvent.click(screen.getByTestId(deckCompareChipId("c")));
    expect(screen.getByTestId(poolCardId(clash))).not.toHaveAttribute("data-unavailable");
    fireEvent.click(screen.getByTestId(addPoolId(clash)));
    expect(screen.getByTestId(deckCardId(clash))).toBeInTheDocument();
  });

  it("R251 picking a trio the deck is in compares it with that trio's other two decks", () => {
    mount({
      decks: [savedDeck("a", "Aggro", ONE.slice(0, 3), 1), savedDeck("c", "Control", TWO, 2), savedDeck("m", "Midrange", THREE, 3)],
      trios: [savedTrio("t", "Ladder", ["a", "c", "m"], 4)],
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.change(screen.getByTestId(DECK_COMPARE_SELECT), { target: { value: "trio:t" } });
    expect(screen.getByTestId(deckCompareChipId("c"))).toBeInTheDocument();
    expect(screen.getByTestId(deckCompareChipId("m"))).toBeInTheDocument();
    expect(screen.getByTestId(poolCardId(TWO[3] ?? ""))).toHaveAttribute("data-held-by", "Control");
    expect(screen.getByTestId(poolCardId(THREE[3] ?? ""))).toHaveAttribute("data-held-by", "Midrange");

    fireEvent.change(screen.getByTestId(DECK_COMPARE_SELECT), { target: { value: "none" } });
    expect(screen.queryByTestId(deckCompareChipId("c"))).toBeNull();
    expect(screen.getByTestId(poolCardId(TWO[3] ?? ""))).not.toHaveAttribute("data-unavailable");
  });

  it("R251 a clash that already exists is shown on the tile and counted, never taken out", () => {
    mount({
      decks: [savedDeck("a", "Aggro", [...ONE.slice(0, 3), clash], 1), savedDeck("c", "Control", TWO, 2)],
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.change(screen.getByTestId(DECK_COMPARE_SELECT), { target: { value: "deck:c" } });
    const tile = screen.getByTestId(deckCardId(clash));
    expect(tile).toHaveAttribute("data-conflict", "true");
    expect(tile).toHaveAttribute("data-conflict-with", "Control");
    expect(screen.getByTestId(deckCardId(ONE[0] ?? ""))).not.toHaveAttribute("data-conflict");
    expect(screen.getByTestId(DECK_CONFLICTS)).toHaveAttribute("data-count", "1");
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", "4");
  });
});

describe("the deck list marks each card that keeps the deck from being queued", () => {
  function errorsOf(cards: readonly string[], owned: Collection) {
    const verdict = validateDeck({ deck: { name: "Aggro", cards }, catalog, collection: owned });
    return verdict.ok ? [] : verdict.errors;
  }

  it("a card the collection owns none of is marked L5 with the validator's sentence; a card with no error is not marked", () => {
    const unowned = ONE[0] ?? "";
    const missing: Collection = Object.fromEntries(Object.entries(collection).filter(([id]) => id !== unowned));
    const cards = ONE.slice(0, 4);
    mount({ decks: [savedDeck("a", "Aggro", cards, 1)], collection: missing, initialOpen: { kind: "deck", id: "a" } });

    const issue = errorsOf(cards, missing).find((error) => error.rule === "L5" && error.cardId === unowned);
    expect(issue).toBeDefined();
    const message = issue?.message ?? "";
    const marked = screen.getByTestId(deckCardId(unowned));
    expect(marked).toHaveAttribute("data-issue", "L5");
    expect(marked).toHaveAccessibleName(`Remove ${nameOf(unowned)} from Aggro: ${message}`);
    expect(marked).toHaveAttribute("title", message);
    expect(marked.querySelector(".db-tile-flag")).not.toBeNull();

    const other = ONE[1] ?? "";
    const plain = screen.getByTestId(deckCardId(other));
    expect(plain).not.toHaveAttribute("data-issue");
    expect(plain).not.toHaveAttribute("title");
    expect(plain).toHaveAccessibleName(`Remove ${nameOf(other)} from Aggro`);
    expect(plain.querySelector(".db-tile-flag")).toBeNull();

    // L2 names no card, so it stays in the verdict box only.
    expect(within(screen.getByTestId(DECK_VERDICT)).getByTestId(loadoutErrorId("L2"))).toBeInTheDocument();
  });

  it("a card two errors name shows the first: a token is L3 before L5", () => {
    const cards = [...ONE.slice(0, 3), TOKEN_ID];
    mount({ decks: [savedDeck("a", "Aggro", cards, 1)], initialOpen: { kind: "deck", id: "a" } });

    const named = errorsOf(cards, collection).filter((error) => error.cardId === TOKEN_ID);
    expect(named.map((error) => error.rule)).toEqual(["L3", "L5"]);
    const tile = screen.getByTestId(deckCardId(TOKEN_ID));
    expect(tile).toHaveAttribute("data-issue", "L3");
    expect(tile).toHaveAttribute("title", named[0]?.message);
  });

  it("a card that clashes with a compared deck and has an error gives both reasons", () => {
    const clash = TWO[0] ?? "";
    const missing: Collection = Object.fromEntries(Object.entries(collection).filter(([id]) => id !== clash));
    const cards = [...ONE.slice(0, 3), clash];
    mount({
      decks: [savedDeck("a", "Aggro", cards, 1), savedDeck("c", "Control", TWO, 2)],
      collection: missing,
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.change(screen.getByTestId(DECK_COMPARE_SELECT), { target: { value: "deck:c" } });

    const message = errorsOf(cards, missing).find((error) => error.cardId === clash)?.message ?? "";
    expect(message).not.toBe("");
    const tile = screen.getByTestId(deckCardId(clash));
    expect(tile).toHaveAttribute("data-conflict", "true");
    expect(tile).toHaveAttribute("data-issue", "L5");
    expect(tile).toHaveAccessibleName(`Remove ${nameOf(clash)} from Aggro (also in Control): ${message}`);
    expect(tile).toHaveAttribute("title", `Also in Control. ${message}`);
    expect(tile.querySelectorAll(".db-tile-flag")).toHaveLength(1);
  });

  it("with no collection no ownership mark appears, the other marks stay, and the hint says why", () => {
    const cards = [...ONE.slice(0, 3), TOKEN_ID];
    mount({ decks: [savedDeck("a", "Aggro", cards, 1)], collection: null, initialOpen: { kind: "deck", id: "a" } });

    for (const cardId of ONE.slice(0, 3)) {
      expect(screen.getByTestId(deckCardId(cardId))).not.toHaveAttribute("data-issue");
    }
    expect(screen.getByTestId(deckCardId(TOKEN_ID))).toHaveAttribute("data-issue", "L3");
    const verdict = screen.getByTestId(DECK_VERDICT);
    expect(within(verdict).queryAllByTestId(loadoutErrorId("L5"))).toHaveLength(0);
    expect(verdict).toHaveTextContent("Your collection couldn");
  });
});

describe("a trio", () => {
  const full = [savedDeck("a", "Aggro", ONE, 1), savedDeck("c", "Control", TWO, 2), savedDeck("m", "Midrange", THREE, 3)];

  it("R253 three full decks with no card in common are Ready for Conquest", () => {
    mount({ decks: full, trios: [savedTrio("t", "Ladder", ["a", "c", "m"], 4)], initialOpen: { kind: "trio", id: "t" } });
    const verdict = screen.getByTestId(TRIO_VERDICT);
    expect(verdict).toHaveAttribute("data-ready", "true");
    expect(verdict).toHaveTextContent("Ready for Conquest");
    expect(screen.getByTestId(trioRowId("t"))).toHaveAttribute("data-ready", "true");
    expect(screen.getByTestId(trioCardId(1, ONE[0] ?? ""))).toHaveAttribute("data-conflict", "false");
  });

  it("R252 a trio with an empty slot is saved, and its verdict is the validator's L1, verbatim", async () => {
    const { server, tick } = mount({ decks: full, initialOpen: null });
    fireEvent.click(screen.getByTestId(TRIO_NEW));
    expect(screen.getByTestId(TRIO_EDITOR)).toHaveAttribute("data-trio", "new-1");
    expect(screen.getByTestId(TRIO_NAME_INPUT)).toHaveValue("Trio 1");
    fireEvent.change(screen.getByTestId(trioSlotId(1)), { target: { value: "a" } });
    fireEvent.change(screen.getByTestId(trioSlotId(3)), { target: { value: "m" } });

    const verdict = screen.getByTestId(TRIO_VERDICT);
    expect(verdict).toHaveAttribute("data-ready", "false");
    const want = validateTrio({
      decks: [
        { name: "Aggro", cards: ONE },
        { name: "Midrange", cards: THREE },
      ],
      catalog,
      collection,
    });
    const l1 = want.ok ? [] : want.errors.filter((issue) => issue.rule === "L1").map((issue) => issue.message);
    expect(l1).toHaveLength(1);
    expect(within(verdict).getAllByTestId(loadoutErrorId("L1")).map((node) => node.textContent)).toEqual(l1);

    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.trios.get("new-1")).toEqual({ name: "Trio 1", deckIds: ["a", null, "m"] });
  });

  it("R251 R252 decks that share a card are saved but not ready: the shared card is marked on both sides, and L4 names both decks", () => {
    const shared = TWO[0] ?? "";
    const clashing = [savedDeck("a", "Aggro", [...ONE.slice(1), shared], 1), full[1] as SavedDeck, full[2] as SavedDeck];
    mount({ decks: clashing, trios: [savedTrio("t", "Ladder", ["a", "c", "m"], 4)], initialOpen: { kind: "trio", id: "t" } });

    const compare = screen.getByTestId(TRIO_COMPARE);
    const inAggro = within(compare).getByTestId(trioCardId(1, shared));
    const inControl = within(compare).getByTestId(trioCardId(2, shared));
    expect(inAggro).toHaveAttribute("data-conflict", "true");
    expect(inAggro).toHaveAttribute("data-conflict-with", "Control");
    expect(inControl).toHaveAttribute("data-conflict", "true");
    expect(inControl).toHaveAttribute("data-conflict-with", "Aggro");
    expect(inAggro.parentElement).toHaveTextContent("Also in Control");
    expect(within(compare).getByTestId(trioCardId(3, THREE[0] ?? ""))).toHaveAttribute("data-conflict", "false");

    const verdict = screen.getByTestId(TRIO_VERDICT);
    expect(verdict).toHaveAttribute("data-ready", "false");
    const want = validateTrio({
      decks: [
        { name: "Aggro", cards: clashing[0]?.cards ?? [] },
        { name: "Control", cards: TWO },
        { name: "Midrange", cards: THREE },
      ],
      catalog,
      collection,
    });
    const l4 = want.ok ? [] : want.errors.filter((issue) => issue.rule === "L4").map((issue) => issue.message);
    expect(l4).toHaveLength(1);
    expect(within(verdict).getAllByTestId(loadoutErrorId("L4")).map((node) => node.textContent)).toEqual(l4);
    expect(screen.getByTestId(trioRowId("t"))).toHaveAttribute("data-ready", "false");
  });

  it("R252 a slot cannot pick a deck another slot holds", () => {
    mount({ decks: full, trios: [savedTrio("t", "Ladder", ["a", null, null], 4)], initialOpen: { kind: "trio", id: "t" } });
    const second = screen.getByTestId(trioSlotId(2));
    const option = within(second).getByRole("option", { name: /Aggro/ });
    expect(option).toBeDisabled();
    expect(within(second).getByRole("option", { name: "Control" })).toBeEnabled();
  });

  it("R252 deleting a deck empties the trio slot that named it", async () => {
    const { server, tick } = mount({ decks: full, trios: [savedTrio("t", "Ladder", ["a", "c", "m"], 4)], initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(trioOpenDeckId(2)));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "c");
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    fireEvent.click(screen.getByTestId(trioRowId("t")));
    expect(screen.getByTestId(trioSlotId(2))).toHaveValue("");
    await tick(UNDO_WINDOW_MS);
    expect(server.trios.get("t")?.deckIds).toEqual(["a", null, "m"]);
  });

  it("R252 Delete trio removes only the trio", async () => {
    const { server, tick } = mount({ decks: full, trios: [savedTrio("t", "Ladder", ["a", "c", "m"], 4)], initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(TRIO_DELETE));
    expect(screen.queryByTestId(trioRowId("t"))).toBeNull();
    expect(screen.getByTestId(deckRowId("a"))).toBeInTheDocument();
    await tick(UNDO_WINDOW_MS);
    expect(server.trios.has("t")).toBe(false);
    expect(server.decks.has("a")).toBe(true);
  });
});

// Undoing a deletion (R2119000)
describe("undoing a deletion (R2119000)", () => {
  const full = [savedDeck("a", "Aggro", ONE, 1), savedDeck("c", "Control", TWO, 2), savedDeck("m", "Midrange", THREE, 3)];
  const ladder = savedTrio("t", "Ladder", ["a", "c", "m"], 4);
  const second = savedTrio("u", "Second", [null, null, null], 5);

  /** The rows a list shows, in its order. */
  function rows(listId: string): string[] {
    return Array.from(screen.getByTestId(listId).querySelectorAll("[data-testid^='deck-row-'], [data-testid^='trio-row-']")).map(
      (row) => row.getAttribute("data-testid") ?? "",
    );
  }

  async function pageHide(): Promise<void> {
    await act(async () => {
      window.dispatchEvent(new Event("pagehide"));
      await settle();
    });
  }

  it("R2119000 Delete deck takes it off at once behind \"Deck deleted · Undo\" and sends nothing", async () => {
    const { server, tick } = mount({ decks: full, initialOpen: { kind: "deck", id: "c" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    expect(rows(DECK_LIST)).toEqual([deckRowId("a"), deckRowId("m")]);
    expect(screen.getByTestId(UNDO_TOAST)).toHaveTextContent("Deck deleted · Undo");
    await tick(UNDO_WINDOW_MS - 1);
    expect(server.calls).toEqual([]);
  });

  it("R2119000 Undo puts the deck back in its place, open, and fills its trio slots again, sending nothing", async () => {
    const { server, tick } = mount({ decks: full, trios: [ladder], initialOpen: { kind: "deck", id: "c" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    fireEvent.click(screen.getByTestId(trioRowId("t")));
    expect(screen.getByTestId(trioSlotId(2))).toHaveValue("");

    fireEvent.click(screen.getByTestId(UNDO_TOAST_UNDO));
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
    expect(rows(DECK_LIST)).toEqual([deckRowId("a"), deckRowId("c"), deckRowId("m")]);
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "c");
    fireEvent.click(screen.getByTestId(trioRowId("t")));
    expect(screen.getByTestId(trioSlotId(2))).toHaveValue("c");
    await tick(UNDO_WINDOW_MS * 2);
    expect(server.calls).toEqual([]);
    expect(server.decks.has("c")).toBe(true);
  });

  it("R2119000 the window's end sends the deck's DELETE exactly once", async () => {
    const { server, tick } = mount({ decks: full, initialOpen: { kind: "deck", id: "c" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    await tick(UNDO_WINDOW_MS);
    expect(server.calls).toEqual([{ op: "deleteDeck", id: "c" }]);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
    await tick(UNDO_WINDOW_MS * 2);
    expect(server.calls).toHaveLength(1);
    expect(syncState()).toBe("saved");
  });

  it("R2119000 pagehide sends the held DELETE at once, with keepalive", async () => {
    const { server } = mount({ decks: full, initialOpen: { kind: "deck", id: "c" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    await pageHide();
    expect(server.calls).toEqual([{ op: "deleteDeck", id: "c", keepalive: true }]);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
  });

  it("R2119000 leaving the workshop sends the held DELETE", async () => {
    const { server } = mount({ decks: full, initialOpen: { kind: "deck", id: "c" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    cleanup();
    await settle();
    expect(server.calls).toEqual([{ op: "deleteDeck", id: "c" }]);
  });

  it("R2119000 a refused DELETE puts the deck back and shows the server's words", async () => {
    const server = fakeDeckServer();
    const message = "This account can't change its decks.";
    server.state.refusals.set("c", new ApiRequestError(403, { code: "forbidden", message }));
    const { tick } = mount({ decks: full, server, initialOpen: { kind: "deck", id: "c" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    await tick(UNDO_WINDOW_MS);
    expect(server.calls).toEqual([{ op: "deleteDeck", id: "c" }]);
    expect(rows(DECK_LIST)).toEqual([deckRowId("a"), deckRowId("c"), deckRowId("m")]);
    expect(syncState()).toBe("error");
    expect(screen.getByTestId(SYNC_STATUS)).toHaveTextContent(message);
  });

  it("R2119000 while the delete is held nothing saves the deck and no trio shows it", async () => {
    const { server, tick } = mount({ decks: full, trios: [ladder], initialOpen: { kind: "deck", id: "a" } });
    expect(screen.getByTestId(trioRowId("t"))).toHaveTextContent("3 of 3 decks");
    fireEvent.change(screen.getByTestId(DECK_NAME_INPUT), { target: { value: "Aggro 2" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    expect(screen.getByTestId(trioRowId("t"))).toHaveTextContent("2 of 3 decks");
    await tick(UNDO_WINDOW_MS - 1);
    expect(server.calls, "the rename waits with the deck").toEqual([]);
    fireEvent.click(screen.getByTestId(trioRowId("t")));
    expect(screen.getByTestId(trioSlotId(1))).toHaveValue("");

    fireEvent.click(screen.getByTestId(UNDO_TOAST_UNDO));
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.calls.map((call) => `${call.op} ${call.id}`)).toEqual(["putDeck a"]);
    expect(server.decks.get("a")?.name).toBe("Aggro 2");
  });

  it("R2119000 a second delete sends the first's DELETE at once and takes the toast", async () => {
    const { server } = mount({ decks: full, initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "c");
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    await act(async () => {
      await settle();
    });
    expect(server.calls).toEqual([{ op: "deleteDeck", id: "a" }]);
    expect(screen.getAllByTestId(UNDO_TOAST)).toHaveLength(1);
    expect(rows(DECK_LIST)).toEqual([deckRowId("m")]);

    fireEvent.click(screen.getByTestId(UNDO_TOAST_UNDO));
    expect(rows(DECK_LIST)).toEqual([deckRowId("c"), deckRowId("m")]);
  });

  it("R2119000 New deck at the cap sends the held DELETE first", async () => {
    const decks = Array.from({ length: MAX_SAVED_DECKS }, (_unused, index) => savedDeck(`d${String(index)}`, `Deck ${String(index)}`, [], index));
    const { server, tick } = mount({ decks, initialOpen: { kind: "deck", id: "d0" } });
    expect(screen.getByTestId(DECK_NEW)).toBeDisabled();
    fireEvent.click(screen.getByTestId(DECK_DELETE));
    fireEvent.click(screen.getByTestId(DECK_NEW));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.calls.map((call) => `${call.op} ${call.id}`)).toEqual(["deleteDeck d0", "putDeck new-1"]);
  });

  it("R2119000 Delete trio takes it off at once behind \"Trio deleted · Undo\" and sends nothing", async () => {
    const { server, tick } = mount({ decks: full, trios: [ladder, second], initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(TRIO_DELETE));
    expect(rows(TRIO_LIST)).toEqual([trioRowId("u")]);
    expect(screen.getByTestId(UNDO_TOAST)).toHaveTextContent("Trio deleted · Undo");
    await tick(UNDO_WINDOW_MS - 1);
    expect(server.calls).toEqual([]);
  });

  it("R2119000 Undo puts the trio back in its place, open, sending nothing", async () => {
    const { server, tick } = mount({ decks: full, trios: [ladder, second], initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(TRIO_DELETE));
    fireEvent.click(screen.getByTestId(UNDO_TOAST_UNDO));
    expect(rows(TRIO_LIST)).toEqual([trioRowId("t"), trioRowId("u")]);
    expect(screen.getByTestId(TRIO_EDITOR)).toHaveAttribute("data-trio", "t");
    await tick(UNDO_WINDOW_MS * 2);
    expect(server.calls).toEqual([]);
    expect(server.trios.has("t")).toBe(true);
  });

  it("R2119000 the window's end sends the trio's DELETE exactly once", async () => {
    const { server, tick } = mount({ decks: full, trios: [ladder], initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(TRIO_DELETE));
    await tick(UNDO_WINDOW_MS);
    expect(server.calls).toEqual([{ op: "deleteTrio", id: "t" }]);
    await tick(UNDO_WINDOW_MS * 2);
    expect(server.calls).toHaveLength(1);
    expect(server.trios.has("t")).toBe(false);
  });

  it("R2119000 pagehide sends the held trio DELETE at once, with keepalive", async () => {
    const { server } = mount({ decks: full, trios: [ladder], initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(TRIO_DELETE));
    await pageHide();
    expect(server.calls).toEqual([{ op: "deleteTrio", id: "t", keepalive: true }]);
  });

  it("R2119000 a refused trio DELETE puts the trio back and shows the server's words", async () => {
    const server = fakeDeckServer();
    const message = "This account can't change its trios.";
    server.state.refusals.set("t", new ApiRequestError(403, { code: "forbidden", message }));
    const { tick } = mount({ decks: full, trios: [ladder, second], server, initialOpen: { kind: "trio", id: "t" } });
    fireEvent.click(screen.getByTestId(TRIO_DELETE));
    await tick(UNDO_WINDOW_MS);
    expect(rows(TRIO_LIST)).toEqual([trioRowId("t"), trioRowId("u")]);
    expect(syncState()).toBe("error");
    expect(screen.getByTestId(SYNC_STATUS)).toHaveTextContent(message);
  });
});

describe("deck codes (R255)", () => {
  function stubClipboard(writeText: (text: string) => Promise<void>): void {
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  }

  afterEach(() => {
    Reflect.deleteProperty(navigator, "clipboard");
  });

  it("R255 Copy code puts the code on the clipboard and in the read-only field, and it reads back as the deck", async () => {
    const writeText = vi.fn(async () => {});
    stubClipboard(writeText);
    mount({ decks: [savedDeck("a", "Aggro", ONE.slice(0, 6), 1)], initialOpen: { kind: "deck", id: "a" } });
    await act(async () => {
      fireEvent.click(screen.getByTestId(DECK_COPY_CODE));
      await Promise.resolve();
    });
    const output = screen.getByTestId(DECK_CODE_OUTPUT);
    expect(output).toHaveAttribute("readonly");
    const code = (output as HTMLInputElement).value;
    expect(writeText).toHaveBeenCalledWith(code);
    const decoded = decodeDeckCode(code, catalog, collection);
    expect(decoded.ok && decoded.name).toBe("Aggro");
    expect(decoded.ok && decoded.cards).toEqual(ONE.slice(0, 6));
    expect(screen.getByTestId(DECK_STATUS)).toHaveTextContent("Code copied");
  });

  it("R255 with no clipboard the code is still shown to copy by hand", () => {
    mount({ decks: [savedDeck("a", "Aggro", ONE.slice(0, 2), 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(DECK_COPY_CODE));
    expect((screen.getByTestId(DECK_CODE_OUTPUT) as HTMLInputElement).value).toBe(encodeDeckCode("Aggro", ONE.slice(0, 2), catalog));
    expect(screen.getByTestId(DECK_STATUS)).toHaveTextContent("Copy the code below");
  });

  it("R255 an import previews the code and makes a NEW deck from it, which is saved", async () => {
    const { server, tick } = mount({ decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    const code = encodeDeckCode("Shared list", TWO.slice(0, 5), catalog);
    fireEvent.click(screen.getByTestId(DECK_IMPORT_OPEN));
    expect(screen.getByTestId(DECK_IMPORT)).toBeInTheDocument();
    fireEvent.change(screen.getByTestId(DECK_IMPORT_INPUT), { target: { value: `  ${code.slice(0, 10)}\n${code.slice(10)}  ` } });
    const preview = screen.getByTestId(DECK_IMPORT_PREVIEW);
    expect(preview).toHaveAttribute("data-ok", "true");
    expect(preview).toHaveTextContent("Shared list");
    expect(preview).toHaveTextContent(`5/${String(DECK_SIZE)}`);

    fireEvent.click(screen.getByTestId(DECK_IMPORT_SUBMIT));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Shared list");
    expect(screen.getByTestId(deckRowId("a")), "the deck that was open is untouched").toHaveAttribute("data-count", String(DECK_SIZE));
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("new-1")).toMatchObject({ name: "Shared list", cards: TWO.slice(0, 5) });
  });

  it("R255 the preview says what was left out and which kept cards are not owned", () => {
    const extra: CardDef = { ...(catalog.cards[fixtureCardId(1)] as CardDef), id: "core-999", index: "999", name: "From the future" };
    const newer = { ...catalog, cards: { ...catalog.cards, [extra.id]: extra } };
    const code = encodeDeckCode("Mixed", [fixtureCardId(1), extra.id, fixtureCardId(2)], newer);
    const missing: Record<string, number> = { ...collection };
    delete missing[fixtureCardId(2)];
    mount({ collection: missing, initialOpen: { kind: "import" } });
    fireEvent.change(screen.getByTestId(DECK_IMPORT_INPUT), { target: { value: code } });
    const preview = screen.getByTestId(DECK_IMPORT_PREVIEW);
    expect(preview).toHaveAttribute("data-ok", "true");
    expect(preview).toHaveTextContent(`2/${String(DECK_SIZE)}`);
    expect(preview).toHaveTextContent("#999");
    expect(preview).toHaveTextContent(fixtureCardName(2));
  });

  it("R255 the preview names every kind of card an import leaves out", () => {
    const lines = droppedLines(
      {
        unknown: [101, 140],
        tokens: [TOKEN_ID],
        duplicates: [fixtureCardId(1), fixtureCardId(1)],
        overflow: [fixtureCardId(2), fixtureCardId(3)],
      },
      catalog,
    );
    expect(lines).toHaveLength(4);
    expect(lines[0]).toContain("#101, #140");
    expect(lines[1]).toContain(nameOf(TOKEN_ID));
    expect(lines[2]).toContain("2 extra copies");
    expect(lines[2]).toContain(fixtureCardName(1));
    expect(lines[3]).toContain(`2 cards past the ${String(DECK_SIZE)}`);
    expect(droppedLines({ unknown: [], tokens: [], duplicates: [], overflow: [] }, catalog)).toEqual([]);
  });

  it("R255 a code that cannot be read shows the decoder's own sentence and imports nothing", () => {
    mount({ initialOpen: { kind: "import" } });
    fireEvent.change(screen.getByTestId(DECK_IMPORT_INPUT), { target: { value: "hello there" } });
    const decoded = decodeDeckCode("hello there", catalog, collection);
    expect(decoded.ok).toBe(false);
    const preview = screen.getByTestId(DECK_IMPORT_PREVIEW);
    expect(preview).toHaveAttribute("data-ok", "false");
    expect(preview).toHaveTextContent(decoded.ok ? "" : decoded.message);
    expect(screen.getByTestId(DECK_IMPORT_SUBMIT)).toBeDisabled();
  });

  it("R255 at the deck cap the import is off, with the reason beside it", () => {
    const decks = Array.from({ length: MAX_SAVED_DECKS }, (_unused, index) => savedDeck(`d${String(index)}`, `Deck ${String(index)}`, [], index));
    mount({ decks, initialOpen: { kind: "import" } });
    fireEvent.change(screen.getByTestId(DECK_IMPORT_INPUT), { target: { value: encodeDeckCode("One more", ONE.slice(0, 2), catalog) } });
    expect(screen.getByTestId(DECK_IMPORT_PREVIEW)).toHaveAttribute("data-ok", "true");
    expect(screen.getByTestId(DECK_IMPORT_SUBMIT)).toBeDisabled();
    expect(screen.getByTestId(DECK_IMPORT_CAP_REASON)).toHaveTextContent(String(MAX_SAVED_DECKS));
  });
});

describe("trio codes (R339–R341)", () => {
  function stubClipboard(writeText: (text: string) => Promise<void>): void {
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
  }

  afterEach(() => {
    Reflect.deleteProperty(navigator, "clipboard");
  });

  const trioDecks = (): SavedDeck[] => [savedDeck("a", "Aggro", ONE, 1), savedDeck("b", "Control", TWO, 2), savedDeck("c", "Ramp", THREE, 3)];

  it("R339 Copy trio code puts the code on the clipboard and in the field, and it reads back as the trio", async () => {
    const writeText = vi.fn(async () => {});
    stubClipboard(writeText);
    mount({ decks: trioDecks(), trios: [savedTrio("t", "Main trio", ["a", null, "c"], 1)], initialOpen: { kind: "trio", id: "t" } });
    await act(async () => {
      fireEvent.click(screen.getByTestId(TRIO_COPY_CODE));
      await Promise.resolve();
    });
    const code = (screen.getByTestId(TRIO_CODE_OUTPUT) as HTMLInputElement).value;
    expect(writeText).toHaveBeenCalledWith(code);
    const decoded = decodeTrioCode(code, catalog, collection);
    expect(decoded.ok && decoded.name).toBe("Main trio");
    expect(decoded.ok && decoded.slots.map((slot) => slot?.name ?? null)).toEqual(["Aggro", null, "Ramp"]);
    expect(decoded.ok && decoded.slots[2]?.cards).toEqual(THREE);
    expect(screen.getByTestId(TRIO_EDITOR)).toHaveTextContent("Trio code copied");
  });

  it("R341 an import previews every deck, flags shared cards, and makes three new decks and a trio in one request", async () => {
    const { server } = mount({ decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    const shared = [TWO[0] ?? "", TWO[1] ?? ""];
    const code = encodeTrioCode(
      "Friend's trio",
      [
        { name: "Tempo", cards: TWO.slice(0, 8) },
        { name: "Burn", cards: [...shared, ...THREE.slice(0, 4)] },
        { name: "Walls", cards: THREE.slice(10, 13) },
      ],
      catalog,
    );
    fireEvent.click(screen.getByTestId(TRIO_IMPORT_OPEN));
    expect(screen.getByTestId(TRIO_IMPORT)).toBeInTheDocument();
    fireEvent.change(screen.getByTestId(TRIO_IMPORT_INPUT), { target: { value: code } });
    const preview = screen.getByTestId(TRIO_IMPORT_PREVIEW);
    expect(preview).toHaveAttribute("data-ok", "true");
    expect(preview).toHaveTextContent("Friend's trio");
    expect(screen.getByTestId(trioImportSlotId(1))).toHaveAttribute("data-count", "8");
    expect(screen.getByTestId(trioImportSlotId(2))).toHaveTextContent("Burn");
    expect(screen.getByTestId(TRIO_IMPORT_SHARED)).toHaveAttribute("data-count", "2");
    expect(screen.getByTestId(TRIO_IMPORT_SHARED)).toHaveTextContent(nameOf(shared[0] ?? ""));

    await act(async () => {
      fireEvent.click(screen.getByTestId(TRIO_IMPORT_SUBMIT));
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(screen.getByTestId(TRIO_EDITOR)).toHaveAttribute("data-trio", "new-1");
    });
    expect(server.calls.filter((call) => call.op !== "putDeck")).toEqual([
      expect.objectContaining({ op: "importTrio", id: "new-1" }),
    ]);
    expect(screen.getByTestId(TRIO_NAME_INPUT)).toHaveValue("Friend's trio");
    expect(screen.getByTestId(trioRowId("new-1"))).toBeInTheDocument();
    expect(server.decks.get("new-2")).toMatchObject({ name: "Tempo", cards: TWO.slice(0, 8) });
    expect(screen.getByTestId(trioCardId(1, shared[0] ?? ""))).toHaveAttribute("data-conflict", "true");
    expect(screen.getByTestId(deckRowId("a")), "what was there is untouched").toHaveAttribute("data-count", String(DECK_SIZE));
  });

  it("R340 at the caps the import is off, with exactly the slots it needs, and nothing is made", () => {
    const decks = Array.from({ length: MAX_SAVED_DECKS - 1 }, (_unused, index) => savedDeck(`d${String(index)}`, `Deck ${String(index)}`, [], index));
    const { server } = mount({ decks, initialOpen: { kind: "trio-import" } });
    fireEvent.change(screen.getByTestId(TRIO_IMPORT_INPUT), {
      target: {
        value: encodeTrioCode(
          "Too many",
          [
            { name: "A", cards: ONE.slice(0, 2) },
            { name: "B", cards: TWO.slice(0, 2) },
            { name: "C", cards: THREE.slice(0, 2) },
          ],
          catalog,
        ),
      },
    });
    const room = checkImportRoom({
      saved: { decks: MAX_SAVED_DECKS - 1, trios: 0 },
      limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS },
      adding: { decks: 3, trios: 1 },
    });
    const reason = screen.getByTestId(TRIO_IMPORT_CAP_REASON);
    expect(reason).toHaveTextContent(room.ok ? "" : room.message);
    expect(reason).toHaveAttribute("data-decks-short", "2");
    expect(screen.getByTestId(TRIO_IMPORT_SUBMIT)).toBeDisabled();
    fireEvent.click(screen.getByTestId(TRIO_IMPORT_SUBMIT));
    expect(server.calls).toEqual([]);
  });

  it("R341 a refusal is shown in the server's words, and nothing is made", async () => {
    const server = fakeDeckServer();
    server.state.refusals.set(
      "new-1",
      new ApiRequestError(409, { code: "conflict", message: "Your decks changed while this import was on its way." }),
    );
    mount({ server, initialOpen: { kind: "trio-import" } });
    fireEvent.change(screen.getByTestId(TRIO_IMPORT_INPUT), {
      target: { value: encodeTrioCode("Refused", [{ name: "A", cards: ONE.slice(0, 2) }, null, null], catalog) },
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId(TRIO_IMPORT_SUBMIT));
      await Promise.resolve();
    });
    expect(await screen.findByTestId(TRIO_IMPORT_ERROR)).toHaveTextContent("Your decks changed while this import was on its way.");
    expect(screen.getByTestId(TRIO_IMPORT)).toBeInTheDocument();
    expect(screen.queryByTestId(trioRowId("new-1"))).toBeNull();
  });

  it("R339 a deck code or a damaged code shows the decoder's own sentence and imports nothing", () => {
    mount({ initialOpen: { kind: "trio-import" } });
    fireEvent.change(screen.getByTestId(TRIO_IMPORT_INPUT), { target: { value: encodeDeckCode("Deck", ONE.slice(0, 2), catalog) } });
    expect(screen.getByTestId(TRIO_IMPORT_PREVIEW)).toHaveTextContent(TRIO_CODE_MESSAGES.deckCode);
    expect(screen.getByTestId(TRIO_IMPORT_SUBMIT)).toBeDisabled();
    fireEvent.change(screen.getByTestId(TRIO_IMPORT_INPUT), { target: { value: "JKT1.AAAA" } });
    expect(screen.getByTestId(TRIO_IMPORT_PREVIEW)).toHaveAttribute("data-ok", "false");
    expect(screen.getByTestId(TRIO_IMPORT_SUBMIT)).toBeDisabled();
  });
});

describe("the save status (R256)", () => {
  it("R256 offline, the edit is kept on this device, and the next visit restores it and saves it", async () => {
    const server = fakeDeckServer();
    const storage = memoryStorage();
    const decks = [savedDeck("a", "Aggro", ONE.slice(0, 2), 1)];
    server.state.offline = true;
    const first = mount({ decks, server, storage, initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(addPoolId(ONE[5] ?? "")));
    await first.tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(syncState()).toBe("offline");
    expect(screen.getByTestId(SYNC_STATUS)).toHaveTextContent(syncWords({ state: "offline", message: OFFLINE_MESSAGE }));
    expect(storage.data.get(mirrorKey(TEST_PROFILE))).toContain(ONE[5] ?? "");
    cleanup();

    server.state.offline = false;
    const second = mount({ decks, server, storage, initialOpen: { kind: "deck", id: "a" } });
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", "3");
    expect(screen.getByTestId(deckRowId("a"))).toHaveAttribute("data-unsynced", "true");
    await second.tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.cards).toEqual([...ONE.slice(0, 2), ONE[5]]);
    expect(syncState()).toBe("saved");
  });

  it("R256 a refusal shows the server's message in the status line and on the deck", async () => {
    const server = fakeDeckServer();
    const message = "This page is out of date. Reload it to keep building.";
    server.state.refusals.set("a", new ApiRequestError(409, { code: "update_required", message }));
    const { tick } = mount({ decks: [savedDeck("a", "Aggro", [], 1)], server, initialOpen: { kind: "deck", id: "a" } });
    fireEvent.change(screen.getByTestId(DECK_NAME_INPUT), { target: { value: "Aggro 2" } });
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(syncState()).toBe("error");
    expect(screen.getByTestId(SYNC_STATUS)).toHaveTextContent(message);
    expect(screen.getByTestId(DECK_SAVE_ERROR)).toHaveTextContent(message);
  });

  it("R256 the status line is a polite live region", () => {
    mount();
    const status = screen.getByTestId(SYNC_STATUS);
    expect(status).toHaveAttribute("role", "status");
    expect(status).toHaveAttribute("aria-live", "polite");
  });
});

describe("the deck's playmat, backdrop and card back (R1297000)", () => {
  it("R1297000 the editor's selects show the deck's picks and a pick saves", async () => {
    const { server, tick } = mount({
      decks: [{ ...savedDeck("a", "Aggro", ONE, 1), tableCosmetic: "table-felt", cardBackCosmetic: "card-back-gone" }],
      initialOpen: { kind: "deck", id: "a" },
    });
    // The picks live behind the top-bar button, so the sidebar keeps its first screen.
    expect(screen.queryByTestId("deck-cosmetic-table")).toBeNull();
    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    const select = (kind: string): HTMLSelectElement => screen.getByTestId<HTMLSelectElement>(`deck-cosmetic-${kind}`);

    // The deck's own pick, "As equipped in Collection" where it has none, and a pick no registry lists reads as the default.
    expect(select("table").value).toBe("table-felt");
    expect(select("field").value).toBe("");
    expect(select("cardBack").value).toBe("card-back-standard");
    // With nothing deciding ownership above it, every backdrop is offered (R1833000).
    expect([...select("field").options].map((option) => option.value)).toEqual(["", ...cosmeticsOf("field").map((cosmetic) => cosmetic.id)]);
    expect(select("field").options[0]?.textContent).toBe("As equipped in Collection");

    fireEvent.change(select("field"), { target: { value: "field-rain" } });
    expect(select("field").value).toBe("field-rain");
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")).toMatchObject({ tableCosmetic: "table-felt", fieldCosmetic: "field-rain", cardBackCosmetic: "card-back-gone" });

    fireEvent.change(select("table"), { target: { value: "" } });
    expect(select("table").value).toBe("");
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.tableCosmetic).toBeUndefined();
    expect(server.decks.get("a")).toMatchObject({ fieldCosmetic: "field-rain" });

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("deck-cosmetic-table")).toBeNull();
  });
});

describe("the deck's history (R2100000)", () => {
  it("R2100000 the History panel lists the versions newest first with the cards in and out, and Restore makes one current", async () => {
    const [kept1 = "", kept2 = "", dropped = "", added1 = "", added2 = ""] = ONE;
    const first = [kept1, kept2, dropped];
    const second = [kept1, kept2, added1, added2];
    const savedAt = Date.UTC(2026, 9, 27, 12);
    const server = fakeDeckServer();
    server.versions.set("a", [
      { id: "v2", name: "Aggro", cards: second, savedAt },
      { id: "v1", name: "Opener", cards: first, savedAt: Date.UTC(2026, 9, 26, 9, 30) },
    ]);
    mount({ server, decks: [savedDeck("a", "Aggro", second, 1)], initialOpen: { kind: "deck", id: "a" } });
    expect(screen.queryByTestId(DECK_HISTORY_PANEL)).toBeNull();

    fireEvent.click(screen.getByTestId(DECK_HISTORY_BUTTON));
    const panel = screen.getByTestId(DECK_HISTORY_PANEL);
    expect(panel).toHaveAttribute("role", "dialog");
    const rows = await within(panel).findAllByTestId(DECK_HISTORY_ROW);
    expect(rows.map((row) => row.getAttribute("data-version"))).toEqual(["v2", "v1"]);
    const [newest, oldest] = rows;
    expect(newest).toHaveTextContent(formatDateTime(savedAt, { dateStyle: "medium", timeStyle: "short" }));
    expect(newest).toHaveTextContent("Latest save");
    expect(newest).toHaveTextContent("Renamed from “Opener”");
    expect(newest).toHaveTextContent(`In: ${nameOf(added1)} and ${nameOf(added2)}`);
    expect(newest).toHaveTextContent(`Out: ${nameOf(dropped)}`);
    expect(within(newest as HTMLElement).queryByRole("button"), "the newest is the deck as saved").toBeNull();
    expect(oldest).toHaveTextContent("The oldest version kept");

    fireEvent.click(screen.getByTestId(deckHistoryRestoreId("v1")));
    await waitFor(() => {
      expect(within(panel).getAllByTestId(DECK_HISTORY_ROW)).toHaveLength(3);
    });
    expect(within(panel).getAllByTestId(DECK_HISTORY_ROW).map((row) => row.getAttribute("data-version"))).toEqual(["v1-restored", "v2", "v1"]);
    expect(within(panel).getAllByTestId(DECK_HISTORY_ROW)[0]).toHaveTextContent("Renamed from “Aggro”");
    expect(server.decks.get("a")).toMatchObject({ name: "Opener", cards: first });
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Opener");
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", String(first.length));
    expect(screen.getByTestId(deckCardId(dropped))).toBeInTheDocument();
    expect(screen.queryByTestId(deckCardId(added1))).toBeNull();
    expect(syncState()).toBe("saved");

    // Escape closes the panel, and each open reads the list again.
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId(DECK_HISTORY_PANEL)).toBeNull();
    fireEvent.click(screen.getByTestId(DECK_HISTORY_BUTTON));
    expect(await within(screen.getByTestId(DECK_HISTORY_PANEL)).findAllByTestId(DECK_HISTORY_ROW)).toHaveLength(3);
    expect(server.calls.filter((call) => call.op === "getDeckVersions")).toHaveLength(2);
  });

  it("R2100000 a restore ends the open deck's Undo and Redo (#2099), whose steps were taken on the deck it replaced", async () => {
    const server = fakeDeckServer();
    server.versions.set("a", [
      { id: "v2", name: "Aggro", cards: [], savedAt: Date.UTC(2026, 9, 27, 12) },
      { id: "v1", name: "Opener", cards: ONE, savedAt: Date.UTC(2026, 9, 26, 9, 30) },
    ]);
    mount({ server, decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(1))));
    expect(screen.getByTestId(DECK_UNDO)).toBeEnabled();

    fireEvent.click(screen.getByTestId(DECK_HISTORY_BUTTON));
    await within(screen.getByTestId(DECK_HISTORY_PANEL)).findAllByTestId(DECK_HISTORY_ROW);
    fireEvent.click(screen.getByTestId(deckHistoryRestoreId("v1")));
    await waitFor(() => {
      expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Opener");
    });
    expect(screen.getByTestId(DECK_UNDO)).toBeDisabled();
    expect(screen.getByTestId(DECK_REDO)).toBeDisabled();
    // The keys take nothing back either: the deck stays as restored.
    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", String(ONE.length));
    expect(syncState()).toBe("saved");
  });
});

describe("issue #2099 undo and redo", () => {
  function undoButton(): HTMLElement {
    return screen.getByTestId(DECK_UNDO);
  }

  function redoButton(): HTMLElement {
    return screen.getByTestId(DECK_REDO);
  }

  function count(): string {
    return screen.getByTestId(DECK_COUNT).getAttribute("data-count") ?? "";
  }

  it("issue #2099 Undo takes back an add and a removal and Redo puts them back; each button is off with nothing to move, and an undo saves like any edit", async () => {
    const { server, tick } = mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    expect(undoButton()).toBeDisabled();
    expect(redoButton()).toBeDisabled();
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(1))));
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(2))));
    fireEvent.click(screen.getByTestId(deckCardId(fixtureCardId(1))));
    expect(count()).toBe("1");
    fireEvent.click(undoButton());
    expect(count()).toBe("2");
    expect(screen.getByTestId(deckCardId(fixtureCardId(1)))).toBeInTheDocument();
    expect(redoButton()).toBeEnabled();
    fireEvent.click(undoButton());
    fireEvent.click(undoButton());
    expect(count()).toBe("0");
    expect(undoButton()).toBeDisabled();
    fireEvent.click(redoButton());
    expect(count()).toBe("1");
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.cards).toEqual([fixtureCardId(1)]);
  });

  it("R2126000 Undo and Redo name their keys in their tooltips and aria-keyshortcuts", () => {
    mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    expect(undoButton().getAttribute("title")).toBe("Undo (Ctrl+Z)");
    expect(undoButton().getAttribute("aria-keyshortcuts")).toBe("Control+Z Meta+Z");
    expect(redoButton().getAttribute("title")).toBe("Redo (Ctrl+Shift+Z)");
    expect(redoButton().getAttribute("aria-keyshortcuts")).toBe("Control+Shift+Z Meta+Shift+Z");
  });

  it("issue #2099 a rename is one step however many keys it takes", async () => {
    const { server, tick } = mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    const input = screen.getByTestId(DECK_NAME_INPUT);
    fireEvent.change(input, { target: { value: "Aggr" } });
    fireEvent.change(input, { target: { value: "Aggro Rush" } });
    fireEvent.blur(input);
    fireEvent.change(input, { target: { value: "Tempo" } });
    fireEvent.blur(input);
    fireEvent.click(undoButton());
    expect(input).toHaveValue("Aggro Rush");
    fireEvent.click(undoButton());
    expect(input).toHaveValue("Aggro");
    expect(undoButton()).toBeDisabled();
    fireEvent.click(redoButton());
    expect(input).toHaveValue("Aggro Rush");
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("a")?.name).toBe("Aggro Rush");
  });

  it("issue #2099 a portrait pick and a cosmetic pick are a step each", () => {
    mount({ decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    const first = screen.getByTestId("portrait-current").getAttribute("title") ?? "";
    fireEvent.click(screen.getByTestId("portrait-current"));
    fireEvent.click(screen.getByTestId("portrait-pick-shredder"));
    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    fireEvent.change(screen.getByTestId("deck-cosmetic-field"), { target: { value: "field-rain" } });
    fireEvent.click(undoButton());
    expect(screen.getByTestId("deck-cosmetic-field")).toHaveValue("");
    expect(screen.getByTestId("portrait-current")).toHaveAttribute("title", `Hero portrait: ${PORTRAIT_DEFS.shredder.def.name}`);
    fireEvent.click(undoButton());
    expect(screen.getByTestId("portrait-current")).toHaveAttribute("title", first);
    fireEvent.click(redoButton());
    fireEvent.click(redoButton());
    expect(screen.getByTestId("portrait-current")).toHaveAttribute("title", `Hero portrait: ${PORTRAIT_DEFS.shredder.def.name}`);
    expect(screen.getByTestId("deck-cosmetic-field")).toHaveValue("field-rain");
  });

  it("issue #2099 a new edit after Undo empties Redo", () => {
    mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(1))));
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(2))));
    fireEvent.click(undoButton());
    expect(redoButton()).toBeEnabled();
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(3))));
    expect(redoButton()).toBeDisabled();
    fireEvent.click(undoButton());
    expect(screen.queryByTestId(deckCardId(fixtureCardId(3)))).toBeNull();
    expect(count()).toBe("1");
  });

  it("issue #2099 Ctrl/Cmd+Z undoes and Ctrl/Cmd+Shift+Z redoes, and neither does anything in a text field", () => {
    mount({ decks: [savedDeck("a", "Aggro", [], 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(1))));
    expect(fireEvent.keyDown(document.body, { key: "z", ctrlKey: true })).toBe(false);
    expect(count()).toBe("0");
    expect(fireEvent.keyDown(document.body, { key: "Z", ctrlKey: true, shiftKey: true })).toBe(false);
    expect(count()).toBe("1");
    expect(fireEvent.keyDown(document.body, { key: "z", metaKey: true })).toBe(false);
    expect(count()).toBe("0");
    expect(fireEvent.keyDown(document.body, { key: "z", metaKey: true, shiftKey: true })).toBe(false);
    expect(count()).toBe("1");
    expect(fireEvent.keyDown(screen.getByTestId(DECK_NAME_INPUT), { key: "z", ctrlKey: true })).toBe(true);
    expect(fireEvent.keyDown(screen.getByTestId(DB_SEARCH), { key: "z", metaKey: true })).toBe(true);
    expect(count()).toBe("1");
    fireEvent.click(undoButton());
    expect(count()).toBe("0");
    expect(fireEvent.keyDown(screen.getByTestId(DB_SEARCH), { key: "z", ctrlKey: true, shiftKey: true })).toBe(true);
    expect(count()).toBe("0");
  });

  it("issue #2099 opening another deck starts a fresh history, and so does closing the editor", () => {
    mount({
      decks: [savedDeck("a", "Aggro", [], 1), savedDeck("b", "Tempo", [], 2)],
      trios: [savedTrio("t", "Ladder", [null, null, null], 3)],
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(1))));
    expect(undoButton()).toBeEnabled();
    fireEvent.click(screen.getByTestId(deckRowId("b")));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "b");
    expect(undoButton()).toBeDisabled();
    expect(redoButton()).toBeDisabled();
    fireEvent.keyDown(document.body, { key: "z", ctrlKey: true });
    expect(screen.getByTestId(deckRowId("a"))).toHaveAttribute("data-count", "1");
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(2))));
    fireEvent.click(screen.getByTestId(trioRowId("t")));
    expect(screen.queryByTestId(DECK_UNDO)).toBeNull();
    fireEvent.click(screen.getByTestId(deckRowId("b")));
    expect(undoButton()).toBeDisabled();
    fireEvent.click(screen.getByTestId(addPoolId(fixtureCardId(3))));
    expect(undoButton()).toBeEnabled();
    fireEvent.click(screen.getByTestId(WORKSHOP_BACK));
    expect(undoButton()).toBeDisabled();
    fireEvent.click(screen.getByTestId(deckRowId("b")));
    expect(undoButton()).toBeDisabled();
    expect(screen.getByTestId(deckRowId("b"))).toHaveAttribute("data-count", "2");
  });
});

describe("duplicating a deck (#2096)", () => {
  it("#2096 Duplicate in the editor makes a deck with the same cards, portrait and picks, opens it and saves it", async () => {
    const { server, tick } = mount({
      decks: [
        {
          ...savedDeck("a", "Aggro", ONE, 1),
          portrait: "gary",
          tableCosmetic: "table-felt",
          fieldCosmetic: "field-rain",
          cardBackCosmetic: "card-back-lacquer",
        },
      ],
      initialOpen: { kind: "deck", id: "a" },
    });
    expect(screen.getByTestId(DECK_DUPLICATE)).toBeEnabled();
    expect(screen.queryByTestId(DECK_DUPLICATE_REASON)).toBeNull();
    fireEvent.click(screen.getByTestId(DECK_DUPLICATE));
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Aggro (copy)");
    expect(screen.getByTestId(deckRowId("new-1"))).toHaveAttribute("aria-current", "true");
    expect(screen.getByTestId(deckRowId("new-1"))).toHaveAttribute("data-count", String(DECK_SIZE));
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.decks.get("new-1")).toMatchObject({
      name: "Aggro (copy)",
      cards: [...ONE],
      portrait: "gary",
      tableCosmetic: "table-felt",
      fieldCosmetic: "field-rain",
      cardBackCosmetic: "card-back-lacquer",
    });
  });

  it("#2096 a row's Duplicate copies that deck, not the open one, and opens the copy from the list", () => {
    mount({
      decks: [savedDeck("a", "Aggro", ONE.slice(0, 3), 1), savedDeck("b", "Brawl", TWO.slice(0, 5), 2)],
    });
    expect(screen.getByTestId(WORKSHOP)).toHaveAttribute("data-view", "list");
    expect(screen.getByRole("button", { name: "Duplicate Brawl" })).toBe(screen.getByTestId(deckDuplicateId("b")));
    fireEvent.click(screen.getByTestId(deckDuplicateId("b")));
    expect(screen.getByTestId(WORKSHOP)).toHaveAttribute("data-view", "editor");
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Brawl (copy)");
    expect(screen.getByTestId(DECK_COUNT)).toHaveAttribute("data-count", "5");
  });

  it("#2096 the copy's name takes (copy 2), then (copy 3), while each is taken", () => {
    mount({
      decks: [savedDeck("a", "Aggro", [], 1), savedDeck("b", "Aggro (copy)", [], 2)],
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.click(screen.getByTestId(deckDuplicateId("a")));
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Aggro (copy 2)");
    fireEvent.click(screen.getByTestId(deckDuplicateId("a")));
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Aggro (copy 3)");
    expect(screen.getByTestId(DECK_CAP)).toHaveTextContent(`4/${String(MAX_SAVED_DECKS)}`);
  });

  it("#2096 the original and the trio slots naming it are unchanged", async () => {
    const { server, tick } = mount({
      decks: [savedDeck("a", "Aggro", ONE, 1), savedDeck("b", "Brawl", TWO, 2)],
      trios: [savedTrio("t", "Ladder", ["a", "b", null], 3)],
      initialOpen: { kind: "deck", id: "a" },
    });
    fireEvent.click(screen.getByTestId(DECK_DUPLICATE));
    await tick(DECK_AUTOSAVE_DEBOUNCE_MS);
    expect(server.calls.map((call) => `${call.op} ${call.id}`)).toEqual(["putDeck new-1"]);
    expect(server.decks.get("a")).toMatchObject({ name: "Aggro", cards: [...ONE] });
    expect(server.trios.get("t")?.deckIds).toEqual(["a", "b", null]);
    fireEvent.click(screen.getByTestId(trioRowId("t")));
    expect(screen.getByTestId(trioSlotId(1))).toHaveValue("a");
    expect(screen.getByTestId(trioSlotId(2))).toHaveValue("b");
    expect(screen.getByTestId(trioSlotId(3))).toHaveValue("");
  });

  it("#2096 at the deck limit Duplicate is off in the list and the editor, and says why", () => {
    const decks = Array.from({ length: MAX_SAVED_DECKS }, (_unused, index) => savedDeck(`d${String(index)}`, `Deck ${String(index)}`, [], index));
    mount({ decks, initialOpen: { kind: "deck", id: "d0" } });
    expect(screen.getByTestId(DECK_DUPLICATE)).toBeDisabled();
    expect(screen.getByTestId(DECK_DUPLICATE_REASON)).toHaveTextContent(
      `You have ${String(MAX_SAVED_DECKS)} decks, the most you can keep. Delete one to duplicate this.`,
    );
    expect(screen.getByTestId(DECK_DUPLICATE)).toHaveAttribute("aria-describedby", screen.getByTestId(DECK_DUPLICATE_REASON).id);
    const rowDuplicate = screen.getByTestId(deckDuplicateId("d0"));
    expect(rowDuplicate).toBeDisabled();
    expect(rowDuplicate).toHaveAttribute("aria-describedby", screen.getByTestId(DECK_CAP_REASON).id);
    fireEvent.click(screen.getByTestId(DECK_DUPLICATE));
    fireEvent.click(rowDuplicate);
    expect(screen.getByTestId(DECK_CAP)).toHaveTextContent(`${String(MAX_SAVED_DECKS)}/${String(MAX_SAVED_DECKS)}`);
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "d0");
  });
});

describe("R2118000 New deck from the command menu", () => {
  afterEach(() => {
    at("/");
  });

  it("R2118000 /decks?new=deck makes and opens one new deck and drops the query", () => {
    at("/decks?new=deck");
    mount();
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
    expect(screen.getByTestId(DECK_NAME_INPUT)).toHaveValue("Deck 1");
    expect(window.location.search).toBe("");
    expect(screen.queryByTestId(deckRowId("new-2"))).toBeNull();
  });

  it("R2118000 a New deck move on an open workshop makes one in place", () => {
    mount();
    act(() => {
      navigate(newDeckPath());
    });
    expect(screen.getByTestId(DECK_EDITOR)).toHaveAttribute("data-deck", "new-1");
  });
});

describe("R2127000 Back closes the deck builder's panels on /decks", () => {
  afterEach(() => {
    at("/");
  });

  /** The deck builder's address with the page before it under it, as a player arrives on it. */
  function arrive(): void {
    at("/play");
    window.history.pushState(null, "", "/decks");
  }

  it("R2127000 one Back closes the deck history and the page stays on /decks; after Escape, one Back leaves", async () => {
    arrive();
    const server = fakeDeckServer();
    server.versions.set("a", [{ id: "v1", name: "Aggro", cards: ONE, savedAt: Date.UTC(2026, 9, 27, 12) }]);
    mount({ server, decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(DECK_HISTORY_BUTTON));
    await within(screen.getByTestId(DECK_HISTORY_PANEL)).findAllByTestId(DECK_HISTORY_ROW);

    await pressBack();
    expect(screen.queryByTestId(DECK_HISTORY_PANEL)).toBeNull();
    expect(screen.getByTestId(DECK_HISTORY_BUTTON)).toHaveFocus();
    expect(window.location.pathname).toBe("/decks");
    expect(screen.getByTestId(DECK_EDITOR)).toBeInTheDocument();

    // Escape closes it once and leaves no layer: the next Back is the page's.
    fireEvent.click(screen.getByTestId(DECK_HISTORY_BUTTON));
    await within(screen.getByTestId(DECK_HISTORY_PANEL)).findAllByTestId(DECK_HISTORY_ROW);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId(DECK_HISTORY_PANEL)).toBeNull();
    expect(window.location.pathname).toBe("/decks");
    await pressBack();
    expect(window.location.pathname).toBe("/play");
  });

  it("R2127000 one Back closes the deck cosmetics and the page stays on /decks; after Escape, one Back leaves", async () => {
    arrive();
    mount({ decks: [savedDeck("a", "Aggro", ONE, 1)], initialOpen: { kind: "deck", id: "a" } });
    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    expect(screen.getByTestId("deck-cosmetic-table")).toBeInTheDocument();

    await pressBack();
    expect(screen.queryByTestId("deck-cosmetic-table")).toBeNull();
    expect(screen.getByTestId(DECK_COSMETICS_BUTTON)).toHaveFocus();
    expect(window.location.pathname).toBe("/decks");
    expect(screen.getByTestId(DECK_EDITOR)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(DECK_COSMETICS_BUTTON));
    expect(screen.getByTestId("deck-cosmetic-table")).toBeInTheDocument();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("deck-cosmetic-table")).toBeNull();
    expect(window.location.pathname).toBe("/decks");
    await pressBack();
    expect(window.location.pathname).toBe("/play");
  });
});
