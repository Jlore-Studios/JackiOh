// R4582002 (issue #4582): the enemy's upgrades beside its hero in an Infinite Assault fight. One
// entry per upgrade the level holds (`AssaultLevel.upgrades`), on a button that opens the list of
// their names, stacks and effects. The view's modifiers carry no handicap or deck mark, so the
// upgrades are listed from the level the setup names, which the Assault states openly:
// presentation only (CLAUDE.md rule 7).
//
// It is a route's slot (`Game`'s `heroAside`), as `boardRail` is: it reads the level, never a game,
// so it can show nothing the view does not (CLAUDE.md rule 7).

import { useCallback, useEffect, useId, useRef, useState, type ReactElement } from "react";

import type { AssaultUpgrade, AssaultUpgradeKind } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { BackLayer } from "../../net/backLayers.ts";
import { assaultFightTestid } from "./testids.ts";
import "./fight.css";

/** R4582002: the enemy's upgrade by its kind, through `t()`; no amount is stated (CLAUDE.md rule 9). */
export function upgradeName(kind: AssaultUpgradeKind): string {
  switch (kind) {
    case "openingCards":
      return t("Expanded Context");
    case "deckSize":
      return t("Bigger Dataset");
    case "radiant":
      return t("Fine-Tuned");
    case "buffed":
      return t("Reinforced Weights");
    case "manaBonus":
      return t("Overclocked");
    case "manaCap":
      return t("Raised Ceiling");
    case "heroHealth":
      return t("Redundancy");
    case "heroArmor":
      return t("Hardened Shell");
    case "extraDraw":
      return t("Parallel Inference");
  }
}

/** R4582002: what the upgrade does, through `t()`; no amount is stated (CLAUDE.md rule 9). */
export function upgradeLine(kind: AssaultUpgradeKind): string {
  switch (kind) {
    case "openingCards":
      return t("More cards in its opening hand.");
    case "deckSize":
      return t("More cards in its deck.");
    case "radiant":
      return t("More of its cards start Radiant.");
    case "buffed":
      return t("More of its cards start with a Buff.");
    case "manaBonus":
      return t("More mana than its turn count gives.");
    case "manaCap":
      return t("A higher cap on its max mana.");
    case "heroHealth":
      return t("More health on its hero.");
    case "heroArmor":
      return t("Its hero starts with Armor.");
    case "extraDraw":
      return t("An extra draw each turn.");
  }
}

type UpgradeTrayProps = {
  /** The level's upgrades (`AssaultLevel.upgrades`), each with its stacks. */
  upgrades: readonly AssaultUpgrade[];
};

/** The enemy's tray; nothing when the level holds no upgrade. */
export function UpgradeTray({ upgrades }: UpgradeTrayProps): ReactElement | null {
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

  if (upgrades.length === 0) return null;

  const count = String(upgrades.length);
  const label = t("Boosts: {count}", { count });

  return (
    <div
      className="assault-boost-tray"
      data-testid={assaultFightTestid.upgrades}
      ref={root}
      onKeyDown={(event) => {
        if (event.key !== "Escape" || !open) return;
        event.stopPropagation();
        close();
      }}
    >
      <button
        type="button"
        className="assault-boost-tray__open button-secondary"
        data-testid={assaultFightTestid.upgradesOpen}
        ref={button}
        aria-expanded={open}
        aria-controls={listId}
        aria-label={label}
        title={label}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        {label}
      </button>
      {open ? (
        <ul className="assault-boost-tray__list" id={listId} data-testid={assaultFightTestid.upgradesList} aria-label={label}>
          {/* R2127000: Back closes the list, as Escape does. */}
          <BackLayer onClose={close} />
          {upgrades.map((upgrade, index) => (
            <li key={`${String(index)}-${upgrade.kind}`} className="assault-boost-tray__item" data-testid={assaultFightTestid.upgrade(upgrade.kind)}>
              <strong className="assault-boost-tray__name">{upgradeName(upgrade.kind)}</strong>
              <span className="assault-boost-tray__stacks">{t("×{count}", { count: String(upgrade.stacks) })}</span>
              <p className="assault-boost-tray__text">{upgradeLine(upgrade.kind)}</p>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
