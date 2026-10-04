// R388, R507: the Patch notes page's body. Every patch newest first with its version, date, title
// and notes; the newest shows its cards on arrival and the others load their snapshots when opened;
// a patch's cards as faces with their changes marked, data-only changes and added cards as names by
// set, a name filter; a card opens its full history. Fixtures through the context, the empty and
// failed states, then the real history through the default source.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_CLOSE, INSPECT_DETAIL } from "../cards/inspect/testids.ts";
import { closeInspect } from "../cards/inspect/store.ts";
import { PatchSourceProvider } from "./context.tsx";
import { FIXTURE_PATCHES, V1, V2, V3, fixtureSource } from "./fixtures.ts";
import { PatchNotes, countsLine } from "./PatchNotes.tsx";
import { EMPTY_PATCH_SOURCE, realPatchSource, type PatchSource } from "./source.ts";
import { patchTestid } from "./testids.ts";

/** The real history's first read runs the JSON chunks through the transform. */
const SLOW = { timeout: 10_000 } as const;

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

function renderPage(source?: PatchSource): void {
  render(source === undefined ? <PatchNotes /> : <PatchSourceProvider source={source}>{<PatchNotes />}</PatchSourceProvider>);
}

function patchEntry(version: string): HTMLElement {
  const found = screen.getAllByTestId(patchTestid.patch).find((element) => element.dataset.version === version);
  if (found === undefined) throw new Error(`no patch ${version}`);
  return found;
}

function byCard(testId: string, id: string): HTMLElement {
  const found = screen.getAllByTestId(testId).find((element) => element.dataset.card === id);
  if (found === undefined) throw new Error(`no ${testId} for ${id}`);
  return found;
}

