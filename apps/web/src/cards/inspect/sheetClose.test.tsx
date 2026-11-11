// R3221000 (issue #3377): the inspect overlay's modal sheets (a card's touch sheet, a pile's list, a
// face-down card and the Stack's wheel) close by the one ✕ every dialog closes by, `DialogClose` at the
// sheet's top right, named "Close": it holds the focus on open and stays last in the sheet, so Tab ends
// on it as it ended on the text Close it replaces.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import { must } from "../../test/stubServer.ts";
import { faceModel } from "../model.ts";
import StackSheet, { stackTestid } from "../wheel/StackSheet.tsx";
import { CardListSheet } from "./CardList.tsx";
import { FaceDownSheet } from "./FaceDown.tsx";
import { InspectSheet } from "./InspectSheet.tsx";
import { INSPECT_CLOSE, INSPECT_FACE_DOWN, INSPECT_LIST_BACK, INSPECT_LIST_CARD, INSPECT_LIST_SHEET, INSPECT_SHEET } from "./testids.ts";

const lookup = lookupFromDefs(CATALOG);

function face(defId: string) {
  return faceModel({ defId, def: must(CATALOG[defId], defId), radiant: false });
}

function pileEntries() {
  return [
    { key: "e1", face: face("core-002") },
    { key: "e2", face: face("core-019") },
  ];
}

function renderSheet(sheet: ReactElement): void {
  render(<CatalogContext.Provider value={lookup}>{sheet}</CatalogContext.Provider>);
}

/** The sheet's close: the shared ✕, named Close, focused and last in the sheet. */
function expectSharedClose(sheetTestid: string): HTMLElement {
  const close = screen.getByTestId(INSPECT_CLOSE);
  expect(close).toHaveClass("dialog__close");
  expect(close).toHaveAccessibleName("Close");
  expect(close).toHaveTextContent("✕");
  expect(close).toHaveFocus();
  expect(screen.getByTestId(sheetTestid).lastElementChild).toBe(close);
  return close;
}

afterEach(cleanup);

const SHEETS: readonly { what: string; testid: string; sheet: (onClose: () => void) => ReactElement }[] = [
  { what: "a card's touch sheet", testid: INSPECT_SHEET, sheet: (onClose) => <InspectSheet face={face("core-002")} onClose={onClose} /> },
  {
    what: "a pile's list",
    testid: INSPECT_LIST_SHEET,
    sheet: (onClose) => <CardListSheet title="Your graveyard" entries={pileEntries()} onClose={onClose} />,
  },
  { what: "a face-down card's sheet", testid: INSPECT_FACE_DOWN, sheet: (onClose) => <FaceDownSheet cost={2} onClose={onClose} /> },
  {
    what: "the Stack's wheel",
    testid: stackTestid.sheet,
    sheet: (onClose) => <StackSheet title="Stack pile" top={face("core-004")} topName="Top" buried={2} onClose={onClose} />,
  },
];

describe("R3221000 the inspect overlay's sheets close by the shared ✕", () => {
  for (const { what, testid, sheet } of SHEETS) {
    it(`R3221000 ${what} closes by the ✕ at its top right, named Close`, () => {
      const onClose = vi.fn();
      renderSheet(sheet(onClose));
      const close = expectSharedClose(testid);
      fireEvent.click(close);
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  }

  it("R3221000 a pile's list draws its actions row only for a face opened large, the ✕ still last", () => {
    renderSheet(<CardListSheet title="Your graveyard" entries={pileEntries()} onClose={vi.fn()} />);
    const sheet = screen.getByTestId(INSPECT_LIST_SHEET);
    expect(sheet.querySelector(".inspect-list-actions")).toBeNull();

    fireEvent.click(must(screen.getAllByTestId(INSPECT_LIST_CARD)[0], "the list's first face"));

    const back = screen.getByTestId(INSPECT_LIST_BACK);
    expect(back).toHaveFocus();
    expect(back.closest(".inspect-list-actions")).not.toBeNull();
    expect(sheet.lastElementChild).toBe(screen.getByTestId(INSPECT_CLOSE));
  });
});
