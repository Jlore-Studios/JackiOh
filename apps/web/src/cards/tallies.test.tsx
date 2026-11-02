// R2321000: a count a card on the field keeps on a Unit (`UnitView.tallies`, Training Grounds #31
// Level Up!'s attacks toward its Radiant) is drawn on the Unit as "▲ 2/3", named by the keeping card,
// on both seats, and the inspect notes spell it out for a touch player.

import { CATALOG } from "@jackioh/cards";
import type { PlayerView, TallyView, UnitView } from "@jackioh/shared";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import { liveFace } from "../game/faces.ts";
import { renderBoard } from "../test/board.tsx";
import { fullBoardView } from "../test/fixtures.ts";
import { StateNotes, hasStateNotes } from "./inspect/StateNotes.tsx";
import { talliesTestid } from "./Tallies.tsx";
import { TALLY_GLYPH, tallyShort, tallyWords } from "./tallies.ts";

afterEach(() => {
  cleanup();
});

const LEVEL_UP = "traininggrounds-031";
const lookup = lookupFromDefs(CATALOG);

/** fullBoardView with `tallies` on the viewer's lane-1 unit and the enemy's lane-1 unit. */
function countedView(tallies: TallyView[]): { view: PlayerView; ids: { mine: string; theirs: string; plain: string } } {
  const view = fullBoardView();
  const mine = view.you.units[0] as UnitView;
  const theirs = view.opponent.units[0] as UnitView;
  const plain = view.you.units.find((unit) => unit !== null && unit !== mine) as UnitView;
  const next: PlayerView = {
    ...view,
    you: { ...view.you, units: view.you.units.map((unit) => (unit === mine ? { ...unit, tallies } : unit)) },
    opponent: { ...view.opponent, units: view.opponent.units.map((unit) => (unit === theirs ? { ...unit, tallies } : unit)) },
  };
  return { view: next, ids: { mine: mine.instanceId, theirs: theirs.instanceId, plain: plain.instanceId } };
}

describe("R2321000 a count kept on a unit", () => {
  it("R2321000 draws each count on the unit it counts, on either seat, by the keeping card's name", () => {
    const { view, ids } = countedView([{ defId: LEVEL_UP, value: 2, goal: 3 }]);
    const root = renderBoard(view);
    for (const id of [ids.mine, ids.theirs]) {
      const card = root.querySelector(`[data-testid="${testid.card(id)}"]`);
      const pills = card?.querySelector(`[data-testid="${talliesTestid(id)}"]`);
      expect(pills, id).not.toBeNull();
      const pill = pills?.querySelector(".tally-badge");
      expect(pill).toHaveAttribute("data-value", "2");
      expect(pill).toHaveAttribute("data-goal", "3");
      expect(pill).toHaveAttribute("title", "Level Up!: 2 of 3");
      expect(pill).toHaveTextContent(`${TALLY_GLYPH}2/3`);
    }
    // A unit nothing counts wears nothing.
    expect(root.querySelector(`[data-testid="${talliesTestid(ids.plain)}"]`)).toBeNull();
  });

  it("R2321000 draws two keepers' counts apart, and marks a count that reached its goal", () => {
    const { view, ids } = countedView([
      { defId: LEVEL_UP, value: 1, goal: 3 },
      { defId: LEVEL_UP, value: 2, goal: 2 },
    ]);
    const root = renderBoard(view);
    const pills = Array.from(root.querySelectorAll(`[data-testid="${talliesTestid(ids.mine)}"] .tally-badge`));
    expect(pills.map((pill) => pill.getAttribute("title"))).toEqual(["Level Up!: 1 of 3", "Level Up!: 2 of 2"]);
    expect(pills.map((pill) => pill.getAttribute("data-reached"))).toEqual([null, "true"]);
  });

  it("R2321000 the inspect notes spell the count out, which a touch screen's missing hover cannot read", () => {
    const { view, ids } = countedView([{ defId: LEVEL_UP, value: 2, goal: 3 }]);
    const unit = view.you.units.find((entry) => entry?.instanceId === ids.mine) as UnitView;
    const tallies = [{ name: "Level Up!", value: 2, goal: 3 }];
    const face = liveFace(lookup(unit.defId, unit.radiant) ?? { name: unit.defId, type: "Unit", text: "", tags: [] }, unit, { tallies });
    expect(face.tallies).toEqual(tallies);
    expect(hasStateNotes(face)).toBe(true);
    const { container } = render(<StateNotes face={face} />);
    const note = container.querySelector('[data-state="tally"]');
    expect(note).toHaveAttribute("data-value", "2");
    expect(note).toHaveTextContent("Level Up!: 2 of 3");
    // No count, no note.
    expect(liveFace(lookup(unit.defId, unit.radiant) ?? { name: unit.defId, type: "Unit", text: "", tags: [] }, unit).tallies).toBeUndefined();
  });

  it("R2321000 words a count short and long", () => {
    const tally = { name: "Level Up!", value: 1, goal: 2 };
    expect(tallyShort(tally)).toBe("1/2");
    expect(tallyWords(tally)).toBe("Level Up!: 1 of 2");
  });
});
