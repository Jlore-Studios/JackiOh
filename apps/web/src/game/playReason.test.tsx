// R979000: a card in your hand that cannot be played says why (`CardView.playReason`): in its
// tooltip, in the inspect notes and, when you click it and `legal` offers no play, in a notice on
// the board. The client reads the field off the view and works nothing out (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, CardView, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CARD_SETTINGS_DEFAULTS, closeInspect, writeCardSettings } from "../cards/index.ts";
import { StateNotes, hasStateNotes } from "../cards/inspect/StateNotes.tsx";
import { baseView, card, emptySide } from "../test/fixtures.ts";
import { IDLE, onClickTarget } from "./actions.ts";
import Board from "./Board.tsx";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import { listedFace, liveFace } from "./faces.ts";
import Game from "./Game.tsx";

afterEach(() => {
  cleanup();
  closeInspect();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
});

const lookup = lookupFromDefs(CATALOG);
const REASON = "you can't play (3)+ Cost cards now";

/** c1 cannot be played and says why; c2 may be played. */
function reasonView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "c1", defId: "core-001", playReason: REASON }),
        card({ instanceId: "c2", defId: "core-001" }),
      ],
    }),
  });
}

/** Only c2 is offered a play. */
const LEGAL: readonly ActionBody[] = [
  { type: "play", instanceId: "c2" },
  { type: "endTurn" },
  { type: "concede" },
];

const el = (id: string): HTMLElement => screen.getByTestId(id);

function handCard(view: PlayerView, id: string): CardView {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) throw new Error("the fixture holds its hand as cards");
  const found = hand.find((candidate) => candidate.instanceId === id);
  if (found === undefined) throw new Error(`the fixture has no hand card ${id}`);
  return found;
}

describe("R979000 a card in hand that cannot be played says why", () => {
  it("R979000 clicking an unplayable hand card shows its playReason as a notice and sends nothing", () => {
    const view = reasonView();
    const click = (instanceId: string, legal: readonly ActionBody[] = LEGAL) =>
      onClickTarget(view, legal, IDLE, { on: "hand", instanceId });

    expect(click("c1")).toEqual({ interaction: IDLE, notice: REASON });

    // c2's one bare play is ready at once, so the click sends it with no notice.
    const playable = click("c2");
    expect(playable).toEqual({ interaction: IDLE, action: { type: "play", instanceId: "c2" } });

    // A hand card with no reason and no play stays silent, as it always did.
    const bare = reasonView();
    bare.you.hand = Array.isArray(bare.you.hand)
      ? bare.you.hand.map((entry) =>
          entry.instanceId === "c1" ? { ...entry, playReason: undefined } : entry,
        )
      : bare.you.hand;
    expect(onClickTarget(bare, LEGAL, IDLE, { on: "hand", instanceId: "c1" })).toEqual({
      interaction: IDLE,
    });

    // `legal` decides: the same card with plays on offer is picked up, with no notice.
    const withPlay: readonly ActionBody[] = [
      ...LEGAL,
      { type: "play", instanceId: "c1", x: 1 },
      { type: "play", instanceId: "c1", x: 2 },
    ];
    const picked = click("c1", withPlay);
    expect(picked.interaction.stage).toBe("playing");
    expect(picked.notice).toBeUndefined();
  });

  it("R979000 the board shows the reason as a notice after the click, sends nothing, and drops it on the next click", () => {
    const onAction = vi.fn<(body: ActionBody) => void>();
    render(<Game view={reasonView()} legal={LEGAL} onAction={onAction} />);
    expect(screen.queryByTestId("attack-reason")).toBeNull();

    // c1 stays greyed out, as `legal` offers it no play (`data-legal` mirrors `legalActions`, which
    // e2e reads), but its click still reaches the board for its reason.
    expect(el("hand-card-c1")).toHaveAttribute("data-legal", "false");
    expect(el("hand-card-c1")).not.toHaveAttribute("data-glow");
    expect(el("hand-card-c2")).toHaveAttribute("data-legal", "true");
    fireEvent.click(el("hand-card-c1"));
    expect(el("attack-reason")).toHaveTextContent(REASON);
    expect(onAction).not.toHaveBeenCalled();

    fireEvent.click(el("hand-card-c2"));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(screen.queryByTestId("attack-reason")).toBeNull();
  });

  it("R979000 a hand card's playReason renders in its tooltip and inspect notes", () => {
    writeCardSettings({ hoverPreviews: false });
    render(
      <CatalogContext.Provider value={lookup}>
        <Board view={reasonView()} />
      </CatalogContext.Provider>,
    );

    const name = CATALOG["core-001"]?.name;
    expect(el("hand-card-c1")).toHaveAttribute("title", `${name}: ${REASON}`);
    expect(el("hand-card-c2")).toHaveAttribute("title", name);
  });

  it("R979000 a hand card's playReason renders in its inspect notes, and one without it has no such note", () => {
    const view = reasonView();
    const faceOf = (hand: CardView) =>
      liveFace(lookup(hand.defId, hand.radiant) ?? { name: hand.defId, type: "Unit", text: "", tags: [] }, hand);

    const face = faceOf(handCard(view, "c1"));
    expect(face.playReason).toBe(REASON);
    expect(hasStateNotes(face)).toBe(true);
    const { container } = render(<StateNotes face={face} />);
    expect(container.querySelector('[data-state="playReason"]')).toHaveTextContent(REASON);

    expect(faceOf(handCard(view, "c2")).playReason).toBeUndefined();
  });
});

