// R388, R507, R724000: the Patch notes page's body, from fixtures through the context, the empty and
// failed states, then the real history through the default source. R3012000, R3012001: micro patches
// nested under their patch, the legend on the side, and art for a major and a patch only.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_CLOSE, INSPECT_DETAIL } from "../cards/inspect/testids.ts";
import { closeInspect } from "../cards/inspect/store.ts";
import { expectEmptyState } from "../test/empty.ts";
import { stubPatchLegendPhone } from "../test/phoneBar.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { PATCH_LEGEND_BAND_MARGIN } from "./constants.ts";
import { PatchSourceProvider } from "./context.tsx";
import { FIXTURE_INDEX, FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, M1, M2, TIERED_PATCHES, V1, V2, V3, fixtureSource, tieredSource } from "./fixtures.ts";
import { PatchNotes } from "./PatchNotes.tsx";
import { countsLine } from "./history.ts";
import { releaseArtPath } from "./releaseArt.ts";
import { RELEASES, majors, releaseGroups, tiers, type Release } from "./releases.ts";
import { EMPTY_PATCH_SOURCE, realPatchSource, sourceFromData, type Patch, type PatchSource, type TechnicalEntry } from "./source.ts";
import { patchTestid } from "./testids.ts";

/** The real history's first read runs the JSON chunks through the transform. */
const SLOW = { timeout: 10_000 } as const;

/** R1369000: each line's first version in the real history, its Z's major, shipped before tiers were recorded. */
const REAL_FIRSTS: readonly (readonly [string, string])[] = [
  ["core", "v0.1.0"],
  ["classic", "v0.2.0"],
  ["meditative", "v0.3.0"],
  ["meditativeplus", "v0.4.0"],
];

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.unstubAllGlobals();
  window.history.replaceState(null, "", "/");
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

function openEverySet(): void {
  for (const toggle of screen.getAllByTestId(patchTestid.setToggle)) {
    if (toggle.getAttribute("aria-expanded") === "false") fireEvent.click(toggle);
  }
}

/** The entry of a version: a major's or a patch's, or a micro patch nested inside one (R3012000). */
function patchEntry(version: string): HTMLElement {
  const found = [...screen.queryAllByTestId(patchTestid.patch), ...screen.queryAllByTestId(patchTestid.micro)].find(
    (element) => element.dataset.version === version,
  );
  if (found === undefined) throw new Error(`no patch ${version}`);
  return found;
}

