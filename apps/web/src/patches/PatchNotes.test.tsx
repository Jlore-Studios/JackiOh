// R388, R507, R724000: the Patch notes page's body. Patches by release line, one bubble per set, the
// newest open; every patch newest first with its version, date, player note title and note; the
// newest shows its cards on arrival and the others load their snapshots when opened; a patch's cards
// as faces with their changes marked, data-only changes and added cards as names by set, a name
// filter; a card opens its full history. Fixtures through the context, the empty and failed states,
// then the real history through the default source.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_CLOSE, INSPECT_DETAIL } from "../cards/inspect/testids.ts";
import { closeInspect } from "../cards/inspect/store.ts";
import { PatchSourceProvider } from "./context.tsx";
import { FIXTURE_PATCHES, V1, V2, V3, fixtureSource } from "./fixtures.ts";
import { PatchNotes, countsLine } from "./PatchNotes.tsx";
import { RELEASES, type Release } from "./releases.ts";
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

function renderPage(source?: PatchSource, releases?: readonly Release[]): void {
  const page = releases === undefined ? <PatchNotes /> : <PatchNotes releases={releases} />;
  render(source === undefined ? page : <PatchSourceProvider source={source}>{page}</PatchSourceProvider>);
}

function releaseSet(id: string): HTMLElement {
  const found = screen.getAllByTestId(patchTestid.set).find((element) => element.dataset.set === id);
  if (found === undefined) throw new Error(`no release line ${id}`);
  return found;
}

/** Opens every release line that is shut. */
function openEverySet(): void {
  for (const toggle of screen.getAllByTestId(patchTestid.setToggle)) {
    if (toggle.getAttribute("aria-expanded") === "false") fireEvent.click(toggle);
  }
}

function patchEntry(version: string): HTMLElement {
  const found = screen.getAllByTestId(patchTestid.patch).find((element) => element.dataset.version === version);
  if (found === undefined) throw new Error(`no patch ${version}`);
  return found;
}

function byCard(testId: string, id: string, inside: HTMLElement = document.body): HTMLElement {
  const found = within(inside).getAllByTestId(testId).find((element) => element.dataset.card === id);
  if (found === undefined) throw new Error(`no ${testId} for ${id}`);
  return found;
}

