// R5050000 (issue #5050): a unit holding gear, a Miner or a Weapon Equipped into it (Radi, Halo, Radix, …), is
// drawn as a Starship is (R2249000): named for the card it was equipped into, its rules box that card's own text,
// then the gear, oldest first, each a reference that opens the piece's card. The face model decides it
// (`crates/shared/src/face/fused.rs` `equipped_of`); the web lays it out (StarshipPieces.tsx `EquippedGear`).

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardFace } from "./CardFace.tsx";
import { faceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { EQUIPPED_GEAR_TESTID, STARSHIP_PIECES_TESTID } from "./StarshipPieces.tsx";
import { fusedDef } from "../test/fixtures.ts";

afterEach(cleanup);

const RADI = "stinkyscapeplus-060";
const REPEATER = "stinkyscape-051";
const BEAM = "stinkyscape-050";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

describe("R5050000: a unit holding gear is drawn as a Starship is", () => {
  it("R5050000 Radi with two Weapons equipped keeps its name and text and lists them oldest first", () => {
    // The engine fuses each pick onto the unit again: [newest, (older + host)].
    const armed = fusedDef([def(BEAM), fusedDef([def(REPEATER), def(RADI)], 1)], 2);
    expect(armed.name).toBe("Beam v3 + Repeater v3 + Radi");
    const face = faceModel({ defId: armed.id, def: armed, radiant: false, inPlay: {} });
    expect(face.name).toBe("Radi");
    expect(face.equipped?.list.map((piece) => piece.defId)).toEqual([REPEATER, BEAM]);
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <CardFace face={face} layout="full" />
      </CardDefsProvider>,
    );
    expect(container.querySelector(".cf-text-base")?.textContent).toContain("Equip them into this");
    const gear = screen.getByTestId(EQUIPPED_GEAR_TESTID);
    expect(gear.getAttribute("data-host")).toBe(RADI);
    expect(gear.querySelector('[data-term="Equip"]')).not.toBeNull();
    expect([...gear.querySelectorAll("[data-piece]")].map((node) => node.getAttribute("data-piece"))).toEqual([REPEATER, BEAM]);
    expect(within(gear).getByText(def(REPEATER).name)).toBeTruthy();
    // The Weapons' own texts are behind their references, not on the face.
    expect(container.textContent).not.toContain("with Cleave and Pierce");
    expect(screen.queryByTestId(STARSHIP_PIECES_TESTID)).toBeNull();
  });

  it("R5050000 any card that holds no gear lists none", () => {
    const joined = fusedDef([def("core-011"), def("core-020")]);
    for (const face of [
      faceModel({ defId: joined.id, def: joined, radiant: false, inPlay: {} }),
      faceModel({ defId: RADI, def: def(RADI), radiant: false, inPlay: {} }),
    ]) {
      expect(face.equipped).toBeUndefined();
      render(
        <CardDefsProvider defs={CATALOG}>
          <CardFace face={face} layout="full" />
        </CardDefsProvider>,
      );
      expect(screen.queryByTestId(EQUIPPED_GEAR_TESTID)).toBeNull();
      cleanup();
    }
  });
});