/** An entry's own control or part: it comes first in the DOM, before those of the micro patches inside it (R3012000). */
function own(entry: HTMLElement, testId: string): HTMLElement {
  const found = within(entry).getAllByTestId(testId)[0];
  if (found === undefined) throw new Error(`no ${testId} in ${entry.dataset.version ?? "the entry"}`);
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
    expectSkeleton(within(patchEntry(V1)).getByTestId(patchTestid.cardsLoading), "grid");
    expect(await within(patchEntry(V1)).findByTestId(patchTestid.cards)).toBeInTheDocument();
    expect(within(patchEntry(V1)).queryByTestId(patchTestid.cardsLoading)).toBeNull();
    expect(asked).toContain(V1);
    expect(within(patchEntry(V1)).getByTestId(patchTestid.toggle)).toHaveTextContent("Hide the cards");

    fireEvent.click(within(patchEntry(V3)).getByTestId(patchTestid.toggle));
    expect(within(patchEntry(V3)).queryByTestId(patchTestid.cards)).toBeNull();
  });

  it("#2115 the page waits as a rows skeleton, and the patches take its place when the read answers", async () => {
    renderPage(fixtureSource());

    expectSkeleton(screen.getByTestId(patchTestid.loading), "rows");
    expect(screen.getByTestId(patchTestid.loading)).toHaveTextContent("Loading the patch notes…");
    expect(screen.queryAllByTestId(patchTestid.patch)).toEqual([]);

    expect((await screen.findAllByTestId(patchTestid.patch)).length).toBeGreaterThan(0);
    expect(screen.queryByTestId(patchTestid.loading)).toBeNull();
    expect(screen.queryByText("Loading the patch notes…")).toBeNull();
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
    const noMatch = within(patchEntry(V3)).getByTestId(patchTestid.noMatch);
    expect(noMatch).toHaveTextContent("No card in this patch matches “zzz”.");
    expectEmptyState(noMatch);
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
    const empty = await screen.findByTestId(patchTestid.empty);
    expect(empty).toHaveTextContent("There are no patch notes yet.");
    expectEmptyState(empty);
    expect(screen.queryAllByTestId(patchTestid.patch)).toEqual([]);
  });

  it("R388 patch notes that fail to load say so and try again", async () => {
    const source = fixtureSource();
    let failures = 1;
    const patches = source.patches;
    const quiet = vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderPage({ ...source, patches: () => (failures-- > 0 ? Promise.reject(new Error("offline")) : patches()) });
    expect(await screen.findByTestId(patchTestid.error)).toHaveTextContent("The patch notes didn't load.");
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
    expect(await screen.findByTestId(patchTestid.cardsError)).toHaveTextContent("This patch's cards didn't load.");
    fireEvent.click(screen.getByTestId(patchTestid.cardsRetry));
    expect(await screen.findByTestId(patchTestid.cards)).toBeInTheDocument();
    quiet.mockRestore();
  });

  it("#2052 a card still loading shows its loading line where its detail opens, then the detail", async () => {
    const source = fixtureSource();
    const index = source.index;
    let release = (): void => undefined;
    const held = new Promise<void>((resolve) => {
      release = () => {
        resolve();
      };
    });
    renderPage({ ...source, index: () => held.then(() => index()) });
    await screen.findByTestId(patchTestid.cards);
    fireEvent.click(within(byCard(patchTestid.changedCard, "core-002")).getByTestId(patchTestid.openCard));
    expect(screen.queryByTestId(patchTestid.opening)).toBeNull();
    const loading = await screen.findByTestId(patchTestid.openingLoading);
    expect(loading).toHaveTextContent("Loading this card…");
    expect(loading).toHaveAttribute("role", "status");
    expect(screen.getByTestId(patchTestid.opening)).toHaveAttribute("role", "dialog");
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    release();
    const detail = await screen.findByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-002");
    expect(screen.queryByTestId(patchTestid.opening)).toBeNull();
  });

  it("#2052 a card that fails to open says so where its detail would be, and Try again opens it", async () => {
    const source = fixtureSource();
    let failures = 1;
    const index = source.index;
    const quiet = vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderPage({ ...source, index: () => (failures-- > 0 ? Promise.reject(new Error("offline")) : index()) });
    await screen.findByTestId(patchTestid.cards);
    fireEvent.click(within(byCard(patchTestid.changedCard, "core-002")).getByTestId(patchTestid.openCard));
    const error = await screen.findByTestId(patchTestid.openingError);
    expect(error).toHaveTextContent("This card didn't load.");
    expect(error).toHaveAttribute("role", "alert");
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    fireEvent.click(screen.getByTestId(patchTestid.openingRetry));
    const detail = await screen.findByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-002");
    expect(screen.queryByTestId(patchTestid.opening)).toBeNull();
    quiet.mockRestore();
  });

  it("#2052 a card that fails to open closes on Close and gives focus back to its name", async () => {
    const source = fixtureSource();
    const quiet = vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderPage({ ...source, index: () => Promise.reject(new Error("offline")) });
    await screen.findByTestId(patchTestid.cards);
    const name = within(byCard(patchTestid.changedCard, "core-002")).getByTestId(patchTestid.openCard);
    name.focus();
    fireEvent.click(name);
    await screen.findByTestId(patchTestid.openingError);
    expect(screen.getByTestId(patchTestid.openingClose)).toHaveFocus();
    fireEvent.click(screen.getByTestId(patchTestid.openingClose));
    expect(screen.queryByTestId(patchTestid.opening)).toBeNull();
    expect(name).toHaveFocus();
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
    // (R646), so the page's list is the file's, whatever the newest patch is called today. A micro
    // patch is no entry of its own: it sits inside the entry of the major or patch before it (R3012000).
    const tierList = tiers(shipped);
    const hosts = shipped.filter((_patch, at) => tierList[at] !== "micro").map((patch) => patch.version);
    expect(entries.map((element) => element.dataset.version)).toEqual(hosts.reverse());
    expect([...entries, ...screen.getAllByTestId(patchTestid.micro)].map((element) => element.dataset.version).sort()).toEqual(
      shipped.map((patch) => patch.version).sort(),
    );
    // Only the newest opens on load (R507), so v0.2.0, the frozen patch this test reads, is opened by
    // hand; whichever is newest (R743's names for issues #113, #181 and #218) opens with every card it changed.
    const newest = shipped.at(-1)?.version ?? "";
    // A newest patch that changed no card has no cards to open: its entry says so instead.
    if ((shipped.at(-1)?.changes ?? []).length === 0) {
      expect(patchEntry(newest)).toHaveTextContent("No card changed.");
      expect(within(patchEntry(newest)).queryByTestId(patchTestid.toggle)).toBeNull();
    } else {
      await within(patchEntry(newest)).findByTestId(patchTestid.cards, undefined, SLOW);
    }
    if (newest === "v0.2.2" || newest === "v0.2.2b" || newest === "v0.2.3") {
      const record = shipped.find((patch) => patch.version === newest);
      const changedIds = (record?.changes ?? []).filter((change) => change.kind === "changed").map((change) => change.id);
      const shownNewest = [
        ...within(patchEntry(newest)).getAllByTestId(patchTestid.changedCard),
        ...within(patchEntry(newest)).queryAllByTestId(patchTestid.dataOnly),
      ].map((element) => element.dataset.card);
      expect(shownNewest.sort()).toEqual([...changedIds].sort());
      fireEvent.click(own(patchEntry(newest), patchTestid.toggle));
    }
    const v020 = patchEntry("v0.2.0");
    if (own(v020, patchTestid.toggle).getAttribute("aria-expanded") === "false") {
      fireEvent.click(own(v020, patchTestid.toggle));
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
    expect(within(patchEntry("v0.1.0")).queryByTestId(patchTestid.cards)).toBeNull();
  }, 30_000);
});

