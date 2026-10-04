// R388, R507: the History section of the collection's card detail view. Collapsed by default; opened,
// it loads only the snapshots the card needs and lists each patch that changed the card, newest
// first, with its faces as that patch left them and what changed marked. Fixtures through the
// source's context, then the real data through the default source.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardDetail } from "../cards/inspect/CardDetail.tsx";
import { INSPECT_DETAIL } from "../cards/inspect/testids.ts";
import { CardDefsProvider } from "../cards/refContext.tsx";
import { closeInspect } from "../cards/inspect/store.ts";
import { PatchSourceProvider } from "./context.tsx";
import { V1, V2, V3, fixtureDef, fixtureSource } from "./fixtures.ts";
import { EMPTY_PATCH_SOURCE, type PatchSource } from "./source.ts";
import { patchTestid } from "./testids.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
/** The real history's first read runs the JSON chunks through the transform. */
const SLOW = { timeout: 10_000 } as const;

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

function renderDetail(def: CardDef, source?: PatchSource, historyOpen?: boolean): void {
  const detail = <CardDetail def={def} onClose={() => undefined} {...(historyOpen === undefined ? {} : { historyOpen })} />;
  render(source === undefined ? detail : <PatchSourceProvider source={source}>{detail}</PatchSourceProvider>);
}

function openHistory(): void {
  fireEvent.click(screen.getByTestId(patchTestid.historyToggle));
}

function entry(version: string): HTMLElement {
  const found = screen.getAllByTestId(patchTestid.historyEntry).find((element) => element.dataset.version === version);
  if (found === undefined) throw new Error(`no history entry for ${version}`);
  return found;
}

function marks(root: Element, selector: string): string[] {
  return Array.from(root.querySelectorAll(selector)).map((element) => element.textContent ?? "");
}

describe("R388 the History section", () => {
  it("R388 is collapsed by default, under a History control, and loads nothing until opened", () => {
    const snapshots: string[] = [];
    const source = fixtureSource((version) => snapshots.push(version));
    const patches = vi.spyOn(source, "patches");
    renderDetail(fixtureDef(V2, "core-001"), source);
    const toggle = screen.getByTestId(patchTestid.historyToggle);
    expect(toggle).toHaveTextContent("History");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(document.getElementById(toggle.getAttribute("aria-controls") ?? "")).not.toBeVisible();
    expect(screen.queryAllByTestId(patchTestid.historyEntry)).toEqual([]);
    expect(patches).not.toHaveBeenCalled();
    expect(snapshots).toEqual([]);
  });

  it("R388 opened, lists each patch that changed the card newest first, reading only the snapshots it needs", async () => {
    const snapshots: string[] = [];
    renderDetail(fixtureDef(V2, "core-001"), fixtureSource((version) => snapshots.push(version)));
    openHistory();
    expect(screen.getByTestId(patchTestid.historyToggle)).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId(patchTestid.historyLoading)).toHaveTextContent("Loading this card’s history…");
    await screen.findAllByTestId(patchTestid.historyEntry);
    expect(screen.getAllByTestId(patchTestid.historyEntry).map((element) => [element.dataset.version, element.dataset.kind])).toEqual([
      [V2, "changed"],
      [V1, "added"],
    ]);
    expect(snapshots.sort()).toEqual([V1, V2].sort());
    const newest = entry(V2);
    expect(newest).toHaveTextContent(V2);
    expect(newest).toHaveTextContent("2026-02-03");
    expect(newest).toHaveTextContent("The knight grows");
  });

  it("R388 prints each entry's faces as that patch left them, from its snapshot", async () => {
    renderDetail(fixtureDef(V2, "core-001"), fixtureSource(), true);
    await screen.findAllByTestId(patchTestid.historyEntry);
    const gems = (version: string): (string | undefined)[] =>
      Array.from(entry(version).querySelectorAll<HTMLElement>(".cost-gem")).map((gem) => gem.dataset.cost);
    expect(gems(V2)).toEqual(["3", "3"]);
    expect(gems(V1)).toEqual(["2", "2"]);
    const faces = entry(V1).querySelectorAll(".cf");
    expect(faces[0]).not.toHaveAttribute("data-radiant-face");
    expect(faces[1]).toHaveAttribute("data-radiant-face", "true");
    expect(entry(V1).querySelector("[data-face-attack]")).toHaveAttribute("data-face-attack", "2");
  });

  it("R388 R507 marks the words and numbers a patch added, and strikes through the ones it took away", async () => {
    renderDetail(fixtureDef(V3, "core-002"), fixtureSource(), true);
    await screen.findAllByTestId(patchTestid.historyEntry);
    const newest = entry(V3);
    const base = within(newest).getAllByTestId(patchTestid.change).find((line) => line.dataset.field === "base.text");
    if (base === undefined) throw new Error("expected a base text change");
    expect(marks(base, "ins.patch-mark")).toEqual(["4", "Your"]);
    expect(marks(base, "del.patch-cut")).toEqual(["3", "this turn"]);
    expect(base).toHaveTextContent("Was");
    const cost = within(newest).getAllByTestId(patchTestid.change).find((line) => line.dataset.field === "cost");
    expect(cost).toHaveTextContent("Cost(1) Cost → becomes (2) Cost, embiggen (4)");
    expect(entry(V1)).toHaveTextContent("Added in this patch.");
  });

  it("R388 a card only its first patch touched says \"Unchanged since\" that version", async () => {
    renderDetail(fixtureDef(V3, "classic-001"), fixtureSource(), true);
    expect(await screen.findByTestId(patchTestid.historyUnchanged)).toHaveTextContent(`Unchanged since ${V3}.`);
    expect(screen.queryAllByTestId(patchTestid.historyEntry)).toEqual([]);
  });

  it("R388 with no patch data it says there are no patch notes yet", async () => {
    renderDetail(fixtureDef(V2, "core-001"), EMPTY_PATCH_SOURCE, true);
    expect(await screen.findByTestId(patchTestid.historyEmpty)).toHaveTextContent("There are no patch notes yet.");
  });

  it("R388 a history that fails to load says so and tries again", async () => {
    const source = fixtureSource();
    let failures = 1;
    const index = source.index;
    const flaky: PatchSource = {
      ...source,
      index: () => (failures-- > 0 ? Promise.reject(new Error("offline")) : index()),
    };
    const quiet = vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderDetail(fixtureDef(V2, "core-001"), flaky, true);
    expect(await screen.findByTestId(patchTestid.historyError)).toHaveTextContent("This card’s history didn’t load.");
    fireEvent.click(screen.getByTestId(patchTestid.historyRetry));
    expect(await screen.findAllByTestId(patchTestid.historyEntry)).toHaveLength(2);
    quiet.mockRestore();
  });

  it("R388 an old face's references are marks, never controls, while the current faces' stay controls (R279)", async () => {
    const def = CATALOG["core-041"];
    if (def === undefined) throw new Error("expected core-041");
    render(
      <CardDefsProvider defs={CATALOG}>
        <CardDetail def={def} onClose={() => undefined} historyOpen />
      </CardDefsProvider>,
    );
    await screen.findAllByTestId(patchTestid.historyEntry, undefined, SLOW);
    const history = screen.getByTestId(patchTestid.history);
    // Sheepish names the Sheep Token and the Lava Golem on every face it ever printed.
    expect(history.querySelectorAll(".cf-ref").length).toBeGreaterThan(0);
    expect(history.querySelectorAll(".cf-ref[tabindex='0']")).toHaveLength(0);
    expect(screen.getByTestId(INSPECT_DETAIL).querySelectorAll(".cf-ref[tabindex='0']").length).toBeGreaterThan(0);
  });
});

