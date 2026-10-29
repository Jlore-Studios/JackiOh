// R2097000: a deck holding a card the newest patch changed says so, until it is saved under it.
// The badge sits on the deck's row and in its open editor, and opens exactly the deck's changed
// cards, each with its change. Presentation only (CLAUDE.md rule 7).

import { useId, useState, type ReactElement } from "react";

import { readerName } from "../../cards/chinese.ts";
import { t } from "../../i18n/i18n.ts";
import { ChangeList } from "../../patches/ChangeList.tsx";
import { useLoaded, usePatchSource } from "../../patches/context.tsx";
import { loadNewestChanges, type ChangedCard, type NewestChanges } from "../../patches/deckChanges.ts";
import { DECK_PATCH_BADGE, DECK_PATCH_CARDS, deckPatchCardId } from "./testids.ts";

import "../../patches/patches.css";

/** The newest patch's changes while it is the catalog the builder shows; null while loading or failed. */
export function useNewestChanges(catalogVersion: string): NewestChanges | null {
  const source = usePatchSource();
  const read = useLoaded(`newest-changes:${catalogVersion}`, () => loadNewestChanges(source, catalogVersion));
  return read.status === "ready" ? read.value : null;
}

export default function DeckPatchBadge({
  deckId,
  cards,
  version,
}: {
  deckId: string;
  cards: readonly ChangedCard[];
  version: string;
}): ReactElement | null {
  const [open, setOpen] = useState(false);
  const listId = useId();
  if (cards.length === 0) return null;
  const count = cards.length;
  return (
    <div className="ws-patch">
      <button
        type="button"
        className="ws-patch-badge"
        data-testid={DECK_PATCH_BADGE}
        data-deck={deckId}
        data-count={String(count)}
        data-version={version}
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        {count === 1 ? t("{count} card changed in {version}", { count, version }) : t("{count} cards changed in {version}", { count, version })}
      </button>
      <div id={listId} className="ws-patch-panel" hidden={!open}>
        {open ? (
          <ul className="ws-patch-cards" data-testid={DECK_PATCH_CARDS} aria-label={t("Cards {version} changed in this deck", { version })}>
            {cards.map((delta) => (
              <li key={delta.id} className="ws-patch-card" data-testid={deckPatchCardId(delta.id)} data-card={delta.id}>
                <span className="ws-patch-name">{readerName(delta.def)}</span>
                <ChangeList delta={delta} />
              </li>
            ))}
          </ul>
        ) : null}
      </div>
    </div>
  );
}

