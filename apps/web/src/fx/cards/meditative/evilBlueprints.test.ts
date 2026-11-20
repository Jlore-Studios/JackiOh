// R3405001: Jlockeed's Evil Blueprints' signature (evilBlueprints.ts), past what signatures.spells.test.ts
// holds: an entry with no takeover hold before it locks the blueprint from the cast's start; it keys on its
// own cast alone, so a card played inside its play (which the dispatch asks it about when that card's own
// recipe claims nothing, R3389000) and another event naming it keep their own rows; and its gallery
// fixture's building, added to the caster's hand.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { castOf, played, run, tid } from "../../../test/signatures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { FX_CENTER } from "../../constants.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { blueprintsFixture } from "./evilBlueprints.ts";

const BLUEPRINTS = "meditative-097";
/** A Unit whose effects entry is a motif and no recipe, played inside its play. */
const PLAIN = "core-004";
const D = 1000;
const UNCLAIMED = { cues: [], row: undefined };

describe("R3405001 Jlockeed's Evil Blueprints' signature, its own cast alone", () => {
  it("R3405000 with no takeover hold before its cast, the blueprint locks from the entry's start and outlasts it by 400 ms", () => {
    const cast = castOf(BLUEPRINTS, D, 0);
    expect(cast.row).toBe("replace");
    expect(cast.cues).toMatchObject([
      { kind: "figure", figure: "blueprint", at: tid(testid.board, FX_CENTER), delayMs: 0, durationMs: D + 400 },
      { kind: "burst", at: tid(testid.board, FX_CENTER), delayMs: 0 },
    ]);
  });

  it("R3389000 a card played inside its play, and an event naming it that is no cast, keep their own rows", () => {
    expect(FX_CARD_TABLE.get(PLAIN)?.kind).not.toBe("bespoke");
    const naming: GameEvent = { type: "costChanged", instanceId: "ex", cost: 0 };
    expect(run(BLUEPRINTS, [naming, played("p1", "fx-inner", PLAIN)], D, "p1", fullBoardView())).toEqual([UNCLAIMED, UNCLAIMED]);
  });

  it("R3405001 its gallery fixture adds the building picked to the caster's hand, folding off the board's centre into their hand pile", () => {
    const view = fullBoardView();
    expect(blueprintsFixture(view, "ex", "p1")).toEqual([{ type: "addedToHand", player: "p1", instanceId: "fx-building", defId: "meditative-097-1" }]);
    expect(blueprintsFixture(view, "ex", "p2")).toEqual([{ type: "addedToHand", player: "p2", instanceId: "fx-building", defId: "meditative-097-1" }]);
    expect(CATALOG["meditative-097-1"]?.name).toBe("Empty Plot");
    const [building] = run(BLUEPRINTS, blueprintsFixture(view, "ex", "p1"), D, "p1", view);
    expect(building?.cues[0]).toMatchObject({ kind: "ghost", from: tid(testid.board, FX_CENTER), to: tid(animTestid.hand("you")) });
  });
});