describe("R724000 the Patch notes page by release line", () => {
  const LINES: readonly Release[] = [
    { id: "older", name: "Older line", summary: "The older line's summary.", technical: ["An older technical change."] },
    { id: "newer", name: "Newer line", summary: "The newer line's summary.", technical: [] },
  ];

  /** The fixtures as shipped before tiers were recorded: V1 and V3 are majors by the sets they add. */
  const untiered = (): PatchSource =>
    sourceFromData({
      patches: FIXTURE_PATCHES.map(({ tier: _tier, ...patch }): Patch => patch),
      index: FIXTURE_INDEX,
      snapshots: FIXTURE_SNAPSHOTS,
    });

  it("R724000 R1369000 groups patches by release line, each opened by its major, newest first, only the newest open", async () => {
    renderPage(untiered(), LINES);
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

  it("R724000 R1369000 a patch before every major sits under Earlier patches", async () => {
    // A recorded tier wins over the fallback: the fixtures' say micro, so none of them is a major.
    renderPage(fixtureSource());
    await screen.findAllByTestId(patchTestid.patch);
    const sets = screen.getAllByTestId(patchTestid.set);
    expect(sets.map((element) => element.dataset.set)).toEqual(["earlier"]);
    expect(within(releaseSet("earlier")).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "true");
    expect(within(releaseSet("earlier")).getByTestId(patchTestid.setToggle)).toHaveTextContent(`Earlier patches${V1} – ${V3}3 patches`);
    expect(within(releaseSet("earlier")).getAllByTestId(patchTestid.patch)).toHaveLength(3);
    expect(screen.queryAllByTestId(patchTestid.technical)).toEqual([]);
  });

  it("R2204000 a wrapped set's patch opens no line and sits under the renamed line it joins", () => {
    const base = FIXTURE_PATCHES[0];
    if (base === undefined) throw new Error("no fixture patch");
    const sets = ["core", "classic", "meditative", "meditativeplus", "stinkyscape"];
    const shipped: Patch[] = sets.map((set, at) => ({
      ...base,
      version: `v0.${String(at + 1)}.0`,
      tier: "major",
      changes: [{ id: `${set}-001`, name: set, kind: "added" }],
    }));
    shipped.push({
      ...base,
      version: "v0.5.0b",
      tier: "patch",
      changes: [{ id: "traininggrounds-001", name: "Spikey Steven", kind: "added" }],
    });
    const groups = releaseGroups(shipped, RELEASES);
    expect(groups.map(({ release }) => release.id)).toEqual([...sets].reverse());
    const newest = groups[0];
    expect(newest?.release.name).toBe("Stinkyscape & Training Grounds");
    expect(newest?.patches.map((patch) => patch.version)).toEqual(["v0.5.0b", "v0.5.0"]);
    expect(newest?.release.summary).toContain("Training Grounds");
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

  it("R724000 R1369000 the real history opens on its newest line, each line at its Z's first version, and every patch has its note", async () => {
    renderPage();
    const shipped = await realPatchSource.patches();
    await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    const versions = shipped.map((patch) => patch.version);
    const ids = RELEASES.slice(0, majors(shipped).filter(Boolean).length)
      .map((release) => release.id)
      .reverse();
    expect(screen.getAllByTestId(patchTestid.set).map((element) => element.dataset.set)).toEqual(ids);
    expect(screen.getAllByTestId(patchTestid.setToggle).map((toggle) => toggle.getAttribute("aria-expanded"))).toEqual(
      ids.map((_id, at) => (at === 0 ? "true" : "false")),
    );
    // Every line opens at its Z's first version, the first patch of the history opening the first.
    const firsts = releaseGroups(shipped, RELEASES).map(({ release, patches }) => [release.id, patches.at(-1)?.version]);
    expect([...firsts].reverse().slice(0, REAL_FIRSTS.length)).toEqual(REAL_FIRSTS);
    expect(versions[0]).toBe(REAL_FIRSTS[0]?.[1]);
    openEverySet();
    // Entries are the majors and patches, newest first; each micro sits inside the one before it (R3012000).
    const tierList = tiers(shipped);
    const hosts = versions.filter((_version, at) => tierList[at] !== "micro");
    const entries = screen.getAllByTestId(patchTestid.patch);
    expect(entries.map((element) => element.dataset.version)).toEqual([...hosts].reverse());
    const micros = screen.getAllByTestId(patchTestid.micro);
    expect(micros.map((element) => element.dataset.version).sort()).toEqual(versions.filter((_version, at) => tierList[at] === "micro").sort());
    for (const micro of micros) {
      const at = versions.indexOf(micro.dataset.version ?? "");
      const host = versions.slice(0, at).findLast((_version, earlier) => tierList[earlier] !== "micro");
      expect(micro.closest(`[data-testid="${patchTestid.patch}"]`)?.getAttribute("data-version"), micro.dataset.version).toBe(host);
    }
    for (const entry of [...entries, ...micros]) {
      const patch = shipped.find((each) => each.version === entry.dataset.version);
      expect(patch?.player, entry.dataset.version).toBeDefined();
      // A major's or patch's title is an h3; a micro patch's, under it, an h4.
      const level = entry.dataset.testid === patchTestid.micro ? 4 : 3;
      expect(within(entry).getAllByRole("heading", { level })[0]).toHaveTextContent(`${patch?.version ?? ""} ${patch?.player?.title ?? ""}`);
      expect(own(entry, patchTestid.note)).toBeInTheDocument();
      expect(entry).not.toHaveTextContent(patch?.source ?? "");
    }
    expect(screen.getAllByTestId(patchTestid.technical)).toHaveLength(RELEASES.slice(0, ids.length).filter((release) => release.technical.length > 0).length);
  }, 30_000);
});

describe("R1369000 the technical log on the Patch notes page", () => {
  const entry = (fields: Partial<TechnicalEntry> & Pick<TechnicalEntry, "version" | "kind">): TechnicalEntry => ({
    commit: `${fields.version}-${fields.kind}-${fields.player ?? "none"}`,
    title: "a change",
    ...fields,
  });
  const LOG: readonly TechnicalEntry[] = [
    entry({ version: V3, kind: "app", pr: 21, player: "The main menu remembers its tab." }),
    entry({ version: V3, kind: "ai", pr: 22, player: "The practice AI guards its Taunts." }),
    entry({ version: V3, kind: "internal", pr: 23, player: "A faster build." }),
    entry({ version: V3, kind: "app", pr: 24 }),
    entry({ version: V1, kind: "ai", player: "The practice AI plays faster." }),
  ];

  it("R1369000 a patch shows the App and AI entries logged on it that have a player line, and no Internal one", async () => {
    renderPage(sourceFromData({ patches: FIXTURE_PATCHES, index: FIXTURE_INDEX, snapshots: FIXTURE_SNAPSHOTS, technical: LOG }));
    await screen.findAllByTestId(patchTestid.patch);
    openEverySet();
    await screen.findAllByTestId(patchTestid.technicalLog);
    const lines = (version: string): (string | null)[] =>
      within(patchEntry(version))
        .queryAllByTestId(patchTestid.technicalLog)
        .flatMap((log) => within(log).getAllByRole("listitem").map((item) => item.textContent));
    expect(lines(V3)).toEqual(["The main menu remembers its tab.", "The practice AI guards its Taunts."]);
    expect(lines(V1)).toEqual(["The practice AI plays faster."]);
    expect(lines(V2)).toEqual([]);
    expect(within(patchEntry(V3)).getByTestId(patchTestid.technicalLog)).toHaveTextContent(/^Technical changes/);
    expect(document.body).not.toHaveTextContent("A faster build.");
  });

  it("R1369000 the real log's player lines are on the page under their X", async () => {
    renderPage();
    const log = await realPatchSource.technical();
    await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    openEverySet();
    await screen.findAllByTestId(patchTestid.technicalLog, undefined, SLOW);
    for (const { version, kind, player } of log) {
      if (kind === "internal" || player === undefined) continue;
      expect(patchEntry(version)).toHaveTextContent(player);
    }
  }, 30_000);
});

describe("#2681 release artwork on the Patch notes page", () => {
  const HERE = dirname(fileURLToPath(import.meta.url));

  it("R3012001 shows a still poster for a major and art for a patch, and none for a micro patch", async () => {
    // V1 is a major, V2 and V3 patches, M1 and M2 micros under V2 (TIERED_PATCHES).
    const asked: string[] = [];
    renderPage(tieredSource((version) => asked.push(version)));
    await screen.findAllByTestId(patchTestid.patch);
    await screen.findByTestId(patchTestid.cards);

    const art = screen.getAllByTestId(patchTestid.releaseArt);
    expect(art.map((image) => image.dataset.version)).toEqual([V3, V2, V1]);
    for (const image of art) {
      const version = image.dataset.version ?? "";
      const major = version === V1;
      // One decorative, lazy image per entry, inside it and right after its heading.
      expect(within(patchEntry(version)).getAllByTestId(patchTestid.releaseArt), version).toEqual([image]);
      expect(image.tagName).toBe("IMG");
      expect(image.getAttribute("alt"), version).toBe("");
      expect(image.getAttribute("loading"), version).toBe("lazy");
      expect(image.getAttribute("decoding"), version).toBe("async");
      expect(image.getAttribute("draggable"), version).toBe("false");
      expect(image.dataset.major, version).toBe(major ? "true" : "false");
      // A major and a patch load their one file, a still picture: the major's `.svg` is its poster.
      expect(image.getAttribute("src"), version).toBe(releaseArtPath(version));
      expect(image.parentElement?.previousElementSibling?.tagName, version).toBe("HEADER");
      expect(image.parentElement?.getAttribute("style"), version).toBeNull();
    }
    expect(art.map((image) => image.getAttribute("src"))).toEqual(["/release-art/v5-third.svg", "/release-art/v1-second.svg", "/release-art/v9-first.svg"]);
    // No micro patch has any art inside its entry, and none of the page's images is a `-still` file.
    for (const micro of screen.getAllByTestId(patchTestid.micro)) {
      expect(within(micro).queryByTestId(patchTestid.releaseArt), micro.dataset.version).toBeNull();
      expect(micro.querySelector(".patch__art, img")).toBeNull();
    }
    expect(art.some((image) => (image.getAttribute("src") ?? "").includes("-still"))).toBe(false);

    // The art asks for no snapshot (V3's cards ask for its own and the one before it, a micro's), and the
    // release line and the patches keep their state (R507, R724000).
    expect(new Set(asked)).toEqual(new Set([V3, M2]));
    expect(screen.getAllByTestId(patchTestid.toggle).map((toggle) => toggle.getAttribute("aria-expanded"))).toEqual(["true", "false", "false"]);
    fireEvent.click(within(releaseSet("core")).getByTestId(patchTestid.setToggle));
    expect(screen.queryAllByTestId(patchTestid.releaseArt)).toEqual([]);
    fireEvent.click(within(releaseSet("core")).getByTestId(patchTestid.setToggle));
    expect(screen.getAllByTestId(patchTestid.releaseArt)).toHaveLength(3);
  });

  it("R3012001 an orphan micro patch, with no major or patch before it in its line, is an entry with no art", async () => {
    renderPage(fixtureSource());
    await screen.findAllByTestId(patchTestid.patch);
    expect(screen.getAllByTestId(patchTestid.patch).map((entry) => entry.dataset.tier)).toEqual(["micro", "micro", "micro"]);
    expect(screen.queryAllByTestId(patchTestid.releaseArt)).toEqual([]);
    expect(document.querySelector(".patch__art")).toBeNull();
  });

  it("R3012001 nothing on the page moves: the stylesheet has no keyframes and no animation", () => {
    const css = readFileSync(resolve(HERE, "patches.css"), "utf8");
    expect(css).not.toMatch(/@keyframes/u);
    expect(css).not.toMatch(/animation\s*:/u);
    expect(css).not.toMatch(/animation-name/u);
    expect(css).not.toMatch(/data-major/u);
  });

  it("R3012001 every major and patch of the real history has its art and no micro patch has any, a major's loading its one poster", async () => {
    renderPage();
    const shipped = await realPatchSource.patches();
    await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    openEverySet();
    const tierList = tiers(shipped);
    const art = screen.getAllByTestId(patchTestid.releaseArt);
    expect(art.map((image) => image.dataset.version)).toEqual(shipped.filter((_patch, at) => tierList[at] !== "micro").map((patch) => patch.version).reverse());
    expect(screen.getAllByTestId(patchTestid.patch)).toHaveLength(art.length);
    for (const image of art) {
      const at = shipped.findIndex((patch) => patch.version === image.dataset.version);
      expect(image.dataset.major, image.dataset.version).toBe(tierList[at] === "major" ? "true" : "false");
      expect(image.getAttribute("src"), image.dataset.version).toBe(releaseArtPath(image.dataset.version ?? ""));
    }
    for (const micro of screen.getAllByTestId(patchTestid.micro)) expect(within(micro).queryByTestId(patchTestid.releaseArt)).toBeNull();
    expect(art.filter((image) => image.dataset.major === "true").map((image) => image.dataset.version)).toEqual(expect.arrayContaining(["v0.3.0", "v0.4.0"]));
  }, 30_000);
});

describe("R3012000 micro patches nested under their patch", () => {
  const LINES: readonly Release[] = [
    { id: "older", name: "Older line", summary: "", technical: [] },
    { id: "newer", name: "Newer line", summary: "", technical: [] },
  ];

  it("R3012000 nests micros under the patch they follow, in order, with their own version, date and note", async () => {
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    // Patch B (V3) is its own entry; both micros sit inside patch A's entry (V2), and V1 holds none.
    const entries = screen.getAllByTestId(patchTestid.patch);
    expect(entries.map((entry) => entry.dataset.version)).toEqual([V3, V2, V1]);
    expect(entries.map((entry) => entry.dataset.tier)).toEqual(["patch", "patch", "major"]);
    const host = patchEntry(V2);
    const micros = within(host).getAllByTestId(patchTestid.micro);
    expect(micros.map((micro) => micro.dataset.version)).toEqual([M1, M2]);
    expect(screen.getAllByTestId(patchTestid.micro)).toHaveLength(2);
    expect(within(patchEntry(V3)).queryAllByTestId(patchTestid.micro)).toEqual([]);
    expect(within(patchEntry(V1)).queryAllByTestId(patchTestid.micro)).toEqual([]);
    expect(within(host).getByRole("list", { name: "Micro patches" })).toBeInTheDocument();
    const [first, second] = micros;
    expect(within(first ?? host).getByRole("heading", { level: 4 })).toHaveTextContent(`${M1} Knight wording`);
    expect(first).toHaveTextContent("2026-02-10");
    expect(first).toHaveTextContent("Test Knight reads clearer.");
    expect(first).not.toHaveTextContent("Quiet Golem stays quiet.");
    expect(within(second ?? host).getByRole("heading", { level: 4 })).toHaveTextContent(`${M2} Golem fix`);
    expect(second).toHaveTextContent("2026-02-17");
    expect(second).toHaveTextContent("Quiet Golem stays quiet.");
    // A micro's note headings sit one level under its title, and its entry is no entry of its own.
    expect(within(first ?? host).getByRole("heading", { level: 5 })).toHaveTextContent("Card text");
    expect(micros.every((micro) => micro.closest(`[data-testid="${patchTestid.patch}"]`) === host)).toBe(true);
  });

  it("R3012000 a micro after a major goes under the major", async () => {
    const patches = TIERED_PATCHES.filter((patch) => patch.version !== V2);
    const data = sourceFromData({ patches, index: FIXTURE_INDEX, snapshots: { ...FIXTURE_SNAPSHOTS, [M1]: FIXTURE_SNAPSHOTS[V2] ?? {}, [M2]: FIXTURE_SNAPSHOTS[V2] ?? {} } });
    renderPage(data);
    await screen.findAllByTestId(patchTestid.patch);
    expect(screen.getAllByTestId(patchTestid.patch).map((entry) => entry.dataset.version)).toEqual([V3, V1]);
    expect(within(patchEntry(V1)).getAllByTestId(patchTestid.micro).map((micro) => micro.dataset.version)).toEqual([M1, M2]);
  });

  it("R3012000 a micro keeps its own cards control, which loads its snapshot only when opened", async () => {
    // M1 changed the knight: its snapshot holds the knight as V1 had it, so its cards differ from V2's.
    const knight = FIXTURE_SNAPSHOTS[V1]?.["core-001"];
    const withCards: Patch[] = TIERED_PATCHES.map((patch) =>
      patch.version === M1 ? { ...patch, changes: [{ id: "core-001", name: "Test Knight", kind: "changed", fields: ["cost"] }] } : patch,
    );
    const asked: string[] = [];
    const data = sourceFromData({
      patches: withCards,
      index: FIXTURE_INDEX,
      snapshots: { ...FIXTURE_SNAPSHOTS, [M1]: { ...FIXTURE_SNAPSHOTS[V2], ...(knight === undefined ? {} : { "core-001": knight }) }, [M2]: FIXTURE_SNAPSHOTS[V2] ?? {} },
    });
    renderPage({
      ...data,
      snapshot: (version) => {
        asked.push(version);
        return data.snapshot(version);
      },
    });
    await screen.findByTestId(patchTestid.cards);
    expect(asked).not.toContain(M1);
    const micro = patchEntry(M1);
    const toggle = within(micro).getByTestId(patchTestid.toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(within(micro).queryByTestId(patchTestid.cards)).toBeNull();
    fireEvent.click(toggle);
    expect(await within(micro).findByTestId(patchTestid.cards)).toBeInTheDocument();
    expect(asked).toContain(M1);
    // M2 changed no card: a note and no control.
    expect(within(patchEntry(M2)).queryByTestId(patchTestid.toggle)).toBeNull();
    expect(patchEntry(M2)).toHaveTextContent("No card changed.");
  });

  it("R3012000 the legend lists every line, major and patch, with micros nested under their patch", async () => {
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    const legend = screen.getByRole("navigation", { name: "Patch notes legend" });
    expect(legend).toBe(screen.getByTestId(patchTestid.legend));
    expect(within(legend).getAllByTestId(patchTestid.legendLine).map((link) => link.dataset.set)).toEqual(["core"]);
    expect(within(legend).getAllByTestId(patchTestid.legendLine)[0]).toHaveTextContent("Core");
    const links = within(legend).getAllByTestId(patchTestid.legendLink);
    expect(links.map((link) => [link.dataset.version, link.dataset.tier])).toEqual([
      [V3, "patch"],
      [V2, "patch"],
      [M1, "micro"],
      [M2, "micro"],
      [V1, "major"],
    ]);
    for (const link of links) expect(link.getAttribute("href"), link.dataset.version).toBe(`#${encodeURIComponent(link.dataset.version ?? "")}`);
    // The micros are under V2's item, behind its button, and V3 and V1 hold none.
    const buttons = within(legend).getAllByTestId(patchTestid.legendMicros);
    expect(buttons.map((button) => [button.dataset.version, button.textContent])).toEqual([[V2, "2 micro patches"]]);
    const item = buttons[0]?.closest("li");
    expect(within(item ?? legend).getAllByTestId(patchTestid.legendLink).map((link) => link.dataset.version)).toEqual([V2, M1, M2]);
    expect(buttons[0]).toHaveAttribute("aria-expanded", "false");
    expect(buttons[0]?.getAttribute("aria-controls")).not.toBeNull();
    expect(document.getElementById(buttons[0]?.getAttribute("aria-controls") ?? "")).not.toBeVisible();
    fireEvent.click(buttons[0] ?? legend);
    expect(buttons[0]).toHaveAttribute("aria-expanded", "true");
    expect(document.getElementById(buttons[0]?.getAttribute("aria-controls") ?? "")).toBeVisible();
    // Every link names an entry that is on the page.
    for (const link of links) expect(document.getElementById(link.dataset.version ?? ""), link.dataset.version).not.toBeNull();
    expect(legend.getAttribute("aria-current")).toBeNull();
    expect(links.filter((link) => link.hasAttribute("aria-current"))).toEqual([]);
  });

  /** The versions the legend marks as in view (`aria-current`). */
  function marked(): (string | undefined)[] {
    return within(screen.getByTestId(patchTestid.legend))
      .getAllByTestId(patchTestid.legendLink)
      .filter((link) => link.getAttribute("aria-current") === "location")
      .map((link) => link.dataset.version);
  }

  /** Records the entries a jump scrolls to, as jsdom has no `scrollIntoView`. */
  function recordScrolls(): string[] {
    const targets: string[] = [];
    Object.defineProperty(Element.prototype, "scrollIntoView", {
      configurable: true,
      writable: true,
      value(this: Element) {
        targets.push(this.id);
      },
    });
    return targets;
  }

  /** The history a click or a hash has to open lines for: V1 and V3 are majors, so V1 and V2 sit in the older line, closed on arrival. */
  function twoLines(onSnapshot: (version: string) => void): PatchSource {
    const patches = TIERED_PATCHES.filter((patch) => patch.tier !== "micro").map((patch) => (patch.version === V3 ? { ...patch, tier: "major" as const } : patch));
    const data = sourceFromData({ patches, index: FIXTURE_INDEX, snapshots: FIXTURE_SNAPSHOTS });
    return {
      ...data,
      snapshot: (version) => {
        onSnapshot(version);
        return data.snapshot(version);
      },
    };
  }

  afterEach(() => {
    Reflect.deleteProperty(Element.prototype, "scrollIntoView");
  });

  it("R3012000 a legend click opens the entry's line, loads its snapshot as R507 does, scrolls to it and puts it in the address", async () => {
    const targets = recordScrolls();
    const asked: string[] = [];
    renderPage(twoLines((version) => asked.push(version)), LINES);
    await screen.findByTestId(patchTestid.cards);
    // V1 is in the older line, shut on arrival, and nothing has read its snapshot.
    expect(asked).not.toContain(V1);
    const older = releaseSet("older");
    expect(within(older).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "false");
    const legend = screen.getByTestId(patchTestid.legend);
    const linkTo = (version: string): HTMLElement => within(legend).getAllByTestId(patchTestid.legendLink).find((link) => link.dataset.version === version) ?? legend;

    fireEvent.click(linkTo(V1));
    expect(within(older).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "true");
    expect(await within(patchEntry(V1)).findByTestId(patchTestid.cards)).toBeInTheDocument();
    expect(asked).toContain(V1);
    expect(targets).toEqual([V1]);
    expect(window.location.hash).toBe(`#${V1}`);
    expect(patchEntry(V1)).toHaveFocus();
    // The entry just jumped to is the one marked in the legend.
    expect(marked()).toEqual([V1]);

    // Another entry of the same line opens its cards too, and a line's own link opens and scrolls to the line.
    fireEvent.click(linkTo(V2));
    expect(await within(patchEntry(V2)).findByTestId(patchTestid.cards)).toBeInTheDocument();
    expect(targets).toEqual([V1, V2]);
    expect(marked()).toEqual([V2]);
    fireEvent.click(within(older).getByTestId(patchTestid.setToggle));
    expect(within(older).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "false");
    const line = within(legend).getAllByTestId(patchTestid.legendLine).find((link) => link.dataset.set === "older") ?? legend;
    fireEvent.click(line);
    expect(within(older).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "true");
    expect(targets).toEqual([V1, V2, "line-older"]);
    expect(window.location.hash).toBe("#line-older");
    // A click with a modifier key is left to the browser: it opens and scrolls nothing here.
    fireEvent.click(within(older).getByTestId(patchTestid.setToggle));
    fireEvent.click(line, { ctrlKey: true });
    expect(within(older).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "false");
    expect(targets).toEqual([V1, V2, "line-older"]);
  });

  it("R3012000 a micro's legend link opens its host's line and scrolls to the micro", async () => {
    const targets = recordScrolls();
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    fireEvent.click(within(screen.getByTestId(patchTestid.legend)).getAllByTestId(patchTestid.legendLink).find((link) => link.dataset.version === M2) ?? document.body);
    expect(targets).toEqual([M2]);
    expect(patchEntry(M2)).toHaveFocus();
    expect(window.location.hash).toBe(`#${M2}`);
  });

  it("R3012000 an address hash opens and scrolls to its entry as a click does", async () => {
    const targets = recordScrolls();
    const asked: string[] = [];
    window.history.replaceState(null, "", `/patch-notes#${V1}`);
    renderPage(twoLines((version) => asked.push(version)), LINES);
    await screen.findAllByTestId(patchTestid.patch);
    expect(await within(patchEntry(V1)).findByTestId(patchTestid.cards)).toBeInTheDocument();
    expect(within(releaseSet("older")).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "true");
    expect(asked).toContain(V1);
    expect(targets).toEqual([V1]);
    expect(marked()).toEqual([V1]);
    cleanup();

    // A hash that names no entry, or one that is no valid escape, opens nothing and breaks nothing.
    for (const hash of ["#nobody", "#%E0%A4%A", "#"]) {
      window.history.replaceState(null, "", `/patch-notes${hash}`);
      renderPage(twoLines(() => undefined), LINES);
      await screen.findAllByTestId(patchTestid.patch);
      expect(within(releaseSet("older")).getByTestId(patchTestid.setToggle), hash).toHaveAttribute("aria-expanded", "false");
      cleanup();
    }
    expect(targets).toEqual([V1]);
  });

  it("R3012000 a hashchange does the same, and a hash in the address names a micro too", async () => {
    const targets = recordScrolls();
    renderPage(twoLines(() => undefined), LINES);
    await screen.findByTestId(patchTestid.cards);
    expect(within(releaseSet("older")).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "false");
    act(() => {
      window.history.replaceState(null, "", `/patch-notes#${V1}`);
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });
    expect(within(releaseSet("older")).getByTestId(patchTestid.setToggle)).toHaveAttribute("aria-expanded", "true");
    expect(targets).toEqual([V1]);
    cleanup();

    targets.length = 0;
    window.history.replaceState(null, "", `/patch-notes#${M1}`);
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    await waitFor(() => {
      expect(targets).toEqual([M1]);
    });
    expect(patchEntry(M1)).toHaveFocus();
  });

  /** An IntersectionObserver that records its observers, so a test can say what is in view. */
  class FakeObserver {
    static all: FakeObserver[] = [];
    readonly targets: Element[] = [];
    readonly root = null;
    readonly thresholds: readonly number[] = [];
    constructor(
      readonly callback: IntersectionObserverCallback,
      readonly options: IntersectionObserverInit | undefined,
    ) {
      FakeObserver.all.push(this);
    }
    get rootMargin(): string {
      return this.options?.rootMargin ?? "";
    }
    observe(target: Element): void {
      this.targets.push(target);
    }
    unobserve(): void {}
    disconnect(): void {
      this.targets.length = 0;
    }
    takeRecords(): IntersectionObserverEntry[] {
      return [];
    }
  }

  const seen = (target: Element, isIntersecting: boolean): IntersectionObserverEntry => ({ target, isIntersecting }) as IntersectionObserverEntry;

  it("R3012000 the entry in view carries aria-current in the legend, a micro winning over the patch holding it", async () => {
    FakeObserver.all = [];
    vi.stubGlobal("IntersectionObserver", FakeObserver);
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    // The page watches its entries once they are in the document (an effect, after the paint).
    await waitFor(() => {
      expect(FakeObserver.all.at(-1)?.targets).toHaveLength(5);
    });
    const observer = FakeObserver.all.at(-1);
    if (observer === undefined) throw new Error("the page watched nothing");
    expect(observer.rootMargin).toBe(PATCH_LEGEND_BAND_MARGIN);
    // Every entry is watched, micros included; nothing is marked until one is in view.
    expect(observer.targets.map((target) => (target as HTMLElement).dataset.entry).sort()).toEqual([V1, V2, V3, M1, M2].sort());
    expect(marked()).toEqual([]);

    const fire = (...records: IntersectionObserverEntry[]): void => {
      act(() => {
        observer.callback(records, observer as unknown as IntersectionObserver);
      });
    };
    fire(seen(patchEntry(V3), true));
    expect(marked()).toEqual([V3]);
    // V2 holds M1: both in the band, the micro is the one in view, and V2's micro list shows it.
    fire(seen(patchEntry(V3), false), seen(patchEntry(V2), true), seen(patchEntry(M1), true));
    expect(marked()).toEqual([M1]);
    const link = within(screen.getByTestId(patchTestid.legend)).getAllByTestId(patchTestid.legendLink).find((each) => each.dataset.version === M1);
    expect(link?.closest("ol")).toBeVisible();
    // Scrolling on leaves the last mark where it was until another entry is in view.
    fire(seen(patchEntry(V2), false), seen(patchEntry(M1), false));
    expect(marked()).toEqual([M1]);
    fire(seen(patchEntry(V1), true));
    expect(marked()).toEqual([V1]);
  });

  it("R3012000 at phone width the legend collapses into a drawer a link closes again", async () => {
    const restore = stubPatchLegendPhone(true);
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    const legend = screen.getByTestId(patchTestid.legend);
    const toggle = within(legend).getByTestId(patchTestid.legendToggle);
    expect(toggle).toHaveTextContent("Jump to a patch");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body).not.toBeNull();
    expect(body).not.toBeVisible();
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(body).toBeVisible();
    fireEvent.click(within(legend).getAllByTestId(patchTestid.legendLink).find((link) => link.dataset.version === V1) ?? legend);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(body).not.toBeVisible();
    restore();
    cleanup();

    // On a wide screen there is no drawer button, and the legend is always shown.
    const wide = stubPatchLegendPhone(false);
    renderPage(tieredSource());
    await screen.findAllByTestId(patchTestid.patch);
    expect(within(screen.getByTestId(patchTestid.legend)).queryByTestId(patchTestid.legendToggle)).toBeNull();
    expect(within(screen.getByTestId(patchTestid.legend)).getAllByTestId(patchTestid.legendLink)[0]).toBeVisible();
    wide();
  });

  it("R3012000 the stylesheet keeps the phone page from scrolling sideways, and the legend sticky", () => {
    const css = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), "patches.css"), "utf8");
    // The legend sticks to the top of the page on a wide screen.
    expect(/\.patch-legend \{[^}]*position: sticky;/u.exec(css)).not.toBeNull();
    // The phone block (the width `PATCH_LEGEND_PHONE_QUERY` names) is one column and wraps every long word.
    const phone = /@media \(max-width: 760px\) \{([\s\S]*?)\n\}/u.exec(css)?.[1] ?? "";
    expect(phone).toContain("grid-template-columns: minmax(0, 1fr)");
    expect(phone).toContain("overflow-wrap: anywhere");
    expect(phone).toContain("max-width: 100%");
    expect(phone).toMatch(/\.patch-legend \{[^}]*z-index: 3/u);
    expect(phone).toContain("overflow-y: auto");
    // Fingers get 44 px targets in the legend (R2122000).
    const touch = /@media \(pointer: coarse\), \(max-width: 600px\) \{([\s\S]*?)\n\}/gu;
    expect([...css.matchAll(touch)].some((block) => (block[1] ?? "").includes(".patch-legend__link") && (block[1] ?? "").includes("min-height: 44px"))).toBe(true);
  });
});
