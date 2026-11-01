// B23 (DR14 part E, #2706): a side's relics beside its hero in a dungeon run's fight. The player's
// tray holds the run's relics and blessings, the enemy's the relics it carries (DR5). One icon per
// relic, colored by its tier, on a button that opens the list of their names, tiers, levels and texts
// from the run's content (`RunContent.passives`). Players read "relics"; the code keeps the run's
// word, artifacts (WRITING.md's jargon).
//
// It is a route's slot (`Game`'s `heroAside`), as `boardRail` is: it reads the run, never a game, so
// it can show nothing the view does not (CLAUDE.md rule 7).

import { useCallback, useEffect, useId, useRef, useState, type ReactElement } from "react";

import type { RunPassiveDef, RunTier } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { BackLayer } from "../../net/backLayers.ts";
import { fightTestid, type TraySide } from "./testids.ts";
import "./fight.css";

/** One relic or blessing in a tray: its passive's id and, for a run's relic, its level. */
export type TrayItem = { id: string; level: number | null };

/** Each tier as the list names it, through `t()`. */
const TIER_LABEL: Readonly<Record<RunTier, string>> = {
  bronze: "Bronze",
  silver: "Silver",
  gold: "Gold",
  platinum: "Platinum",
  diamond: "Diamond",
};

type ArtifactTrayProps = {
  side: TraySide;
  items: readonly TrayItem[];
  /** The run's passives (`RunContent.passives`), where each item's name, text and tier are read. */
  passives: readonly RunPassiveDef[];
};

/** A side's tray; nothing when the side holds no relic or blessing. */
export function ArtifactTray({ side, items, passives }: ArtifactTrayProps): ReactElement | null {
  const [open, setOpen] = useState(false);
  const listId = useId();
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);

  /** Escape and Back close the list and put the focus back on the button that opened it. */
  const close = useCallback(() => {
    setOpen(false);
    button.current?.focus();
  }, []);

  // A press anywhere else closes the list, as the board's other menus do (ModifierList.tsx).
  useEffect(() => {
    if (!open) return;
    function onPointer(event: PointerEvent): void {
      if (event.target instanceof Node && root.current?.contains(event.target) === true) return;
      setOpen(false);
    }
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("pointerdown", onPointer);
    };
  }, [open]);

  if (items.length === 0) return null;

  const entries = items.map((item, index) => {
    const def = passives.find((passive) => passive.id === item.id);
    return { key: `${String(index)}-${item.id}`, item, def, tier: def?.tier ?? null };
  });
  const count = String(items.length);
  const label =
    side === "own" ? t("Your relics and blessings: {count}", { count }) : t("The enemy's relics: {count}", { count });

  return (
    <div
      className="artifact-tray"
      data-side={side}
      data-testid={fightTestid.tray(side)}
      ref={root}
      onKeyDown={(event) => {
        if (event.key !== "Escape" || !open) return;
        event.stopPropagation();
        close();
      }}
    >
      <button
        type="button"
        className="artifact-tray__open"
        data-testid={fightTestid.trayOpen(side)}
        ref={button}
        aria-expanded={open}
        aria-controls={listId}
        aria-label={label}
        title={label}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        {entries.map(({ key, item, tier }) => (
          <span
            key={key}
            className="artifact-icon"
            data-testid={fightTestid.trayIcon(side, item.id)}
            data-tier={tier ?? "none"}
            aria-hidden="true"
          />
        ))}
      </button>
      {open ? (
        <ul className="artifact-tray__list" id={listId} data-testid={fightTestid.trayList(side)} aria-label={label}>
          {/* R2127000: Back closes the list, as Escape does. */}
          <BackLayer onClose={close} />
          {entries.map(({ key, item, def, tier }) => (
            <li key={key} className="artifact-tray__item" data-testid={fightTestid.trayItem(side, item.id)} data-tier={tier ?? "none"}>
              <strong className="artifact-tray__name">{def === undefined ? item.id : t(def.name)}</strong>
              {tier === null && item.level === null ? null : (
                <span className="artifact-tray__meta">
                  {tier === null ? null : <span className="artifact-tray__tier">{t(TIER_LABEL[tier])}</span>}
                  {item.level === null ? null : (
                    <span className="artifact-tray__level">{t("Level {level}", { level: String(item.level) })}</span>
                  )}
                </span>
              )}
              {def === undefined ? null : <p className="artifact-tray__text">{t(def.text)}</p>}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