describe("R388 the Patch notes page", () => {
  it("R388 R724000 lists every patch newest first, in the file's order, with its version, date, title, note and counts", async () => {
    renderPage(fixtureSource());
    const entries = await screen.findAllByTestId(patchTestid.patch);
    expect(entries.map((element) => element.dataset.version)).toEqual([V3, V2, V1]);
    const newest = patchEntry(V3);
    expect(within(newest).getByRole("heading", { level: 3 })).toHaveTextContent(`${V3} Two fixture sets`);
    expect(newest).toHaveTextContent("2026-03-04");
    expect(within(newest).getByTestId(patchTestid.note)).toHaveTextContent("Test Bolt costs (2), up from (1).");
    expect(newest).not.toHaveTextContent("Fixture commit 3");
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
    expect(bolt).toHaveTextContent("(1) Cost → becomes (2) Cost, Embiggened (4)");
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
    await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    // Only the newest release line opens on load (R724000): open the others to read the whole list.
    openEverySet();
    const entries = screen.getAllByTestId(patchTestid.patch);
    // Newest first is patches.json read backwards (R105, R388) — and promotions only append
    // (R646), so the page's list is the file's, whatever the newest patch is called today.
    expect(entries.map((element) => element.dataset.version)).toEqual(shipped.map((patch) => patch.version).reverse());
    // Only the newest opens on load (R507); an older patch's cards wait for its toggle, so v0.2.0
    // — the patch this test reads, frozen history — is opened by hand once it is no longer newest.
    // v0.2.3 added the Mask's faces, v0.2.4 aimed random casts and v0.2.5 removes the faces again
    // (R743's names for issues #113, #181 and #218): whichever of the three is the newest opens with
    // every card it changed.
    const newest = shipped.at(-1)?.version ?? "";
    await within(patchEntry(newest)).findByTestId(patchTestid.cards, undefined, SLOW);
    if (newest === "v0.2.3" || newest === "v0.2.4" || newest === "v0.2.5") {
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
    // Inside v0.2.0's entry: the newest patch is open too, and a promoted one may change the Mask again (R646).
    const mask = byCard(patchTestid.changedCard, "core-065", v020);
    expect(mask).toHaveTextContent("(2) Cost → becomes (1) Cost");
    expect(mask.querySelector(".cost-gem")).toHaveAttribute("data-cost", "1");
    const hinder = byCard(patchTestid.changedCard, "core-021", v020);
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

describe("R724000 the Patch notes page by release line", () => {
  const LINES: readonly Release[] = [
    { id: "older", name: "Older line", first: V1, summary: "The older line's summary.", technical: ["An older technical change."] },
    { id: "newer", name: "Newer line", first: V3, summary: "The newer line's summary.", technical: [] },
  ];

  it("R724000 groups patches by release line, newest first, only the newest open", async () => {
    renderPage(fixtureSource(), LINES);
    await screen.findAllByTestId(patchTestid.patch);
    const sets = screen.getAllByTestId(patchTestid.set);
    expect(sets.map((element) => element.dataset.set)).toEqual(["newer", "older"]);
    const toggles = screen.getAllByTestId(patchTestid.setToggle);
    expect(toggles.map((toggle) => toggle.getAttribute("aria-expanded"))).toEqual(["true", "false"]);
    expect(within(releaseSet("newer")).getByRole("heading", { level: 2 })).toHaveTextContent(`Newer line${V3}1 patch`);
    expect(screen.getAllByTestId(patchTestid.patch).map((element) => element.dataset.version)).toEqual([V3]);
    expect(screen.queryAllByTestId(patchTestid.technical)).toEqual([]);

    const older = releaseSet("older");
    const toggle = within(older).getByTestId(patchTestid.setToggle);
    expect(toggle).toHaveTextContent(`Older line${V1} – ${V2}2 patches`);
    expect(document.getElementById(toggle.getAttribute("aria-controls") ?? "")).not.toBeVisible();
    expect(older).not.toHaveTextContent("The older line's summary.");
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(older).toHaveTextContent("The older line's summary.");
    expect(within(older).getByTestId(patchTestid.technical)).toHaveTextContent("Technical changesAn older technical change.");
    expect(within(older).getAllByTestId(patchTestid.patch).map((element) => element.dataset.version)).toEqual([V2, V1]);
    // Opening a line opens no patch's cards: only the newest patch shows them on arrival (R507).
    expect(within(older).queryByTestId(patchTestid.cards)).toBeNull();

    fireEvent.click(within(releaseSet("newer")).getByTestId(patchTestid.setToggle));
    expect(within(releaseSet("newer")).queryAllByTestId(patchTestid.patch)).toEqual([]);
  });

  it("R724000 a patch before every release line's first sits under Earlier patches", async () => {
    renderPage(fixtureSource());
    await screen.findAllByTestId(patchTestid.patch);
    const sets = screen.getAllByTestId(patchTestid.set);
    expect(sets.map((element) => element.dataset.set)).toEqual(["earlier"]);
    expect(within(releaseSet("earlier")).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "true");
    expect(within(releaseSet("earlier")).getByTestId(patchTestid.setToggle)).toHaveTextContent(`Earlier patches${V1} – ${V3}3 patches`);
    expect(within(releaseSet("earlier")).getAllByTestId(patchTestid.patch)).toHaveLength(3);
    expect(screen.queryAllByTestId(patchTestid.technical)).toEqual([]);
  });

  it("R724000 a patch shows its player note under fixed headings, never its record's source or notes", async () => {
    renderPage(fixtureSource());
    await screen.findAllByTestId(patchTestid.patch);
    const third = patchEntry(V3);
    expect(within(third).getByRole("heading", { level: 3 })).toHaveTextContent(`${V3} Two fixture sets`);
    const sections = within(third).getAllByTestId(patchTestid.noteSection);
    expect(sections.map((element) => element.dataset.section)).toEqual(["new", "balance", "fixes"]);
    expect(sections.map((element) => within(element).getByRole("heading", { level: 4 }).textContent)).toEqual(["New", "Balance", "Fixes"]);
    expect(within(sections[2] ?? third).getAllByRole("listitem").map((item) => item.textContent)).toEqual(["Quiet Golem records its lines of code."]);
    expect(third).not.toHaveTextContent("Fixture commit 3");
    expect(third).not.toHaveTextContent("Classic and Classic+ arrive.");
    expect(third).not.toHaveTextContent("Two new sets");
    // A patch with no note shows its record's title, and nothing of its record's source or notes.
    const second = patchEntry(V2);
    expect(within(second).getByRole("heading", { level: 3 })).toHaveTextContent(`${V2} The knight grows`);
    expect(within(second).queryByTestId(patchTestid.note)).toBeNull();
    expect(second).not.toHaveTextContent("Fixture commit 2");
    expect(second).not.toHaveTextContent("Old Relic leaves.");
  });

  it("R724000 the real history opens on Meditative and every patch has its note", async () => {
    renderPage();
    const shipped = await realPatchSource.patches();
    await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    expect(screen.getAllByTestId(patchTestid.set).map((element) => element.dataset.set)).toEqual(["meditative", "classic", "core"]);
    expect(screen.getAllByTestId(patchTestid.setToggle).map((toggle) => toggle.getAttribute("aria-expanded"))).toEqual(["true", "false", "false"]);
    // Every line starts at a shipped patch, in the lines' order.
    const versions = shipped.map((patch) => patch.version);
    const firsts = RELEASES.map((release) => versions.indexOf(release.first));
    expect(firsts.every((at) => at >= 0)).toBe(true);
    expect([...firsts].sort((a, b) => a - b)).toEqual(firsts);
    expect(firsts[0]).toBe(0);
    openEverySet();
    const entries = screen.getAllByTestId(patchTestid.patch);
    expect(entries.map((element) => element.dataset.version)).toEqual([...versions].reverse());
    for (const entry of entries) {
      const patch = shipped.find((each) => each.version === entry.dataset.version);
      expect(patch?.player, entry.dataset.version).toBeDefined();
      expect(within(entry).getByRole("heading", { level: 3 })).toHaveTextContent(`${patch?.version ?? ""} ${patch?.player?.title ?? ""}`);
      expect(within(entry).getByTestId(patchTestid.note)).toBeInTheDocument();
      expect(entry).not.toHaveTextContent(patch?.source ?? "");
    }
    // Every line that has technical changes shows them once it is opened.
    expect(screen.getAllByTestId(patchTestid.technical)).toHaveLength(RELEASES.filter((release) => release.technical.length > 0).length);
  }, 30_000);
});
