// R1955000: a reveal's Equip for the cosmetic a slot granted. R2949000: Signature's, only on a card wearing the Full
// art version and Animated, which it needs.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { readCardPicks, resetCosmeticsForTests, writeCardVersion } from "../cards/cosmetics.ts";
import { AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, VERSION_FULL_ART_ID } from "../cards/versions.ts";
import EquipButton from "./EquipButton.tsx";

/** Carnivorous Cube, an Epic Unit offered the Full art version, Animated and Signature (R2725001, R2949001). */
const CUBE = "core-022";
const TESTID = "equip";

function clearStorage(): void {
  try {
    window.localStorage.clear();
  } catch {
    // Storage is optional.
  }
}

beforeEach(() => {
  clearStorage();
  resetCosmeticsForTests();
});

afterEach(() => {
  cleanup();
  clearStorage();
  resetCosmeticsForTests();
});

describe("R1955000 a revealed slot's Equip", () => {
  it("R2949000 a granted Signature offers Equip only on a card wearing Full art and Animated", () => {
    const signature = (): HTMLElement | null => screen.queryByTestId(TESTID);
    const wear = (fullArt: boolean | string, augments: readonly string[]): void => {
      act(() => {
        writeCardVersion(CUBE, fullArt, augments);
      });
    };
    render(<EquipButton cosmeticId={AUGMENT_SIGNATURE_ID} cardId={CUBE} testid={TESTID} />);
    // The normal face: no Equip.
    expect(signature()).toBeNull();
    // Full art alone, then Animated alone on the normal face: still none.
    wear(VERSION_FULL_ART_ID, []);
    expect(signature()).toBeNull();
    wear(false, [AUGMENT_ANIMATED_ID]);
    expect(signature()).toBeNull();
    // Both worn: Equip layers Signature beside them.
    wear(VERSION_FULL_ART_ID, [AUGMENT_ANIMATED_ID]);
    const equip = screen.getByTestId(TESTID);
    expect(equip).toHaveTextContent("Equip");
    fireEvent.click(equip);
    expect(readCardPicks(CUBE)).toEqual({ fullArt: VERSION_FULL_ART_ID, augments: [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID] });
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equipped");
    // Animated's Equip is there on any face it is offered.
    cleanup();
    wear(false, []);
    render(<EquipButton cosmeticId={AUGMENT_ANIMATED_ID} cardId={CUBE} testid={TESTID} />);
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equip");
  });
});
