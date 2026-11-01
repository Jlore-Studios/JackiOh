// Every live modifier (R169), in full, one tap from the table's controls.
//
// The board draws R169's badges beside each hero, but on a phone the rail beside the portrait
// holds a count and a few shortened labels at most, and a badge's full text is only its `title`,
// which a touch screen never shows. This is where the rest can be read: a chip with the count while
// any modifier is live on either side, which opens the whole list. Practice and the lessons mount it
// in their HUD; the online match, the Duos board, the watch screen and the replay player in the
// board's rail. It reads the same `SideView.modifiers` the board does, so it can show nothing the
// view does not (§10.8, CLAUDE.md rule 7).

import { useEffect, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { PlayerView } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import { BackLayer } from "../net/backLayers.ts";
import "./modifierList.css";

/** The chip (`data-count`: the live modifiers on both sides) and the list it opens. `e2e/support/testids.ts` carries the same strings. */
export const modifierListTestid = {
  chip: "modifier-list",
  panel: "modifier-list-panel",
} as const;

type ModifierListProps = {
  view: PlayerView;
  /** The heading over the opponent's list, already translated: "AI" in practice and lessons, "Opponent" against a person. */
  opponentLabel: string;
};

export function ModifierList({ view, opponentLabel }: ModifierListProps): ReactElement | null {
  const [open, setOpen] = useState(false);
  const panelId = useId();
  const root = useRef<HTMLSpanElement>(null);
  const panel = useRef<HTMLSpanElement>(null);

  const yours = view.you.modifiers ?? [];
  const theirs = view.opponent.modifiers ?? [];
  const count = yours.length + theirs.length;

  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent): void {
      if (event.key === "Escape") setOpen(false);
    }
    function onPointer(event: PointerEvent): void {
      if (!(event.target instanceof Node)) return;
      if (root.current?.contains(event.target) === true || panel.current?.contains(event.target) === true) return;
      setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onPointer);
    };
  }, [open]);

  if (count === 0) return null;

  return (
    <span className="modifier-list" ref={root}>
      <button
        type="button"
        className="modifier-list__chip"
        data-testid={modifierListTestid.chip}
        data-count={count}
        aria-expanded={open}
        aria-controls={panelId}
        aria-label={
          count === 1
            ? t("{count} active modifier", { count: String(count) })
            : t("{count} active modifiers", { count: String(count) })
        }
        title={t("Active modifiers")}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        <span aria-hidden="true">✦</span> {count}
      </button>
      {/* The HUD clips its contents (a blurred, rounded bar), so the list opens on the page. */}
      {open
        ? createPortal(
        <span
          ref={panel}
          className="modifier-list__panel"
          id={panelId}
          data-testid={modifierListTestid.panel}
          role="group"
          aria-label={t("Active modifiers")}
        >
          {/* R2127000: Back closes the list, as Escape does. */}
          <BackLayer onClose={() => setOpen(false)} />
          {[
            { key: "you", title: t("You"), list: yours },
            { key: "opponent", title: opponentLabel, list: theirs },
          ].map((side) =>
            side.list.length === 0 ? null : (
              <span key={side.key} className="modifier-list__side" data-side={side.key}>
                <span className="modifier-list__who">{side.title}</span>
                {side.list.map((modifier) => (
                  <span key={modifier.id} className="modifier-list__item" data-modifier-id={modifier.id}>
                    {t(modifier.label)}
                  </span>
                ))}
              </span>
            ),
          )}
        </span>,
            document.body,
          )
        : null}
    </span>
  );
}
