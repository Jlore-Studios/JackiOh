// The dungeon run's shared faces and choices (DR14 part F, #2707): the catalog card faces the
// deck builder draws, the run's relic and blessing details, and the deck-index lists every tile
// screen renders its card actions from. Players read "relic"; the code keeps the run's word,
// artifact (WRITING.md's jargon). Presentation only (CLAUDE.md rule 7): nothing here answers a
// rule, and every word goes through `t()` at render, never at module scope.

import type { ReactElement, ReactNode } from "react";

import type { CardDef } from "@jackioh/shared";
import type { RunAction, RunCard, RunContent, RunCurseDef, RunPassiveDef, RunState } from "@jackioh/run";

import { CardFace, faceModel } from "../../cards/index.ts";
import { readerName } from "../../cards/chinese.ts";
import { currentCatalog } from "../../game/currentCatalog.ts";
import { t } from "../../i18n/i18n.ts";
import { TIER_LABEL } from "../fight/ArtifactTray.tsx";

/** What every screen below sends to the run: part D's `Dungeon.act` behind it. */
export type SendRunAction = (action: RunAction) => void;

/** The compiled-in catalog's definition of a card, or undefined for a card it lacks. */
export function catalogDef(id: string): CardDef | undefined {
  return currentCatalog().cards[id];
}

/**
 * A card's name as the reader reads it: the catalog's, else the run pool's, else its id, so a
 * card the catalog lacks names itself instead of crashing (DR1).
 */
export function cardName(content: RunContent, id: string): string {
  const def = catalogDef(id);
  if (def !== undefined) return readerName(def);
  return content.pool.find((card) => card.id === id)?.name ?? id;
}

/** A relic's or blessing's definition in the run's content, or undefined for an unknown id. */
function passiveDef(content: RunContent, id: string): RunPassiveDef | undefined {
  return content.passives.find((passive) => passive.id === id);
}

/** A relic's or blessing's name, translated, falling back to its id. */
export function passiveName(content: RunContent, id: string): string {
  const def = passiveDef(content, id);
  return def === undefined ? id : t(def.name);
}

function curseDef(content: RunContent, id: string): RunCurseDef | undefined {
  return content.curses.find((curse) => curse.id === id);
}

/** A curse's name, translated, falling back to its id: a record may name one the content dropped. */
export function curseName(content: RunContent, id: string): string {
  const def = curseDef(content, id);
  return def === undefined ? id : t(def.name);
}

/**
 * Whether the run may keep an opened pack empty: a relic held or a blessing taken whose effect
 * is a positive `skip` value (its Coins). Levels only raise the value, so any positive one opens
 * the choice.
 */
export function hasSkipPassive(run: RunState, content: RunContent): boolean {
  const held = [...run.artifacts.map((artifact) => artifact.id), ...run.blessings];
  return held.some((id) => {
    const effect = passiveDef(content, id)?.effect;
    return effect !== null && effect !== undefined && effect.type === "skip" && effect.coins > 0;
  });
}

type RunCardFaceProps = {
  /** The card's catalog id. */
  id: string;
  /** The run deck's Radiant mark, which the face is dealt on. */
  radiant: boolean;
  /** The run deck's Buff count, shown beside the face. */
  buffs?: number;
  content: RunContent;
  /** Carried by the wrapping button when `onSelect` is set, else the plain face's. */
  testid?: string;
  /** The button's accessible name; the face inside stays decoration, as in the deck builder. */
  label?: string;
  selected?: boolean;
  disabled?: boolean;
  /** Set, the face is drawn in a real button that calls it; absent, a plain face. */
  onSelect?: () => void;
};

/**
 * One card on the deck builder's full face (`faceModel` over the catalog, `CardFace`), in a real
 * button only when it does something. A missing catalog definition still draws (its pool name),
 * never crashes.
 */
