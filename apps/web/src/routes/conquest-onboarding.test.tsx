// Conquest first-series walkthrough and onboarding tests (SPEC §9.5, §9.10; R330–R334, R604).

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED } from "@jackioh/server-config";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";
import { TRIO_L4_HINT } from "../game/deckbuilder/testids.ts";
import TrioEditor from "../game/deckbuilder/TrioEditor.tsx";
import type { TrioItem, WorkshopLimits } from "../game/deckbuilder/sync.ts";
import { getSeries, type SeriesView } from "../net/api.ts";
import {
  CONQUEST_EXPLAINER_STORAGE_KEY,
  CONQUEST_MESSAGES,
  conquestText,
} from "./conquestCopy.ts";
import { conquestExplainerTestid } from "./ConquestExplainer.tsx";
import SeriesRoute, { ratingWords, seriesTestid } from "./series.tsx";
import SeriesPicker, { seriesPickerTestid } from "./SeriesPicker.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getSeries: vi.fn(), pickSeriesDeck: vi.fn(), forfeitSeries: vi.fn() };
});
vi.mock("../net/navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

const here = dirname(fileURLToPath(import.meta.url));
const VALIDATOR_PATH = resolve(here, "../../../../crates/engine/src/validator.rs");
const RUST_TOKENS = /\/\/[^\n]*|\/\*[\s\S]*?\*\/|'(?:[^'\\\n]|\\.)'|"((?:[^"\\]|\\.)*)"/g;

