// R375: the detail view's History section, over the real patch list and snapshots.
//
// Collapsed by default with its version count; opened, it loads the snapshots (the loader is
// wrapped here only to count and to fail on request) and lists the versions newest first, the
// creation last; each entry can draw the card as that version left it. The versions up to v0.1.1
// are fixed, so each test pins those and reads any later ones off changes.json.

import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { loadSnapshots, RECONSTRUCTED_NOTE, versionCount } from "../patches.ts";
import { versionsLabel } from "./History.tsx";
import { CardDetail, closeInspect } from "./index.ts";
import {
  INSPECT_HISTORY,
  INSPECT_HISTORY_BADGE,
  INSPECT_HISTORY_ENTRY,
  INSPECT_HISTORY_FACES,
  INSPECT_HISTORY_SHOW,
  INSPECT_HISTORY_TOGGLE,
} from "./testids.ts";

const failures = vi.hoisted(() => ({ left: 0 }));

vi.mock("../patches.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../patches.ts")>();
  return {
    ...actual,
    loadSnapshots: vi.fn(() => {
      if (failures.left > 0) {
        failures.left -= 1;
        return Promise.reject(new Error("the network is down"));
      }
      return actual.loadSnapshots();
    }),
  };
});

/** Loading every snapshot can outrun the 1 s default when the whole suite runs at once. */
const SLOW = { timeout: 5_000 } as const;

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`expected ${id} in the catalog`);
  return def;
}

function open(id: string): void {
  render(<CardDetail def={defOf(id)} onClose={() => undefined} />);
}

async function openHistory(id: string): Promise<HTMLElement[]> {
  open(id);
  await userEvent.click(screen.getByTestId(INSPECT_HISTORY_TOGGLE));
  return screen.findAllByTestId(INSPECT_HISTORY_ENTRY, undefined, SLOW);
}

function entryFor(version: string): HTMLElement {
  const entry = screen.getAllByTestId(INSPECT_HISTORY_ENTRY).find((element) => element.dataset["version"] === version);
  if (entry === undefined) throw new Error(`no ${version} entry`);
  return entry;
}

function changeIn(entry: HTMLElement, field: string): HTMLElement {
  const change = entry.querySelector<HTMLElement>(`[data-field="${field}"]`);
  if (change === null) throw new Error(`no ${field} change in ${entry.dataset["version"] ?? "?"}`);
  return change;
}

function textsOf(root: HTMLElement, selector: string): string[] {
  return Array.from(root.querySelectorAll(selector)).map((element) => element.textContent);
}

beforeEach(() => {
  failures.left = 0;
  vi.mocked(loadSnapshots).mockClear();
});

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