describe("R388 the Patch notes page", () => {
  it("R388 lists every patch newest first, in the file's order, with its version, date, title, source, notes and counts", async () => {
    renderPage(fixtureSource());
    const entries = await screen.findAllByTestId(patchTestid.patch);
    expect(entries.map((element) => element.dataset.version)).toEqual([V3, V2, V1]);
    const newest = patchEntry(V3);
    expect(within(newest).getByRole("heading", { level: 2 })).toHaveTextContent(`${V3} Two new sets`);
    expect(newest).toHaveTextContent("2026-03-04");
    expect(newest).toHaveTextContent("Fixture commit 3");
    expect(newest).toHaveTextContent("Classic and Classic+ arrive.");
    expect(newest).toHaveTextContent("3 cards added · 2 cards changed");
    const second = FIXTURE_PATCHES.find((patch) => patch.version === V2);
    expect(second === undefined ? "" : countsLine(second)).toBe("1 card changed · 1 card removed");
  });

  it("R507 only the newest patch shows its cards on arrival; the others load their snapshots when opened", async () => {
    const asked: string[] = [];
    renderPage(fixtureSource((version) => asked.push(version)));
    await screen.findByTestId(patchTestid.cards);
    const toggles = screen.getAllByTestId(patchTestid.toggle);
    expect(toggles.map((toggle) => toggle.getAttribute("aria-expanded"))).toEqual(["true", "false", "false"]);
    expect(within(patchEntry(V1)).getByTestId(patchTestid.toggle)).toHaveTextContent("Show the 4 cards");
    expect(new Set(asked)).toEqual(new Set([V3, V2]));

    fireEvent.click(within(patchEntry(V1)).getByTestId(patchTestid.toggle));
    expect(await within(patchEntry(V1)).findByTestId(patchTestid.cards)).toBeInTheDocument();
    expect(asked).toContain(V1);
    expect(within(patchEntry(V1)).getByTestId(patchTestid.toggle)).toHaveTextContent("Hide the cards");

    fireEvent.click(within(patchEntry(V3)).getByTestId(patchTestid.toggle));
    expect(within(patchEntry(V3)).queryByTestId(patchTestid.cards)).toBeNull();
  });

  it("R388 R507 shows a changed card as its face from the patch's snapshot, with its changes marked", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    const bolt = byCard(patchTestid.changedCard, "core-002");
    expect(bolt.querySelector(".cf .card-name")).toHaveTextContent("Test Bolt");
    expect(bolt.querySelector(".cost-gem")).toHaveAttribute("data-cost", "2");
    const marked = Array.from(bolt.querySelectorAll("ins.patch-mark")).map((element) => element.textContent);
    expect(marked).toEqual(["4", "Your", "Your"]);
    const struck = Array.from(bolt.querySelectorAll("del.patch-cut")).map((element) => element.textContent);
    expect(struck).toEqual(["3", "this turn", "this turn"]);
    expect(bolt).toHaveTextContent("(1) Cost → becomes (2) Cost, embiggen (4)");
  });

  it("R507 lists a card changed only in data the face doesn't print by name, with what changed", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    const golem = byCard(patchTestid.dataOnly, "core-004");
    expect(golem).toHaveTextContent("Quiet Golem");
    expect(golem).toHaveTextContent("Lines of code: not recorded → 12");
    expect(golem.querySelector(".cf")).toBeNull();
  });

  it("R507 lists added cards as names grouped by set, each set's tokens apart", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    const groups = screen.getAllByTestId(patchTestid.addedGroup);
    expect(groups.map((group) => [group.dataset.group, within(group).getAllByTestId(patchTestid.openCard).map((name) => name.textContent)])).toEqual([
      ["Classic", ["Fresh Face"]],
      ["Classic+", ["Pancake Pal"]],
      ["Classic+ tokens", ["Syrup Token"]],
    ]);
    expect(groups[0]?.querySelector(".cf")).toBeNull();
  });

  it("R507 lists removed cards by name", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    fireEvent.click(within(patchEntry(V2)).getByTestId(patchTestid.toggle));
    await within(patchEntry(V2)).findByTestId(patchTestid.cards);
    expect(byCard(patchTestid.removed, "core-003")).toHaveTextContent("Old Relic");
  });

  it("R507 filters a patch's cards by name, and says when nothing matches", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    const filter = within(patchEntry(V3)).getByTestId(patchTestid.filter);
    expect(within(patchEntry(V3)).getByLabelText(`Find a card in ${V3}`)).toBe(filter);
    fireEvent.change(filter, { target: { value: "PAL" } });
    expect(within(patchEntry(V3)).getAllByTestId(patchTestid.openCard).map((name) => name.textContent)).toEqual(["Pancake Pal"]);
    expect(within(patchEntry(V3)).queryAllByTestId(patchTestid.changedCard)).toEqual([]);
    fireEvent.change(filter, { target: { value: "zzz" } });
    expect(within(patchEntry(V3)).getByTestId(patchTestid.noMatch)).toHaveTextContent("No card in this patch matches “zzz”.");
  });

  it("R388 a card's name opens it as it stands now, in the detail view with its whole history open", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    const name = within(byCard(patchTestid.changedCard, "core-002")).getByTestId(patchTestid.openCard);
    name.focus();
    fireEvent.click(name);
    const detail = await screen.findByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-002");
    expect(within(detail).getByTestId(patchTestid.historyToggle)).toHaveAttribute("aria-expanded", "true");
    const entries = await within(detail).findAllByTestId(patchTestid.historyEntry);
    expect(entries.map((element) => element.dataset.version)).toEqual([V3, V1]);
    fireEvent.click(within(detail).getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    expect(name).toHaveFocus();
  });

  it("R388 a removed card opens as it stood before it was removed", async () => {
    renderPage(fixtureSource());
    await screen.findByTestId(patchTestid.cards);
    fireEvent.click(within(patchEntry(V2)).getByTestId(patchTestid.toggle));
    await within(patchEntry(V2)).findByTestId(patchTestid.cards);
    fireEvent.click(within(byCard(patchTestid.removed, "core-003")).getByTestId(patchTestid.openCard));
    const detail = await screen.findByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-003");
    const entries = await within(detail).findAllByTestId(patchTestid.historyEntry);
    expect(entries.map((element) => element.dataset.kind)).toEqual(["removed", "added"]);
  });

  it("R388 with no patch data the page says there are no patch notes yet", async () => {
    renderPage(EMPTY_PATCH_SOURCE);
    expect(await screen.findByTestId(patchTestid.empty)).toHaveTextContent("There are no patch notes yet.");
    expect(screen.queryAllByTestId(patchTestid.patch)).toEqual([]);
  });

  it("R388 patch notes that fail to load say so and try again", async () => {
    const source = fixtureSource();
    let failures = 1;
    const patches = source.patches;
    const quiet = vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderPage({ ...source, patches: () => (failures-- > 0 ? Promise.reject(new Error("offline")) : patches()) });
    expect(await screen.findByTestId(patchTestid.error)).toHaveTextContent("The patch notes didn’t load.");
    fireEvent.click(screen.getByTestId(patchTestid.retry));
    expect(await screen.findAllByTestId(patchTestid.patch)).toHaveLength(3);
    quiet.mockRestore();
  });

  it("R388 a patch whose snapshot fails to load says so and tries again", async () => {
    const source = fixtureSource();
    let failures = 1;
    const snapshot = source.snapshot;
    const quiet = vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderPage({
      ...source,
      snapshot: (version) => (version === V2 && failures-- > 0 ? Promise.reject(new Error("offline")) : snapshot(version)),
    });
    expect(await screen.findByTestId(patchTestid.cardsError)).toHaveTextContent("This patch’s cards didn’t load.");
    fireEvent.click(screen.getByTestId(patchTestid.cardsRetry));
    expect(await screen.findByTestId(patchTestid.cards)).toBeInTheDocument();
    quiet.mockRestore();
  });
});