export function RunCardFace({
  id,
  radiant,
  buffs = 0,
  content,
  testid,
  label,
  selected = false,
  disabled = false,
  onSelect,
}: RunCardFaceProps): ReactElement {
  const face = faceModel({ defId: id, def: catalogDef(id), radiant });
  const name = cardName(content, id);
  const inner = (
    <>
      <span className="dungeon-run-card__face" aria-hidden={onSelect === undefined ? undefined : true}>
        <CardFace face={face} layout="full" />
      </span>
      {buffs <= 0 ? null : (
        <span className="dungeon-run-card__buffs">{t("Buffs: {count}", { count: String(buffs) })}</span>
      )}
    </>
  );
  if (onSelect === undefined) {
    return (
      <span className="dungeon-run-card" data-testid={testid} data-card={id} title={name}>
        {inner}
      </span>
    );
  }
  return (
    <button
      type="button"
      className="dungeon-run-card"
      data-testid={testid}
      data-card={id}
      data-selected={selected ? "true" : undefined}
      aria-label={label ?? name}
      aria-pressed={selected}
      disabled={disabled}
      onClick={onSelect}
    >
      {inner}
    </button>
  );
}

type PassiveChoiceProps = {
  /** The relic's or blessing's id. */
  id: string;
  /** A relic held's level; null for a blessing or an offer. */
  level?: number | null;
  content: RunContent;
};

/**
 * A relic or blessing with its translated name, tier or god, text and level; an unknown id shows
 * under its id rather than hiding.
 */
export function PassiveChoice({ id, level = null, content }: PassiveChoiceProps): ReactElement {
  const def = passiveDef(content, id);
  if (def === undefined) {
    return <span className="dungeon-run-passive">{id}</span>;
  }
  const tier = def.tier === null ? null : TIER_LABEL[def.tier];
  return (
    <span className="dungeon-run-passive">
      <strong className="dungeon-run-passive__name">{t(def.name)}</strong>
      {tier === null || tier === undefined ? null : (
        <span className="dungeon-run-passive__meta">{t(tier)}</span>
      )}
      {def.god === null ? null : <span className="dungeon-run-passive__meta">{t(def.god)}</span>}
      {level === null ? null : (
        <span className="dungeon-run-passive__level">{t("Level {level}", { level: String(level) })}</span>
      )}
      <span className="dungeon-run-passive__text">{t(def.text)}</span>
    </span>
  );
}

/** A curse with its translated name, points and text; an unknown id shows under its id rather than hiding. */
export function CurseChoice({ id, content }: { id: string; content: RunContent }): ReactElement {
  const def = curseDef(content, id);
  if (def === undefined) {
    return <span className="dungeon-run-passive">{id}</span>;
  }
  return (
    <span className="dungeon-run-passive">
      <strong className="dungeon-run-passive__name">{t(def.name)}</strong>
      <span className="dungeon-run-passive__meta">{t("Points: {points}", { points: String(def.points) })}</span>
      <span className="dungeon-run-passive__text">{t(def.text)}</span>
    </span>
  );
}

type IndexedCardChoicesProps = {
  run: RunState;
  content: RunContent;
  /** The button's test id, by the card's place in the deck: the action's own `index`. */
  testid: (index: number) => string;
  /** The button's accessible name, by card and deck index. */
  label: (card: RunCard, index: number) => string;
  /** Sends the deck index on, never the card's id. */
  onPick: (index: number) => void;
  /** Which deck places are unavailable; every other card stays a live button. */
  disabledIndex?: (card: RunCard, index: number) => boolean;
  /** Which deck places read selected (a removal form's toggles). */
  selectedIndex?: (card: RunCard, index: number) => boolean;
};

/**
 * The run's deck in source order, each card a button sending its own deck index. Indices are the
 * run's: the list is never sorted or filtered, so a rendered place is the action's `index`.
 */
export function IndexedCardChoices({
  run,
  content,
  testid,
  label,
  onPick,
  disabledIndex,
  selectedIndex,
}: IndexedCardChoicesProps): ReactElement {
  return (
    <ul className="dungeon-run-cards">
      {run.deck.map((card, index) => (
        <li className="dungeon-run-cards__item" key={`${String(index)}-${card.id}`}>
          <RunCardFace
            id={card.id}
            radiant={card.radiant}
            buffs={card.buffs}
            content={content}
            testid={testid(index)}
            label={label(card, index)}
            selected={selectedIndex?.(card, index) ?? false}
            disabled={disabledIndex?.(card, index) ?? false}
            onSelect={() => {
              onPick(index);
            }}
          />
        </li>
      ))}
    </ul>
  );
}

/** A section of a tile screen: its heading and its choices. */
export function TileSection({ title, children }: { title: string; children: ReactNode }): ReactElement {
  return (
    <section className="dungeon-run-section">
      <h3 className="dungeon-run-section__title">{title}</h3>
      {children}
    </section>
  );
}
