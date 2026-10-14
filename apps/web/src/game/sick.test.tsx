// R849000: a summoning-sick unit wears a zzz (`UnitView.sick`), and your own unit that may act but
// cannot attack says why (`UnitView.attackReason`): in its tooltip, in the inspect notes and, when you
// click it and `legal` offers no attack, in a notice on the board. The client reads both fields off the
// view and works neither out; whether an attack is offered stays `legal`'s answer (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PlayerView, UnitView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/index.ts";
import { SICK_WORDS } from "../cards/MinionFace.tsx";
import { StateNotes, hasStateNotes } from "../cards/inspect/StateNotes.tsx";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { IDLE, onClickTarget } from "./actions.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { liveFace } from "./faces.ts";
import Game from "./Game.tsx";

afterEach(() => {
  cleanup();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
});

const lookup = lookupFromDefs(CATALOG);
const ASLEEP = "that unit is summoning sick";

/** u1 is asleep and says so; u2 may attack; u3 is in Defense Position (no reason given); e1 is asleep too. */
function sickView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { instanceId: "u1", sick: true, attackReason: ASLEEP }),
        unit("p1", { instanceId: "u2" }),
        unit("p1", { instanceId: "u3", position: "DEF" }),
        null,
        null,
      ],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1", sick: true }), null, null, null, null],
    }),
  });
}

/** u2 attacks e1 or the hero; every unit of yours may switch, a sick one included (§4.1). */
const LEGAL: readonly ActionBody[] = [
  { type: "attack", attackerId: "u2", targetId: "e1" },
  { type: "attack", attackerId: "u2", targetId: "hero-p2" },
  { type: "switchPosition", instanceId: "u1" },
  { type: "switchPosition", instanceId: "u2" },
  { type: "switchPosition", instanceId: "u3" },
  { type: "endTurn" },
  { type: "concede" },
];

const el = (id: string): HTMLElement => screen.getByTestId(id);

function unitOf(view: PlayerView, id: string): UnitView {
  const found = view.you.units.find((candidate) => candidate?.instanceId === id);
  if (found === null || found === undefined) throw new Error(`the fixture has no unit ${id}`);
  return found;
}

describe("R849000 a summoning-sick unit wears a zzz, and your own unit says why it cannot attack", () => {
  it("R849000 a unit the view marks sick renders the zzz on either seat, and one it does not mark renders none", () => {
    render(<Board view={sickView()} />);

    for (const id of ["card-u1", "card-e1"]) {
      const mark = el(id).querySelector('[data-sick="true"]');
      expect(mark, id).not.toBeNull();
      expect(mark).toHaveAttribute("aria-label", SICK_WORDS);
    }
    expect(el("card-u2").querySelector("[data-sick]")).toBeNull();
    expect(el("card-u3").querySelector("[data-sick]")).toBeNull();
  });

  it("R849000 clicking your own unit with an attackReason and no legal attack returns the reason and sends nothing", () => {
    const view = sickView();
    const click = (instanceId: string, lane: number, legal: readonly ActionBody[] = LEGAL) =>
      onClickTarget(view, legal, IDLE, { on: "unit", instanceId, side: "you", lane });

    expect(click("u1", 1)).toEqual({ interaction: IDLE, notice: ASLEEP });

    const attacker = click("u2", 2);
    expect(attacker.interaction.stage).toBe("attacking");
    expect(attacker.notice).toBeUndefined();

    // A unit with no reason and no attack stays silent, as it always did.
    expect(click("u3", 3)).toEqual({ interaction: IDLE });

    // `legal` decides: the same unit with an attack on offer is picked up, with no notice.
    const withAttack: readonly ActionBody[] = [...LEGAL, { type: "attack", attackerId: "u1", targetId: "e1" }];
    const picked = click("u1", 1, withAttack);
    expect(picked.interaction.stage).toBe("attacking");
    expect(picked.notice).toBeUndefined();
  });

  it("R849000 the board shows the reason as a notice after the click, sends nothing, and drops it on the next click", () => {
    const onAction = vi.fn<(body: ActionBody) => void>();
    render(<Game view={sickView()} legal={LEGAL} onAction={onAction} />);
    expect(screen.queryByTestId("attack-reason")).toBeNull();

    fireEvent.click(el("card-u1"));
    expect(el("attack-reason")).toHaveTextContent(ASLEEP);
    expect(onAction).not.toHaveBeenCalled();

    fireEvent.click(el("card-u3"));
    expect(screen.queryByTestId("attack-reason")).toBeNull();
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R849000 the unit's native tooltip names the card and the reason, and a unit with none shows the name alone", () => {
    writeCardSettings({ hoverPreviews: false });
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={sickView()} />
      </CatalogContext.Provider>,
    );

    const name = CATALOG["core-004"]?.name;
    expect(el("card-u1")).toHaveAttribute("title", `${name}: ${ASLEEP}`);
    expect(el("card-u2")).toHaveAttribute("title", name);
  });

  it("R849000 the inspect notes spell out the reason, and a unit without one has no such note", () => {
    const view = sickView();
    const faceOf = (unitView: UnitView) =>
      liveFace(lookup(unitView.defId, unitView.radiant) ?? { name: unitView.defId, type: "Unit", text: "", tags: [] }, unitView);

    const face = faceOf(unitOf(view, "u1"));
    expect(face.attackReason).toBe(ASLEEP);
    expect(hasStateNotes(face)).toBe(true);
    const { container } = render(<StateNotes face={face} />);
    expect(container.querySelector('[data-state="attackReason"]')).toHaveTextContent(ASLEEP);

    expect(faceOf(unitOf(view, "u2")).attackReason).toBeUndefined();
  });
});
