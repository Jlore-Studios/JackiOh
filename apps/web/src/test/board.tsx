// The board tests' shared mount (#2215). Test support only; helpers.test.ts fails a test file that defines
// its own `withCatalog` or `renderBoard`.

import { render } from "@testing-library/react";
import type { ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs, type CardLookup } from "../game/catalog.ts";
import type { BoardProps } from "../game/contract.ts";

const SHIPPED: CardLookup = lookupFromDefs(CATALOG);

/** `node` inside a `CatalogContext` holding `catalog`, the shipped catalog by default. */
export function withCatalog(node: ReactElement, catalog: CardLookup = SHIPPED): ReactElement {
  return <CatalogContext.Provider value={catalog}>{node}</CatalogContext.Provider>;
}

/** Renders `<Board>` on `view` with `props` inside the shipped catalog; returns the container. */
export function renderBoard(view: PlayerView, props: Omit<BoardProps, "view"> = {}): HTMLElement {
  return render(withCatalog(<Board view={view} {...props} />)).container;
}
