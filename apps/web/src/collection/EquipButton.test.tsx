// R1955000: a reveal's Equip for the cosmetic a slot granted. R2949000, R3015005 (#3015): a level's Equip wears that one
// level on its card; one granted before #3015 (a retired full art or augment) the level it maps to, and Signature alone,
// which maps to none, has none.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CARD_PICKS_SETTINGS_KEY, readCardPicks, resetCosmeticsForTests, writeCardLevel } from "../cards/cosmetics.ts";
import { FULL_ARTS_OF_CARD } from "../cards/fullArt.ts";
import { AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, VERSION_ANIMATED_ID, VERSION_FULL_ART_ID, VERSION_FULL_ART_SIGNED_ID } from "../cards/versions.ts";
import EquipButton from "./EquipButton.tsx";

/** Carnivorous Cube, an Epic Unit offered every level of Full art and the Default's animated one (R2725001, R3015000). */
const CUBE = "core-022";
/** Mr. Vanilla, one of the 21, whose retired Mythic full art maps to Full art's signed level (R3015006). */
const VANILLA = "core-008";
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
  it("R2949000 R3015005 a granted level's Equip wears that one level, and a granted Signature, which names no level, offers none", () => {
    const equip = (): HTMLElement | null => screen.queryByTestId(TESTID);
    render(<EquipButton cosmeticId={VERSION_FULL_ART_SIGNED_ID} cardId={CUBE} testid={TESTID} />);
    // Full art's signed level: Equip, on any face the card wears, wears it alone.
    expect(equip()).toHaveTextContent("Equip");
    fireEvent.click(screen.getByTestId(TESTID));
    expect(readCardPicks(CUBE)).toEqual({ fullArt: VERSION_FULL_ART_SIGNED_ID });
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equipped");
    // Another level worn: Equip again.
    act(() => {
      writeCardLevel(CUBE, VERSION_FULL_ART_ID);
    });
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equip");
    expect(screen.getByTestId(TESTID)).not.toHaveTextContent("Equipped");
    cleanup();

    // A pick of #2725's shape that names the level (the Full art version, Animated and Signature) holds it already.
    window.localStorage.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify({ [CUBE]: { fullArt: VERSION_FULL_ART_ID, augments: [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID] } }));
    resetCosmeticsForTests();
    render(<EquipButton cosmeticId={VERSION_FULL_ART_SIGNED_ID} cardId={CUBE} testid={TESTID} />);
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equipped");
    cleanup();

    // A Signature granted before #3015 maps to no level (R3015006): no Equip, whatever the card wears.
    render(<EquipButton cosmeticId={AUGMENT_SIGNATURE_ID} cardId={CUBE} testid={TESTID} />);
    expect(equip()).toBeNull();
    act(() => {
      writeCardLevel(CUBE, VERSION_FULL_ART_SIGNED_ID);
    });
    expect(equip()).toBeNull();
    cleanup();

    // The Animated augment granted before #3015: Equip wears the Default's animated level.
    act(() => {
      writeCardLevel(CUBE, null);
    });
    render(<EquipButton cosmeticId={AUGMENT_ANIMATED_ID} cardId={CUBE} testid={TESTID} />);
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equip");
    fireEvent.click(screen.getByTestId(TESTID));
    expect(readCardPicks(CUBE)).toEqual({ fullArt: VERSION_ANIMATED_ID });
    cleanup();

    // A retired Mythic full art: Equip wears Full art's signed level on its own card.
    const mythic = FULL_ARTS_OF_CARD.get(VANILLA)?.get("Mythic") ?? "";
    render(<EquipButton cosmeticId={mythic} cardId={VANILLA} testid={TESTID} />);
    fireEvent.click(screen.getByTestId(TESTID));
    expect(readCardPicks(VANILLA)).toEqual({ fullArt: VERSION_FULL_ART_SIGNED_ID });
    expect(screen.getByTestId(TESTID)).toHaveTextContent("Equipped");
  });
});