describe("R388 the History section over the real history", () => {
  it("R388 Masochism Mask's history opens on v0.2.11's removal of Animated, then v0.2.10's Animated face, then v0.2.0's new cost, (2) Cost to (1) Cost, and ends where it was added", async () => {
    const def = CATALOG["core-065"];
    if (def === undefined) throw new Error("expected core-065");
    renderDetail(def);
    openHistory();
    const entries = await screen.findAllByTestId(patchTestid.historyEntry, undefined, SLOW);
    expect(entries.map((element) => element.dataset.version)).toEqual(["v0.2.11", "v0.2.10", "v0.2.0", "v0.1.1", "v0.1.0d", "v0.1.0"]);
    const cost = within(entry("v0.2.0")).getAllByTestId(patchTestid.change).find((line) => line.dataset.field === "cost");
    expect(cost).toHaveTextContent("(2) Cost → becomes (1) Cost");
    expect(entry("v0.1.0").dataset.kind).toBe("added");
  });

  it("R388 a Classic card says it is unchanged since v0.2.0", async () => {
    const def = CATALOG["classic-001"];
    if (def === undefined) throw new Error("expected classic-001");
    renderDetail(def);
    openHistory();
    expect(await screen.findByTestId(patchTestid.historyUnchanged, undefined, SLOW)).toHaveTextContent("Unchanged since v0.2.0.");
  });
});

describe("R507 the patch mark", () => {
  it("R507 is bold with a double underline in its own teal, and a removal is struck through, in the stylesheet the section loads", () => {
    const css = readFileSync(resolve(HERE, "patches.css"), "utf8");
    const mark = /\.patch-mark\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(mark).toMatch(/font-weight:\s*800/);
    expect(mark).toMatch(/text-decoration:\s*underline double/);
    expect(mark).toMatch(/color:\s*var\(--patch-mark-ink\)/);
    const cut = /\.patch-cut\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(cut).toMatch(/text-decoration:\s*line-through/);
    // Not R277's Radiant gold, and not R279's dotted reference line.
    const cards = readFileSync(resolve(HERE, "../cards/cards.css"), "utf8");
    const gold = /--mark-ink:\s*([^;]+);/.exec(cards)?.[1];
    const teal = /--patch-mark-ink:\s*([^;]+);/.exec(css)?.[1];
    expect(teal).toBeDefined();
    expect(teal).not.toBe(gold);
    expect(mark).not.toMatch(/dotted/);
    // A screen reader hears what each mark is.
    expect(css).toMatch(/\.patch-mark::before\s*\{\s*content:\s*" new: ";/);
    expect(css).toMatch(/\.patch-cut::before\s*\{\s*content:\s*" removed: ";/);
  });
});
