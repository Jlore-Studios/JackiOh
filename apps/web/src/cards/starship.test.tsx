// R2249000: a Starship's face in play (#2249) wears the Starship's own art, drawn from its pieces and its
// launch, whatever definition its fusions gave it; its rules box prints only the Starship term and the
// pieces fused into it, each a reference that opens the piece's card. R2801000: a Dormant one bolts on
// a hull plate for each piece fused into it. R3038000: Starship is a tag, and the real engine's view
// lists a Starship's Flying and Ranged, Dormant or launched, which the board draws as any Unit's.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { Action, ActionBody, CardDef, PlayerId, PlayerView, UnitView } from "@jackioh/shared";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { CardArt } from "./art/CardArt.tsx";
import { starshipSvg } from "./art/stinkyscape.ts";
import { starshipArtUri } from "./art/svg.ts";
import { CardFace } from "./CardFace.tsx";
import { faceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { STARSHIP_PIECES_TESTID } from "./StarshipPieces.tsx";
import { STARSHIP_HULL_PLATES_MAX } from "./StarshipHull.tsx";
import { renderBoard } from "../test/board.tsx";
import { baseView, dormantStarship, emptySide, unit } from "../test/fixtures.ts";

afterEach(cleanup);

const HULL = "stinkyscape-t-dormant-starship";
const SHIELD = "stinkyscape-038";
const CAPSULE = "stinkyscape-043";
const AP_ROUNDS = "stinkyscape-100";
const NETHER = "core-088";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

function art(defId: string, starship: { pieces: number; launched: boolean } | null): HTMLElement {
  const card = def(defId);
  const { container } = render(<CardArt defId={defId} radiant={false} tags={card.tags} type={card.type} shape="oval" starship={starship} />);
  const found = container.querySelector<HTMLElement>(".cf-art");
  if (found === null) throw new Error("no art");
  return found;
}

describe("R2249000: a Starship's face", () => {
  it("R2249000 a Starship wears its own art, the same whatever definition its fusions gave it", () => {
    const hull = art(HULL, { pieces: 3, launched: false });
    expect(hull.getAttribute("data-art-family")).toBe("starship");
    const hullArt = hull.style.backgroundImage;
    cleanup();
    // Fused, its definition is not the hull's; its art stays the Starship's.
    const fused = art("core-008", { pieces: 3, launched: false });
    expect(fused.getAttribute("data-art-family")).toBe("starship");
    expect(fused.style.backgroundImage).toBe(hullArt);
    expect(hullArt).toContain(starshipArtUri(3, false, false));
  });

  it("R2249000 the art grows with its pieces and lights up once it launches", () => {
    const dormant = starshipSvg(2, false, false);
    expect(starshipSvg(5, false, false)).not.toBe(dormant);
    expect(starshipSvg(2, true, false)).not.toBe(dormant);
    expect(starshipSvg(2, false, true)).not.toBe(dormant);
    expect(starshipSvg(2, false, false)).toBe(dormant);
  });

  it("R2249000 a card that is no Starship keeps its own art", () => {
    expect(art(SHIELD, null).getAttribute("data-art-family")).not.toBe("starship");
  });

  it("R2249000 in play a Starship prints no text, only the Starship term and its pieces, each a card reference", () => {
    const pieces = [
      { defId: SHIELD, radiant: false },
      { defId: CAPSULE, radiant: true },
    ];
    const face = faceModel({
      defId: HULL,
      def: def(HULL),
      radiant: false,
      inPlay: { starship: { pieces: pieces.length, name: null, list: pieces } },
    });
    expect(face.text.full).toBe("");
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <CardFace face={face} layout="full" />
      </CardDefsProvider>,
    );
    expect(container.querySelector(".cf-text-base")).toBeNull();
    const list = screen.getByTestId(STARSHIP_PIECES_TESTID);
    expect(list.getAttribute("data-pieces")).toBe("2");
    expect(within(list).getByText(def(SHIELD).name)).toBeTruthy();
    expect(within(list).getByText(def(CAPSULE).name)).toBeTruthy();
    expect(list.querySelector('[data-term="Starship"]')).not.toBeNull();
    expect([...list.querySelectorAll("[data-piece]")].map((node) => node.getAttribute("data-piece"))).toEqual([SHIELD, CAPSULE]);
    // The Starship's printed text is nowhere on the face.
    expect(container.textContent).not.toContain("launch this");
  });

  it("R2801000 a Dormant hull shows min(pieces, STARSHIP_HULL_PLATES_MAX) plates; a launched one none", () => {
    const platesOf = (pieces: number, launched = false): number | null => {
      const list = Array.from({ length: pieces }, () => SHIELD);
      const ship = launched
        ? unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: list.map((defId) => ({ defId })), name: "Ember of Departure", launched: 1 } })
        : dormantStarship("p1", list, { instanceId: "ship" });
      const container = renderBoard(baseView({ you: emptySide("p1", { units: [null, null, ship, null, null] }) }));
      const hull = container.querySelector('[data-testid="card-ship"] .cf-starship-hull');
      const plates = hull === null ? null : hull.querySelectorAll(".cf-starship-plate").length;
      if (hull !== null && plates !== null) {
        expect(hull.getAttribute("data-plates")).toBe(String(plates));
        expect(hull.getAttribute("aria-hidden")).toBe("true");
        // Port and starboard in turn, as the art's modules are.
        expect(hull.querySelectorAll('[data-flank="port"] .cf-starship-plate')).toHaveLength(Math.ceil(plates / 2));
        expect(hull.querySelectorAll('[data-flank="starboard"] .cf-starship-plate')).toHaveLength(Math.floor(plates / 2));
      }
      cleanup();
      return plates;
    };
    // One plate a piece, the fuse's bolt-on, up to the most a face bolts on.
    expect(platesOf(1)).toBe(1);
    expect(platesOf(3)).toBe(3);
    expect(platesOf(STARSHIP_HULL_PLATES_MAX + 4)).toBe(STARSHIP_HULL_PLATES_MAX);
    // No pieces yet, or launched: none.
    expect(platesOf(0)).toBeNull();
    expect(platesOf(3, true)).toBeNull();
  });

  it("R2249000 out of play, in the collection, the Dormant Starship prints its short text", () => {
    const face = faceModel({ defId: HULL, def: def(HULL), radiant: false });
    expect(face.text.full).toBe(
      "Flying, Ranged\nDormant until it launches. Your Starship Pieces that die fuse into this.\nActivate: Pay (4) to launch this.",
    );
  });
});

