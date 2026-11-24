import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import { DECK_SIZE } from "@jackioh/engine/config";

import { PRACTICE_PRESETS, savedDeckRefusal } from "../../practice/decks.ts";
import type { SavedDecks } from "../../practice/PracticeSetup.tsx";
import { assaultTestid } from "../testids.ts";
import { DAY, bindingFixtures } from "../test/fixtures.ts";
import { DeckPicker, dayDeckChoice, type AssaultDeckChoice } from "./DeckPicker.tsx";

vi.mock("../../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  ...(await import("../test/fixtures.ts")).bindingFixtures,
}));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  bindingFixtures.assaultDayDeck.mockClear();
});

it("a refused saved deck is disabled with its reason, and the chosen deck shows its cards and curve", () => {
  const refused = Array.from({ length: DECK_SIZE }, () => "core-001");
  const refusedDeck = { name: "Needs work", cards: refused };
  const saved: SavedDecks = { kind: "ready", decks: [refusedDeck] };
  const onChange = vi.fn();
  const choice = dayDeckChoice(DAY.date);
  render(<DeckPicker date={DAY.date} saved={saved} choice={choice} onChange={onChange} />);

  const option = screen.getByTestId(assaultTestid.option("saved", 0));
  expect(option).toBeDisabled();
  expect(option.closest("label")).toHaveTextContent("Needs work");
  expect(option.closest("label")).toHaveTextContent(savedDeckRefusal(refusedDeck) ?? "missing refusal");
  expect(screen.getByTestId(assaultTestid.curve)).toHaveAttribute("data-testid", assaultTestid.curve);
  expect(screen.getByRole("list", { name: /Today's Assault deck/ })).toHaveTextContent("Big D-fender");
});

it("the three named presets and the day's deck are offered", () => {
  const choice: AssaultDeckChoice = dayDeckChoice(DAY.date);
  const onChange = vi.fn();
  render(<DeckPicker date={DAY.date} saved={{ kind: "anonymous" }} choice={choice} onChange={onChange} />);

  const presets = screen.getByTestId(assaultTestid.deckPreset);
  for (const preset of PRACTICE_PRESETS) expect(within(presets).getByText(preset.name)).toBeInTheDocument();
  expect(screen.getByTestId(assaultTestid.deckDay)).toHaveTextContent("Today's Assault deck");
  expect(screen.getByTestId(assaultTestid.option("day", 0))).toBeChecked();

  fireEvent.click(screen.getByTestId(assaultTestid.option("preset", 0)));
  expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ source: "preset" }));
});