describe("R375 the card detail's History", () => {
  it("R375 is collapsed by default, labelled with its version count, and loads nothing until opened", () => {
    expect(versionsLabel(1)).toBe("1 version");
    expect(versionsLabel(3)).toBe("3 versions");
    open("core-044");
    const toggle = screen.getByTestId(INSPECT_HISTORY_TOGGLE);
    expect(toggle).toHaveTextContent(`History (${versionsLabel(versionCount("core-044"))})`);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByTestId(INSPECT_HISTORY)).toHaveAttribute("data-open", "false");
    expect(screen.queryAllByTestId(INSPECT_HISTORY_ENTRY)).toHaveLength(0);
    expect(loadSnapshots).not.toHaveBeenCalled();
  });

  it("R375 opens onto every version newest first, the creation last, reconstructed ones badged", async () => {
    const entries = await openHistory("core-044");
    expect(loadSnapshots).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId(INSPECT_HISTORY_TOGGLE)).toHaveAttribute("aria-expanded", "true");
    expect(entries).toHaveLength(versionCount("core-044"));
    // Up to v0.1.1, newest first; a later patch's entries come before these.
    const recorded = entries.slice(-3);
    expect(recorded.map((entry) => entry.dataset["version"])).toEqual(["v0.1.1", "v0.1.0-r3", "v0.1.0"]);
    expect(entries.map((entry) => entry.dataset["kind"])).toEqual([...entries.slice(0, -1).map(() => "changed"), "created"]);

    const [v011, r3, created] = recorded;
    if (v011 === undefined || r3 === undefined || created === undefined) throw new Error("three entries");
    expect(v011).toHaveTextContent("v0.1.1 · 2026-09-27");
    expect(v011).toHaveTextContent("Patch v0.1.1");
    expect(within(v011).queryByTestId(INSPECT_HISTORY_BADGE)).toBeNull();
    expect(within(r3).getByTestId(INSPECT_HISTORY_BADGE)).toHaveTextContent("Reconstructed");
    expect(within(r3).getByTestId(INSPECT_HISTORY_BADGE)).toHaveAttribute("title", RECONSTRUCTED_NOTE);
    expect(created.querySelector(".inspect-history-head")).toHaveTextContent(/^Created in v0\.1\.0 · 2026-09-18/);
    expect(within(created).getByTestId(INSPECT_HISTORY_BADGE)).toBeInTheDocument();
    expect(screen.getByTestId(INSPECT_HISTORY)).toHaveTextContent("reconstructed from the repository's history");
  });

  it("R375 prints a field as old → new", async () => {
    await openHistory("core-085");
    expect(changeIn(entryFor("v0.1.1"), "cost")).toHaveTextContent("Cost 1 → 2");
    cleanup();

    await openHistory("core-068");
    expect(changeIn(entryFor("v0.1.0-r1"), "name")).toHaveTextContent("Name Twisted Sourcerer → Twisted Sorcerer");
  });

  it("R375 word-diffs a text, the words removed struck through and the words added underlined", async () => {
    await openHistory("core-068");
    const text = changeIn(entryFor("v0.1.1"), "text");
    expect(textsOf(text, "del.inspect-history-del")).toEqual(["is below"]);
    expect(textsOf(text, "ins.inspect-history-ins")).toEqual(["or", "has less than", "health"]);
    expect(text.querySelector(".inspect-history-before")).toHaveTextContent(
      "Cry: deal 4 damage to a target, 8 if your hero is below 10",
    );
    expect(text.querySelector(".inspect-history-after")).toHaveTextContent(
      "Cry: Deal 4 damage to a target, or 8 if your hero has less than 10 health.",
    );
  });

  it("R375 says so when only capitals, punctuation or line breaks changed, which mark no word", async () => {
    await openHistory("core-001");
    const text = changeIn(entryFor("v0.1.1"), "text");
    expect(text).toHaveAttribute("data-layout-only", "true");
    expect(text).toHaveTextContent("Text, capitals, punctuation or line breaks only");
    expect(text.querySelectorAll("del, ins")).toHaveLength(0);
    expect(changeIn(entryFor("v0.1.1"), "stats")).toHaveTextContent("Stats 0/8 → 0/7");
  });

  it("R375 labels a shorthand Radiant text, and says it was written out in full instead of diffing it", async () => {
    await openHistory("core-044");
    const radiant = changeIn(entryFor("v0.1.0-r3"), "radiantText");
    expect(radiant).toHaveAttribute("data-written-out", "true");
    expect(radiant).toHaveTextContent("Radiant text, written out in full");
    expect(radiant.querySelector(".inspect-history-before")).toHaveTextContent("9 (shorthand)");
    expect(radiant.querySelector(".inspect-history-after")).toHaveTextContent("Deal 9 damage to a target, ignoring Armor; exile this");
    expect(radiant.querySelectorAll("del, ins")).toHaveLength(0);
  });

  it("R375 draws the card as a version left it, from that version's snapshot", async () => {
    await openHistory("core-068");
    const created = entryFor("v0.1.0");
    expect(within(created).queryByTestId(INSPECT_HISTORY_FACES)).toBeNull();

    const show = within(created).getByTestId(INSPECT_HISTORY_SHOW);
    expect(show).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(show);
    expect(show).toHaveAttribute("aria-expanded", "true");
    const faces = within(created).getByTestId(INSPECT_HISTORY_FACES);
    expect(faces).toHaveAttribute("data-version", "v0.1.0");
    const drawn = Array.from(faces.querySelectorAll<HTMLElement>(".cf"));
    expect(drawn).toHaveLength(2);
    for (const face of drawn) expect(face.querySelector(".card-name")).toHaveTextContent("Twisted Sourcerer");
    expect(drawn[1]).toHaveAttribute("data-radiant-face", "true");
    expect(drawn[1]).toHaveTextContent("6, or 12");
    expect(faces).toHaveTextContent("Radiant (shorthand)");

    await userEvent.click(show);
    expect(within(created).queryByTestId(INSPECT_HISTORY_FACES)).toBeNull();
  });

  it("R375 says so when the history fails to load, and loads it on a retry", async () => {
    failures.left = 1;
    open("core-044");
    await userEvent.click(screen.getByTestId(INSPECT_HISTORY_TOGGLE));
    const alert = await screen.findByRole("alert", undefined, SLOW);
    expect(alert).toHaveTextContent("The history didn’t load.");
    await userEvent.click(within(alert).getByRole("button", { name: "Try again" }));
    expect(await screen.findAllByTestId(INSPECT_HISTORY_ENTRY, undefined, SLOW)).toHaveLength(versionCount("core-044"));
    expect(loadSnapshots).toHaveBeenCalledTimes(2);
  });

  it("R375 closes again, and reopens without loading twice", async () => {
    await openHistory("core-044");
    const toggle = screen.getByTestId(INSPECT_HISTORY_TOGGLE);
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByTestId(INSPECT_HISTORY)).toHaveAttribute("data-open", "false");
    await userEvent.click(toggle);
    await waitFor(() => {
      expect(screen.getAllByTestId(INSPECT_HISTORY_ENTRY)).toHaveLength(versionCount("core-044"));
    });
    expect(loadSnapshots).toHaveBeenCalledTimes(1);
  });

  it("R375 shows no History for a definition no version holds", () => {
    render(<CardDetail def={{ ...defOf("core-044"), id: "t-1" }} onClose={() => undefined} />);
    expect(screen.queryByTestId(INSPECT_HISTORY)).toBeNull();
  });
});
