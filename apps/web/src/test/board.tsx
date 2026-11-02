// The board tests' shared mount (#2215): helpers.test.ts fails a test file that defines its own
// `withCatalog` or `renderBoard`.

import { render } from "@testing-library/react";
import type { ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs, type CardLookup } from "../game/catalog.ts";
import type { BoardProps } from "../game/contract.ts";

const SHIPPED: CardLookup = lookupFromDefs(CATALOG);

export function withCatalog(node: ReactElement, catalog: CardLookup = SHIPPED): ReactElement {
  return <CatalogContext.Provider value={catalog}>{node}</CatalogContext.Provider>;
}

/** `<Board>` on `view` with `props` in the shipped catalog; returns the container. */
export function renderBoard(view: PlayerView, props: Omit<BoardProps, "view"> = {}): HTMLElement {
  return render(withCatalog(<Board view={view} {...props} />)).container;
}
