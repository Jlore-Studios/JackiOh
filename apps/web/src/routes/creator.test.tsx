// R1484000, #1484: the card creator asks before leaving a card with changes not yet saved.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CustomCard } from "@jackioh/shared";

import { type CreatorApi } from "../creator/Creator.tsx";
import { creatorTestid } from "../creator/testids.ts";
import { type CustomCardInput } from "../net/customCards.ts";
import { paths } from "../net/navigate.ts";
import CreatorRoute from "./creator.tsx";
import { navTestid } from "./nav.tsx";

afterEach(cleanup);

beforeEach(() => {
  window.history.replaceState(null, "", paths.creator);
});

const UNSAVED = "This card has changes you haven't saved.";

/** A fake account's store, as the server keeps it: ids minted at create, a version moved by every update. */
function fakeApi(): CreatorApi {
  let minted = 0;
  return {
    list: vi.fn(() => Promise.resolve({ cards: [], limits: { cards: 50 } })),
    create: vi.fn((_token: string, input: CustomCardInput) => {
      minted += 1;
      const card: CustomCard = {
        id: `custom-${String(minted).padStart(32, "0")}`,
        version: 1,
        frame: input.frame,
        source: input.source,
        ...(input.builder === undefined ? {} : { builder: input.builder }),
      };
      return Promise.resolve({ card });
    }),
    update: vi.fn(() => Promise.reject(new Error("not used"))),
    remove: vi.fn(() => Promise.resolve({ deleted: false })),
    check: vi.fn(() => Promise.resolve({ script: [] })),
  };
}

function unloadIsCancelled(): boolean {
  const event = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(event);
  return event.defaultPrevented;
}

async function openRoute(): Promise<ReturnType<typeof render>> {
  const view = render(<CreatorRoute token="token-1" api={fakeApi()} />);
  await screen.findByText("0 of 50 cards");
  return view;
}

function typeName(value: string): void {
  fireEvent.change(screen.getByTestId(creatorTestid.name), { target: { value } });
}

describe("R1484000 the creator asks before leaving a card with changes not yet saved", () => {
  it("R1484000 Back on a new card with a name typed stays and shows the unsaved-changes note; Keep editing keeps the draft, Discard them leaves", async () => {
    await openRoute();
    typeName("Half made");
    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.creator);
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(screen.queryByText(UNSAVED)).toBeNull();
    expect((screen.getByTestId(creatorTestid.name) as HTMLInputElement).value).toBe("Half made");
    expect(window.location.pathname).toBe(paths.creator);

    fireEvent.click(screen.getByTestId(navTestid.back));
    fireEvent.click(screen.getByRole("button", { name: "Discard them" }));
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R1484000 Back on an untouched new card leaves at once", async () => {
    await openRoute();
    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.collection);
    expect(screen.queryByText(UNSAVED)).toBeNull();
  });

  it("R1579000 the creator's Back leads to the Collection hub", async () => {
    await openRoute();
    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.collection);
    expect(window.location.search).toBe("");
  });

  it("R1484000 with unsaved work a reload or a closed tab asks first, and not on an untouched new card", async () => {
    await openRoute();
    expect(unloadIsCancelled()).toBe(false);
    typeName("Half made");
    expect(unloadIsCancelled()).toBe(true);
  });

  it("R1484000 after a save Back leaves at once and a reload is not held, and a note Back opened goes away", async () => {
    await openRoute();
    typeName("Saved card");
    await waitFor(() => expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(false));
    expect(unloadIsCancelled()).toBe(true);

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(creatorTestid.save));
    await screen.findByTestId(creatorTestid.saved);
    // The note Back opened clears on the report the save's own render carries, a render later.
    await waitFor(() => expect(screen.queryByText(UNSAVED)).toBeNull());
    expect(unloadIsCancelled()).toBe(false);

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R1484000 the beforeunload listener is gone after the page unmounts", async () => {
    const view = await openRoute();
    typeName("Half made");
    expect(unloadIsCancelled()).toBe(true);
    view.unmount();
    expect(unloadIsCancelled()).toBe(false);
  });
});
