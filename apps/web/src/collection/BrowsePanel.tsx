// Issue #3734 (R3734000): where the Collection's filters sit. Over `tablet` (1025 px and wider) they are a sidebar to
// the left of the grid; at 1024 px and under a "Filters" button opens them in a sheet (the Dialog's), so a phone and
// an upright iPad keep the room for the grid. Exactly one of the two is drawn, so the controls are mounted once and
// each test id is found once. Nothing here decides what a filter means (`game/deckbuilder/filters.ts`).

import { useState, useSyncExternalStore, type ReactElement, type ReactNode } from "react";

import { upTo } from "../breakpoints.ts";
import { Dialog } from "../game/Dialog.tsx";
import { queryList, subscribeTo } from "../game/mediaQuery.ts";
import { t } from "../i18n/i18n.ts";
import { collectionTestid } from "./collection.ts";

/** 1024 px and under: the sheet's widths. Asked as its own query, so a host that answers none (a test's jsdom) is wide. */
const NARROW = upTo("tablet");

/** Whether the window is wide enough for a sidebar; a host with no `matchMedia` is wide. */
function isWide(): boolean {
  return queryList(NARROW)?.matches !== true;
}

function subscribeToWidth(onChange: () => void): () => void {
  return subscribeTo(NARROW, onChange);
}

export default function BrowsePanel({
  controls,
  active,
}: {
  /** The filters themselves, drawn once in whichever place is shown. */
  controls: ReactNode;
  /** How many chips and boxes are on, for the button's count. */
  active: number;
}): ReactElement {
  const wide = useSyncExternalStore(subscribeToWidth, isWide, () => true);
  const [open, setOpen] = useState(false);

  if (wide) {
    return (
      <aside className="collection-filters" data-testid={collectionTestid.filters} aria-label={t("Filters")}>
        {controls}
      </aside>
    );
  }
  return (
    <>
      <button
        type="button"
        className="collection-filters-open button-secondary"
        data-testid={collectionTestid.filtersOpen}
        data-active={active > 0 ? "true" : "false"}
        aria-haspopup="dialog"
        onClick={() => {
          setOpen(true);
        }}
      >
        {active > 0 ? t("Filters · {count}", { count: active }) : t("Filters")}
      </button>
      {open ? (
        <Dialog
          role="dialog"
          sheet
          title={t("Filters")}
          onClose={() => {
            setOpen(false);
          }}
          actions={[]}
          testid={collectionTestid.filtersSheet}
          className="collection-filters-sheet"
        >
          <div className="collection-filters" data-testid={collectionTestid.filters}>
            {controls}
          </div>
        </Dialog>
      ) : null}
    </>
  );
}