describe("R1571000 a card in your graveyard that cannot be played says why", () => {
  const GY_REASON = "that card costs 3, more than your mana";

  /** gy1 cannot be played and says why; gy2 may be played. */
  function graveyardView(): PlayerView {
    return baseView({
      you: emptySide("p1", {
        graveyard: [
          card({ instanceId: "gy1", defId: "core-001", playReason: GY_REASON }),
          card({ instanceId: "gy2", defId: "core-004" }),
        ],
      }),
    });
  }

  /** Only gy2 is offered a play. */
  const GY_LEGAL: readonly ActionBody[] = [
    { type: "play", instanceId: "gy2" },
    { type: "endTurn" },
    { type: "concede" },
  ];

  function gyCard(view: PlayerView, id: string): CardView {
    const found = view.you.graveyard.find((candidate) => candidate.instanceId === id);
    if (found === undefined) throw new Error(`the fixture has no graveyard card ${id}`);
    return found;
  }

  function tileIn(sheet: HTMLElement, key: string): HTMLElement {
    const tile = within(sheet)
      .getAllByTestId("inspect-list-card")
      .find((element) => element.getAttribute("data-entry-key") === key);
    if (tile === undefined) throw new Error(`${key} is not in the sheet`);
    return tile;
  }

  it("R1571000 the graveyard's sheet shows the reason where Play would be, in the grid and opened large, and sends nothing", () => {
    const onAction = vi.fn<(body: ActionBody) => void>();
    render(
      <CatalogContext.Provider value={lookup}>
        <Game view={graveyardView()} legal={GY_LEGAL} onAction={onAction} />
      </CatalogContext.Provider>,
    );

    fireEvent.click(el(testid.graveyard("you")));
    const sheet = el("inspect-list-sheet");

    // gy1 offers no play, so where "Play" would be it says why; gy2 keeps its button.
    expect(within(sheet).queryByTestId(testid.pilePlay("gy1"))).toBeNull();
    const gy1 = tileIn(sheet, "gy1");
    expect(gy1.parentElement).toHaveAttribute("data-playable", "false");
    expect(gy1.parentElement?.querySelector('[data-state="playReason"]')).toHaveTextContent(GY_REASON);
    expect(within(sheet).getByTestId(testid.pilePlay("gy2"))).toHaveTextContent("Play");
    const gy2 = tileIn(sheet, "gy2");
    expect(gy2.parentElement?.querySelector('[data-state="playReason"]')).toBeNull();

    // Opening gy1 large shows the reason beside the missing Play, and clicks send nothing.
    fireEvent.click(gy1);
    expect(el("inspect-list-detail")).toBeInTheDocument();
    expect(
      document.querySelector('.inspect-list-actions [data-state="playReason"]'),
    ).toHaveTextContent(GY_REASON);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("R1571000 a graveyard card's playReason reaches its face, whose inspect notes spell it out", () => {
    const view = graveyardView();

    const face = listedFace(lookup, view, gyCard(view, "gy1"));
    if (face === null) throw new Error("gy1 has no face");
    expect(face.playReason).toBe(GY_REASON);
    expect(hasStateNotes(face)).toBe(true);
    const { container } = render(<StateNotes face={face} />);
    expect(container.querySelector('[data-state="playReason"]')).toHaveTextContent(GY_REASON);

    expect(listedFace(lookup, view, gyCard(view, "gy2"))?.playReason).toBeUndefined();
  });
});
