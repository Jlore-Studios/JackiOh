// The practice deck preview's card list (practice/DeckPreview.tsx): #2115, R2115000, a skeleton of rows
// while the worker's catalog loads, and a plain sentence when it could not be read.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { registeredCatalog } from "@jackioh/engine";

import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { expectSkeleton } from "../test/skeleton.ts";
import { DeckPreview } from "./DeckPreview.tsx";
import { practiceTestid } from "./testids.ts";

afterEach(() => {
  cleanup();
});

describe("#2115 the deck preview's card list", () => {
  it("#2115 the card list waits behind rows, then lists the deck", () => {
    const defs = registeredCatalog();
    const id = Object.keys(defs)[0];
    expect(id).toBeDefined();
    if (id === undefined) return;

    const { rerender } = render(<DeckPreview title="Test" identity="Test" cards={[id]} defs={null} defsFailed={false} />);
    expectSkeleton(screen.getByText("Loading the card list…"), "rows");
    expect(screen.queryByTestId(practiceTestid.deckCard(id))).toBeNull();

    rerender(<DeckPreview title="Test" identity="Test" cards={[id]} defs={defs} defsFailed={false} />);
    expect(screen.getByTestId(practiceTestid.deckCard(id))).toBeInTheDocument();
    expect(screen.queryByText("Loading the card list…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
  });

  it("#2115 a failed card list stays a sentence", () => {
    render(<DeckPreview title="Test" identity="Test" cards={["core-001"]} defs={null} defsFailed onRetry={() => undefined} />);
    expect(screen.getByText("The card list could not be loaded; the deck still plays.")).toBeInTheDocument();
    expect(screen.getByTestId(practiceTestid.defsRetry)).toBeInTheDocument();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
    expect(screen.queryByText("Loading the card list…")).toBeNull();
  });
});