describe("R388 the Patch notes page over the real history", () => {
  it("R388 lists every patch newest first and opens v0.2.0 with Masochism Mask's new cost and the new sets by name", async () => {
    renderPage();
    const shipped = await realPatchSource.patches();
    const entries = await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    expect(entries.map((element) => element.dataset.version)).toEqual(shipped.map((patch) => patch.version).reverse());
    // Only the newest opens on load (R507); an older patch's cards wait for its toggle, so v0.2.0
    // — the patch this test reads, frozen history — is opened by hand once it is no longer newest.
    // Today the newest is v0.2.11 (issue #218): it opens on arrival with every card it changed.
    // Either animated patch opens the same way (v0.2.10 added the faces, v0.2.11 removes them).
    const newest = shipped.at(-1)?.version ?? "";
    await within(patchEntry(newest)).findByTestId(patchTestid.cards, undefined, SLOW);
    if (newest === "v0.2.10" || newest === "v0.2.11") {
      const record = shipped.find((patch) => patch.version === newest);
      const changedIds = (record?.changes ?? []).filter((change) => change.kind === "changed").map((change) => change.id);
      const shownNewest = [
        ...within(patchEntry(newest)).getAllByTestId(patchTestid.changedCard),
        ...within(patchEntry(newest)).queryAllByTestId(patchTestid.dataOnly),
      ].map((element) => element.dataset.card);
      expect(shownNewest.sort()).toEqual([...changedIds].sort());
      fireEvent.click(within(patchEntry(newest)).getByTestId(patchTestid.toggle));
    }
    const v020 = patchEntry("v0.2.0");
    if (within(v020).getByTestId(patchTestid.toggle).getAttribute("aria-expanded") === "false") {
      fireEvent.click(within(v020).getByTestId(patchTestid.toggle));
    }
    await within(v020).findByTestId(patchTestid.cards, undefined, SLOW);
    const mask = byCard(patchTestid.changedCard, "core-065");
    expect(mask).toHaveTextContent("(2) Cost → becomes (1) Cost");
    expect(mask.querySelector(".cost-gem")).toHaveAttribute("data-cost", "1");
    const hinder = byCard(patchTestid.changedCard, "core-021");
    expect(Array.from(hinder.querySelectorAll("ins.patch-mark")).map((element) => element.textContent)).toEqual(["Discard 1"]);
    // Every card v0.2.0 records as changed is on the page once: a face, or a name in the data-only list.
    const record = shipped.find((patch) => patch.version === "v0.2.0");
    const changedIds = (record?.changes ?? []).filter((change) => change.kind === "changed").map((change) => change.id);
    const shown = [...within(v020).getAllByTestId(patchTestid.changedCard), ...within(v020).getAllByTestId(patchTestid.dataOnly)].map(
      (element) => element.dataset.card,
    );
    expect(shown.sort()).toEqual([...changedIds].sort());
    expect(within(v020).getAllByTestId(patchTestid.dataOnly).length).toBeGreaterThan(0);
    const groups = within(v020).getAllByTestId(patchTestid.addedGroup);
    expect(groups.map((group) => [group.dataset.group, within(group).getAllByTestId(patchTestid.openCard).length])).toEqual([
      ["Classic", 90],
      ["Classic+", 78],
      ["Classic+ tokens", 38],
    ]);
    // The older patches wait to be opened.
    expect(within(patchEntry("v0.1.0")).queryByTestId(patchTestid.cards)).toBeNull();
  }, 30_000);
});