function getValidatorFragments(): string[] {
  const source = readFileSync(VALIDATOR_PATH, "utf8");
  const unescaped = (t: string): string => t.replace(/\\(["'\\])/g, "$1");
  const literals = [...source.matchAll(RUST_TOKENS)].flatMap((match) =>
    match[1] === undefined ? [] : [unescaped(match[1])],
  );
  const fragments = new Set<string>();
  for (const literal of literals) {
    for (const piece of literal.split(/\{[^{}]*\}/)) {
      const trimmed = piece.trim();
      const prose = trimmed.replace(/^[\s;:,."()]+|[\s;:,."()]+$/g, "");
      if (prose.length < 9) continue;
      if (!/[A-Za-z]\s[A-Za-z]/.test(prose)) continue;
      fragments.add(trimmed);
    }
  }
  return [...fragments].sort();
}

const TOKEN = "token-1";
const SERIES_ID = "series-1";
const SERVER_NOW = 1_000_000;

function sampleSeriesView(overrides: Partial<SeriesView> = {}): SeriesView {
  return {
    id: SERIES_ID,
    status: "picking",
    gameNo: 1,
    winsNeeded: SERIES_WINS_NEEDED,
    maxGames: SERIES_MAX_GAMES,
    pickDeadline: SERVER_NOW + SERIES_PICK_SECONDS * 1000,
    now: SERVER_NOW,
    currentMatchId: null,
    ranked: true,
    you: {
      seat: "p1",
      wins: 0,
      trioName: "Main trio",
      decks: [
        { slot: 0, name: "Aggro", cards: ["core-001"], won: false, games: 0 },
        { slot: 1, name: "Control", cards: ["core-002"], won: false, games: 0 },
        { slot: 2, name: "Ramp", cards: ["core-003"], won: false, games: 0 },
      ],
      pick: null,
      autoPick: false,
    },
    opponent: {
      wins: 0,
      decks: [
        { slot: 0, won: false },
        { slot: 1, won: false },
        { slot: 2, won: false },
      ],
      picked: false,
    },
    games: [],
    result: null,
    ...overrides,
  };
}

describe("Conquest first-series walkthrough and onboarding", () => {
  beforeEach(() => {
    window.localStorage.clear();
    vi.mocked(getSeries).mockResolvedValue(sampleSeriesView());
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it("explainer displays all 6 conquest rules on first visit", async () => {
    render(<SeriesRoute seriesId={SERIES_ID} token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(seriesTestid.explainer)).toBeInTheDocument();
    });

    const explainer = screen.getByTestId(seriesTestid.explainer);
    expect(explainer).toHaveTextContent(conquestText("explainerTitle"));

    // 1. wins needed (R330)
    const winsItem = screen.getByTestId(conquestExplainerTestid.item("wins"));
    expect(winsItem).toHaveTextContent(conquestText("winsNeeded"));
    expect(winsItem).toHaveTextContent(`${String(SERIES_WINS_NEEDED)} decks`);

    // 2. lock-on-win (R330)
    const locksItem = screen.getByTestId(conquestExplainerTestid.item("locks"));
    expect(locksItem).toHaveTextContent(conquestText("lockOnWin"));

    // 3. sealed simultaneous picks (R331)
    const sealedItem = screen.getByTestId(conquestExplainerTestid.item("sealed"));
    expect(sealedItem).toHaveTextContent(conquestText("sealedPicks"));

    // 4. draws lock nothing (R334)
    const drawsItem = screen.getByTestId(conquestExplainerTestid.item("draws"));
    expect(drawsItem).toHaveTextContent(conquestText("draws"));
    expect(drawsItem).toHaveTextContent(`${String(SERIES_MAX_GAMES)} games`);

    // 5. concede vs forfeit (R334)
    const concedeItem = screen.getByTestId(conquestExplainerTestid.item("concede"));
    expect(concedeItem).toHaveTextContent(conquestText("concedeVsForfeit"));

    // 6. rating moves once (R604)
    const ratingItem = screen.getByTestId(conquestExplainerTestid.item("rating"));
    expect(ratingItem).toHaveTextContent(conquestText("ratingMovesOnce"));
  });

  it("dismissing the explainer saves to localStorage and hides the explainer", async () => {
    render(<SeriesRoute seriesId={SERIES_ID} token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(seriesTestid.explainer)).toBeInTheDocument();
    });

    expect(window.localStorage.getItem(CONQUEST_EXPLAINER_STORAGE_KEY)).toBeNull();

    const dismissButton = screen.getByTestId(seriesTestid.explainerDismiss);
    fireEvent.click(dismissButton);

    expect(window.localStorage.getItem(CONQUEST_EXPLAINER_STORAGE_KEY)).toBe("true");
    expect(screen.queryByTestId(seriesTestid.explainer)).toBeNull();
    expect(screen.getByTestId(seriesTestid.explainerToggle)).toBeInTheDocument();
  });

  it("subsequent visits with dismissed localStorage start with explainer closed and toggle available", async () => {
    window.localStorage.setItem(CONQUEST_EXPLAINER_STORAGE_KEY, "true");

    render(<SeriesRoute seriesId={SERIES_ID} token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(seriesTestid.screen)).toBeInTheDocument();
    });

    // Explainer is closed initially
    expect(screen.queryByTestId(seriesTestid.explainer)).toBeNull();
    const toggleButton = screen.getByTestId(seriesTestid.explainerToggle);
    expect(toggleButton).toHaveTextContent(conquestText("toggleOpen"));

    // Clicking toggle opens the explainer
    fireEvent.click(toggleButton);
    expect(screen.getByTestId(seriesTestid.explainer)).toBeInTheDocument();
  });

  it("lock-in button in SeriesPicker displays inline reminder for sealed pick and lock-on-win", () => {
    const view = sampleSeriesView();
    render(
      <SeriesPicker
        view={view}
        secondsLeft={SERIES_PICK_SECONDS}
        busy={false}
        onLockIn={() => {}}
      />,
    );

    const reminder = screen.getByTestId(seriesPickerTestid.lockInReminder);
    expect(reminder).toBeInTheDocument();
    expect(reminder).toHaveTextContent(conquestText("lockInReminder"));
    expect(reminder).toHaveTextContent("Locking in is final and sealed");
    expect(reminder).toHaveTextContent("A win locks this deck");
  });

  it("series result shows inline rating reminder on ranked series end", async () => {
    const finishedRankedView = sampleSeriesView({
      status: "over",
      result: {
        outcome: "win",
        endReason: "decided",
        ranked: true,
      },
    });
    vi.mocked(getSeries).mockResolvedValue(finishedRankedView);

    render(<SeriesRoute seriesId={SERIES_ID} token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(seriesTestid.result)).toBeInTheDocument();
    });

    const endReminder = screen.getByTestId(seriesTestid.endReminder);
    expect(endReminder).toBeInTheDocument();
    expect(endReminder).toHaveTextContent(conquestText("seriesEndRatingReminder"));
    // The reminder adds to the rating line above it and never repeats it.
    const ratingLine = ratingWords({ outcome: "win", endReason: "decided", ranked: true });
    expect(endReminder).not.toHaveTextContent(ratingLine);
    expect((screen.getByTestId(seriesTestid.result).textContent ?? "").split(ratingLine)).toHaveLength(2);
  });

  it("series result does not show inline rating reminder when unrated", async () => {
    const unratedView = sampleSeriesView({
      status: "over",
      result: {
        outcome: "win",
        endReason: "decided",
        ranked: false,
      },
    });
    vi.mocked(getSeries).mockResolvedValue(unratedView);

    render(<SeriesRoute seriesId={SERIES_ID} token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(seriesTestid.result)).toBeInTheDocument();
    });
    expect(screen.queryByTestId(seriesTestid.endReminder)).toBeNull();
  });

  it("series result does not show inline rating reminder when abandoned", async () => {
    const abandonedView = sampleSeriesView({
      status: "over",
      result: {
        outcome: "abandoned",
        endReason: "abandoned",
        ranked: true,
      },
    });
    vi.mocked(getSeries).mockResolvedValue(abandonedView);

    render(<SeriesRoute seriesId={SERIES_ID} token={TOKEN} />);
    await waitFor(() => {
      expect(screen.getByTestId(seriesTestid.result)).toBeInTheDocument();
    });
    expect(screen.queryByTestId(seriesTestid.endReminder)).toBeNull();
  });

  it("trio editor surfaces L4 shared-card rule before queue", () => {
    const dummyTrio: TrioItem = {
      id: "trio-1",
      name: "Main Trio",
      deckIds: [null, null, null],
      createdAt: 1,
      updatedAt: 1,
    };
    const dummyLimits: WorkshopLimits = {
      nameLength: 24,
      decks: 10,
      trios: 5,
    };
    const dummyCatalog: CatalogSnapshot = {
      version: "1.0.0",
      cards: {},
    };
    const dummyCollection: Collection = {};

    render(
      <TrioEditor
        trio={dummyTrio}
        decks={[]}
        catalog={dummyCatalog}
        collection={dummyCollection}
        limits={dummyLimits}
        refusal={null}
        onBack={() => {}}
        onRename={() => {}}
        onSlots={() => {}}
        onOpenDeck={() => {}}
        onDelete={() => {}}
      />,
    );

    const l4Hint = screen.getByTestId(TRIO_L4_HINT);
    expect(l4Hint).toBeInTheDocument();
    expect(l4Hint).toHaveTextContent(conquestText("trioL4Hint"));
    expect(l4Hint).toHaveTextContent("Rule L4");
  });

  it("conquest copy text passes validator fragment exclusion check", () => {
    const validatorFragments = getValidatorFragments();
    expect(validatorFragments.length).toBeGreaterThanOrEqual(12);

    for (const [key, message] of Object.entries(CONQUEST_MESSAGES)) {
      for (const fragment of validatorFragments) {
        expect(
          message.includes(fragment),
          `CONQUEST_MESSAGES.${key} contains validator fragment: "${fragment}"`,
        ).toBe(false);
      }
    }
  });
});
