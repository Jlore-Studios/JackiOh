// Issue #3734 (R3734000): the Decks tab: every saved deck as a tile (its hero portrait, its name, how many cards it holds,
// its validity and the modes it can play), each a link that opens the deck in the workshop (`?deck=`), and a New deck
// tile. A deck's validity and its modes are the validator's verdicts (`deckVerdict`, `trioVerdict`), never the client's
// (CLAUDE.md rule 7); ownership is `GET /api/collection`, and with none the verdict is "Complete", as the workshop's rail
// says. The workshop at `/decks` is where a deck is edited.

import type { ReactElement } from "react";

import { portraitOrDefault } from "@jackioh/shared";

import { CardDefsProvider } from "../cards/index.ts";
import { PortraitArt } from "../emotes/PortraitArt.tsx";
import { currentCatalog } from "../game/currentCatalog.ts";
import { DECK_SIZE } from "../game/deckbuilder/deckSize.ts";
import { collectionFrom } from "../game/deckbuilder/loadout.ts";
import { deckLabel, deckStatus, deckVerdict } from "../game/deckbuilder/workshop.ts";
import { t } from "../i18n/i18n.ts";
import type { CollectionResponse, DecksResponse } from "../net/api.ts";
import { deckPath, newDeckPath } from "../net/navigate.ts";
import { followInApp } from "../routes/nav.tsx";
import { collectionTestid as T, deckModes, type DeckMode } from "./collection.ts";

/** Each text is its own literal, so the translation guard sees it. */
function modeWords(mode: DeckMode): string {
  return mode === "bo1" ? t("Best of 1") : t("Conquest");
}

export default function DeckTiles({
  data,
  collection,
}: {
  data: DecksResponse;
  /** Null when the collection could not be read: ownership is then unknown. */
  collection: CollectionResponse | null;
}): ReactElement {
  const catalog = currentCatalog();
  const owned = collection === null ? null : collectionFrom(collection.entries);
  const { decks, trios, limits } = data;
  const atCap = decks.length >= limits.decks;
  return (
    <CardDefsProvider defs={catalog.cards}>
      <ul className="collection-deck-tiles" data-testid={T.decks}>
        {decks.map((deck) => {
          const verdict = deckVerdict(deck, catalog, owned, limits.nameLength);
          const status = deckStatus(verdict, owned !== null);
          const modes = deckModes(deck, decks, trios, catalog, owned, limits.nameLength);
          const href = deckPath(deck.id);
          return (
            <li key={deck.id}>
              <a
                className="collection-deck-tile"
                href={href}
                data-testid={T.deckTile(deck.id)}
                data-status={status.kind}
                data-modes={modes.join(" ")}
                onClick={followInApp(href)}
              >
                <PortraitArt portrait={portraitOrDefault(deck.portrait)} className="collection-deck-tile__portrait" />
                <span className="collection-deck-tile__name">{deckLabel(deck, limits.nameLength)}</span>
                <span className="collection-deck-tile__count">{`${String(deck.cards.length)}/${String(DECK_SIZE)}`}</span>
                <span className="ws-chip collection-deck-tile__status" data-status={status.kind}>
                  {status.label}
                </span>
                <span className="collection-deck-tile__modes">
                  {modes.length === 0 ? t("Not playable yet") : modes.map(modeWords).join(t(", "))}
                </span>
              </a>
            </li>
          );
        })}
        <li>
          {atCap ? (
            <button type="button" className="collection-deck-tile collection-deck-tile--new" data-testid={T.deckNew} disabled>
              <span className="collection-deck-tile__name">{t("New deck")}</span>
            </button>
          ) : (
            <a
              className="collection-deck-tile collection-deck-tile--new"
              href={newDeckPath()}
              data-testid={T.deckNew}
              onClick={followInApp(newDeckPath())}
            >
              <span className="collection-deck-tile__name">{t("New deck")}</span>
            </a>
          )}
        </li>
      </ul>
    </CardDefsProvider>
  );
}
