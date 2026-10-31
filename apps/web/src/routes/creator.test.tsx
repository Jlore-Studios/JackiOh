// R1484000, #1484: the card creator asks before leaving unsaved changes.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CustomCard } from "@jackioh/shared";

import { type CreatorApi } from "../creator/Creator.tsx";
import { creatorTestid } from "../creator/testids.ts";
import { type CustomCardInput } from "../net/customCards.ts";
import { navigate, paths, tryPath, usePathname } from "../net/navigate.ts";
import CreatorRoute from "./creator.tsx";
import { navTestid } from "./nav.tsx";

afterEach(cleanup);

beforeEach(() => {
  window.history.replaceState(null, "", paths.creator);
});

const UNSAVED = "This card has changes you haven't saved.";

/** A fake account's store, as the server keeps it. */
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

function unloadIsCanceled(): boolean {
  const event = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(event);
  return event.defaultPrevented;
}

async function openRoute(): Promise<ReturnType<typeof render>> {
  const view = render(<CreatorRoute token="token-1" api={fakeApi()} />);
  await screen.findByText("0 of 50 cards");
  return view;
}

/** Any other path shows as text: what a move off the creator leaves behind. */
function Shell({ api }: { api: CreatorApi }): ReactElement {
  const path = usePathname();
  return path === paths.creator ? <CreatorRoute token="token-1" api={api} /> : <p data-testid="elsewhere">{path}</p>;
}

async function openShell(): Promise<void> {
  render(<Shell api={fakeApi()} />);
  await screen.findByText("0 of 50 cards");
}

/** The browser's Back, done once its `popstate` has fired. */
async function browserBack(): Promise<void> {
  await act(async () => {
    const popped = new Promise((resolve) => {
      window.addEventListener("popstate", resolve, { once: true });
    });
    window.history.back();
    await popped;
  });
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
    expect(unloadIsCanceled()).toBe(false);
    typeName("Half made");
    expect(unloadIsCanceled()).toBe(true);
  });

  it("R1484000 after a save Back leaves at once and a reload is not held, and a note Back opened goes away", async () => {
    await openRoute();
    typeName("Saved card");
    await waitFor(() => expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(false));
    expect(unloadIsCanceled()).toBe(true);

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(creatorTestid.save));
    await screen.findByTestId(creatorTestid.saved);
    // The note clears a render after the save.
    await waitFor(() => expect(screen.queryByText(UNSAVED)).toBeNull());
    // Its effect cleanup drops the beforeunload listener after the commit, and a busy scheduler can
    // yield in between, so the listener is waited for too.
    await waitFor(() => expect(unloadIsCanceled()).toBe(false));

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(window.location.pathname).toBe(paths.collection);
  });

  it("R1484000 the beforeunload listener is gone after the page unmounts", async () => {
    const view = await openRoute();
    typeName("Half made");
    expect(unloadIsCanceled()).toBe(true);
    view.unmount();
    expect(unloadIsCanceled()).toBe(false);
  });
});

describe("R1754000 every way off the creator asks before it drops unsaved changes", () => {
  it("R1754000 a move the queue or a challenge makes off a card with unsaved changes shows the note; Keep editing stays and is asked again, Discard them goes", async () => {
    await openShell();
    typeName("Half made");

    act(() => {
      navigate(paths.match("m-1"));
    });
    expect(window.location.pathname).toBe(paths.creator);
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();
    expect((screen.getByTestId(creatorTestid.name) as HTMLInputElement).value).toBe("Half made");

    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(screen.queryByText(UNSAVED)).toBeNull();

    act(() => {
      navigate(paths.match("m-1"));
    });
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Discard them" }));
    expect(window.location.pathname).toBe(paths.match("m-1"));
    expect(screen.getByTestId("elsewhere")).toHaveTextContent(paths.match("m-1"));
  });

  it("R1754000 the browser's Back off a card with unsaved changes stays on the card and shows the note; Discard them goes back", async () => {
    window.history.replaceState(null, "", paths.collection);
    window.history.pushState(null, "", paths.creator);
    await openShell();
    typeName("Half made");

    await browserBack();
    expect(window.location.pathname).toBe(paths.creator);
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();
    expect((screen.getByTestId(creatorTestid.name) as HTMLInputElement).value).toBe("Half made");
    expect(screen.queryByTestId("elsewhere")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(screen.queryByText(UNSAVED)).toBeNull();
    await browserBack();
    expect(screen.getByText(UNSAVED)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Discard them" }));
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.collection);
    });
    await screen.findByTestId("elsewhere");
  });

  it("R1754000 a card with nothing unsaved is left at once by the browser's Back and by any move", async () => {
    window.history.replaceState(null, "", paths.collection);
    window.history.pushState(null, "", paths.creator);
    await openShell();

    await browserBack();
    expect(window.location.pathname).toBe(paths.collection);
    expect(screen.queryByText(UNSAVED)).toBeNull();

    act(() => {
      navigate(paths.creator);
    });
    await screen.findByText("0 of 50 cards");

    act(() => {
      navigate(paths.match("m-1"));
    });
    expect(window.location.pathname).toBe(paths.match("m-1"));
    expect(screen.queryByText(UNSAVED)).toBeNull();
  });

  it("R1754000 Try it on a card with unsaved changes saves it and goes to practice without the note", async () => {
    await openShell();
    typeName("Tried card");
    await waitFor(() => expect((screen.getByTestId(creatorTestid.tryIt) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId(creatorTestid.tryIt));
    await waitFor(() => {
      expect(window.location.pathname + window.location.search).toBe(tryPath(`custom-${"1".padStart(32, "0")}`));
    });
    expect(screen.queryByText(UNSAVED)).toBeNull();
  });
});