/** A side's Units, standing in its unit zones or on its backrow (R2249000: a Starship stands there first). */
function unitsOf(side: PlayerView["you"]): UnitView[] {
  const backrow = side.backrow.flatMap((card) => (card !== null && !card.faceDown && card.unit !== undefined ? [card.unit] : []));
  return [...side.units.flatMap((unit) => (unit === null ? [] : [unit])), ...backrow];
}

/** p1's Starship as the real engine shows it to p1: the Unit with a Starship's memory (R1852005). */
function shipOf(view: PlayerView): UnitView {
  const found = unitsOf(view.you).find((unit) => unit.starship !== undefined);
  if (found === undefined) throw new Error("p1 has no Starship");
  return found;
}

/** The kinds of the keyword marks the board draws on `id`. */
function marksOn(view: PlayerView, id: string): string[] {
  const container = renderBoard(view);
  const card = container.querySelector(`[data-testid="card-${id}"]`);
  if (card === null) throw new Error(`the board draws no ${id}`);
  const kinds = [...card.querySelectorAll("[data-keyword]")].map((node) => node.getAttribute("data-keyword") ?? "");
  cleanup();
  return kinds;
}

describe("R3038000: a Starship's Flying and Ranged on the board", () => {
  it("R3038000 the real engine's view lists Flying and Ranged on a Dormant and a launched Starship, and the board draws both", () => {
    const port = realPort();
    const cheap = devDeck("cheap20");
    let state = port.beginGame(port.createGame({ seed: "r3038000-icons", decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
    let nonce = 0;
    const act = (player: PlayerId, body: ActionBody): void => {
      nonce += 1;
      const result = port.reduce(state, { ...body, playerId: player, nonce: `r3038000-${String(nonce)}` } as Action);
      if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
      state = result.state;
    };
    const toMain = (): void => {
      for (let step = 0; step < 40; step += 1) {
        const view = port.viewFor(state, "p1");
        if (view.active === "p1" && view.phase === "main" && view.pending === null) return;
        const who = (["p1", "p2"] as const).find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
        if (who === undefined) return;
        const legal = port.legalActions(state, who);
        const hand = port.viewFor(state, who).you.hand;
        if (legal.some((a) => a.type === "mulligan")) act(who, { type: "mulligan", keep: Array.isArray(hand) ? hand.map((c) => c.instanceId) : [] });
        else act(who, legal.find((a) => a.type === "answer") ?? { type: "endTurn" });
      }
    };
    const sandbox = (op: unknown): void => {
      act("p1", { type: "sandbox", op } as unknown as ActionBody);
    };
    toMain();
    sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
    sandbox({ kind: "spawn", player: "p1", defId: AP_ROUNDS, to: { zone: "field", row: "units", lane: 1 } });
    sandbox({ kind: "spawn", player: "p1", defId: NETHER, to: { zone: "hand" } });
    // Nether kills the piece, which fuses into the Dormant Starship the pass summons (R1852000).
    const hand = port.viewFor(state, "p1").you.hand;
    const nether = Array.isArray(hand) ? hand.find((c) => c.defId === NETHER) : undefined;
    const play = port.legalActions(state, "p1").find((a) => a.type === "play" && a.instanceId === nether?.instanceId);
    if (play === undefined) throw new Error("no play of Nether");
    act("p1", play);

    const dormant = shipOf(port.viewFor(state, "p1"));
    expect(dormant.slumber).toBeDefined();
    expect(dormant.keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
    expect(dormant.keywords.map((k) => k.kind)).not.toContain("Starship");
    expect(marksOn(port.viewFor(state, "p1"), dormant.instanceId)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
    // The other seat's view lists them too.
    const seen = unitsOf(port.viewFor(state, "p2").opponent).find((unit) => unit.instanceId === dormant.instanceId);
    expect(seen?.keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));

    // Launched by its Activate (R1852003), it keeps both and takes no Starship keyword (R3038001).
    const findLaunch = () => port.legalActions(state, "p1").find((a) => a.type === "activate" && a.ability === "launch");
    if (findLaunch() === undefined) {
      act("p1", { type: "endTurn" });
      toMain();
    }
    sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
    const launch = findLaunch();
    if (launch === undefined) throw new Error("no launch to activate");
    act("p1", launch);
    toMain();
    const launched = shipOf(port.viewFor(state, "p1"));
    expect(launched.instanceId).toBe(dormant.instanceId);
    expect(launched.slumber).toBeUndefined();
    expect(launched.keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
    expect(launched.keywords.map((k) => k.kind)).not.toContain("Starship");
    expect(marksOn(port.viewFor(state, "p1"), launched.instanceId)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
  });
});
