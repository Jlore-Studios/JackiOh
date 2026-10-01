// The Activate control (docs/classic-sets.md B3.2, SPEC §6.2, R384; its presentation R510).
//
// A card the viewer controls whose view lists `activations` (the engine puts them on its
// controller's own view of a card acting on the field: the top of a pile, or a face-up backrow
// card) wears one control per ability on the card itself: a lightning glyph and a badge counting
// the uses left this turn ("2"; "∞" for Activate ♾️, whose `usesLeft` is null). Its label, the
// ability's words, is the control's tooltip and accessible name, beside the card's full text in the
// hover preview and the inspect sheet.
//
// No rule lives here (CLAUDE.md rule 7). Whether the control fires is `highlight.legal`, which
// `actions.ts` derived from the `activate` actions `legalActions` listed; the view's `usable` and
// `reason` are drawn, never obeyed: a control that is not legal is greyed, says why in its tooltip
// (the engine's own words, `ActivationView.reason`) and sends nothing. A press reports
// `{ on: "activate", instanceId, ability? }`, which `actions.ts` builds exactly as a play's choices
// are built: one listed activation is sent at once, several wait for a target clicked (or dragged
// to, game/drag) on the board, a Tribute, or a mode in the inline picker.
//
// `ability` is named only when the card lists several abilities (`namedAbility`), so a card with
// one is `activate-<instanceId>` and its badge `activate-uses-<instanceId>`.
//
// The `activated` event's animation row plays on the card (`card-<instanceId>`, animations.ts); the
// control reads that from the board's animating map and flashes (`data-flash="activated"`). Under
// reduced motion the flash does not move: a static ring marks the control for the row's duration.

import type { KeyboardEvent, MouseEvent, ReactElement } from "react";

import { fillParams, type ActivationView, type CardView } from "@jackioh/shared";

import { useCardInfo, type CardInfo } from "./catalog.ts";
import { NO_HIGHLIGHT, namedAbility, testid, type AnimatingMap, type ClickTarget, type Highlight } from "./contract.ts";
import { glowAttr } from "./glow.ts";

import "./activate.css";

/** The event whose animation row the control flashes on (animations.ts `activated`). */
export const ACTIVATED_EVENT = "activated";

/** What the uses badge says for Activate ♾️ (`usesLeft: null`). */
export const UNLIMITED_USES = "∞";

/** The tooltip's reason when the view gives none and the control is not live (the board catching up, say). */
export const NOT_NOW = "can't be activated right now";

/** The attribute a drag reads a control's card off (game/drag/targets.ts). */
export const ACTIVATE_FOR_ATTRIBUTE = "data-activate-for";
/** The attribute a drag reads the ability a control names off, when it names one. */
export const ACTIVATE_ABILITY_ATTRIBUTE = "data-activate-ability";

/** "2", or "∞" for Activate ♾️. */
export function usesText(usesLeft: number | null): string {
  return usesLeft === null ? UNLIMITED_USES : String(usesLeft);
}

function usesWords(usesLeft: number | null): string {
  if (usesLeft === null) return "unlimited uses this turn";
  return usesLeft === 1 ? "1 use left this turn" : `${String(usesLeft)} uses left this turn`;
}

function capitalised(text: string): string {
  return text.length === 0 ? text : `${text.charAt(0).toUpperCase()}${text.slice(1)}`;
}

/**
 * The ability's words with the card's declared numbers filled in (B3.4: a Degrade or Upgrade moves
 * them, and the view carries them as `params`), so no raw `{key}` reaches the screen.
 */
export function abilityLabel(info: CardInfo, card: CardView, label: string): string {
  const def = info.def;
  if (def === undefined || !label.includes("{")) return label;
  return fillParams(
    { params: def.params, base: { ...def.base, text: label }, radiant: { ...def.radiant, text: label } },
    card.radiant ? "radiant" : "base",
    card.params,
  );
}

export type ActivateControlsProps = {
  /** The card as the view lists it: a `UnitView` on top of its pile, or a face-up `BackrowView`. */
  card: CardView;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
};

/** Every Activate ability the view lists on `card`, one control each; nothing when it lists none. */
export default function ActivateControls(props: ActivateControlsProps): ReactElement | null {
  const { card } = props;
  const info = useCardInfo(card.defId, card.radiant);
  const activations = card.activations ?? [];
  if (activations.length === 0) return null;
  const flashing = props.animating?.get(testid.card(card.instanceId)) === ACTIVATED_EVENT;
  return (
    <span className="activate-controls" data-count={activations.length}>
      {activations.map((activation) => (
        <ActivateButton
          key={activation.ability}
          card={card}
          name={info.name}
          label={abilityLabel(info, card, activation.label)}
          activation={activation}
          ability={namedAbility(activations.length, activation.ability)}
          highlight={props.highlight ?? NO_HIGHLIGHT}
          flashing={flashing}
          onClick={props.onClick}
        />
      ))}
    </span>
  );
}

function ActivateButton(props: {
  card: CardView;
  name: string;
  label: string;
  activation: ActivationView;
  /** The ability the control names: undefined when the card lists only one. */
  ability: string | undefined;
  highlight: Highlight;
  flashing: boolean;
  onClick?: (target: ClickTarget) => void;
}): ReactElement {
  const { card, activation, ability } = props;
  const testId = testid.activate(card.instanceId, ability);
  const legal = props.highlight.legal.has(testId);
  const selected = props.highlight.selected.has(testId);
  const why = legal ? null : capitalised(activation.reason ?? NOT_NOW);
  const target: ClickTarget =
    ability === undefined ? { on: "activate", instanceId: card.instanceId } : { on: "activate", instanceId: card.instanceId, ability };

  function press(event: MouseEvent<HTMLButtonElement>): void {
    // The card under the control has its own click (an attack, a target): this one is the control's.
    event.stopPropagation();
    if (!legal) return;
    props.onClick?.(target);
  }

  function key(event: KeyboardEvent<HTMLButtonElement>): void {
    // Enter and Space press the button natively; the card's own key handler must not take them too.
    if (event.key === "Enter" || event.key === " ") event.stopPropagation();
  }

  const uses = usesWords(activation.usesLeft);
  return (
    <button
      type="button"
      className="activate-control"
      data-testid={testId}
      // ACTIVATE_FOR_ATTRIBUTE and ACTIVATE_ABILITY_ATTRIBUTE: what a drag reads the control's build off.
      data-activate-for={card.instanceId}
      data-activate-ability={ability}
      data-legal={legal ? "true" : "false"}
      data-glow={glowAttr(props.highlight, testId)}
      data-selected={selected ? "true" : undefined}
      data-usable={activation.usable ? "true" : "false"}
      data-uses={activation.usesLeft ?? "unlimited"}
      data-flash={props.flashing ? ACTIVATED_EVENT : undefined}
      aria-disabled={legal ? undefined : "true"}
      aria-label={`Activate ${props.name}: ${props.label}. ${capitalised(uses)}.${why === null ? "" : ` ${why}.`}`}
      title={why === null ? `Activate: ${props.label} (${uses})` : `Activate: ${props.label} — ${why}`}
      onClick={press}
      onKeyDown={key}
    >
      <svg className="activate-glyph" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
        <path d="M9.5 1 3 9.2h4.3L6.2 15 13 6.6H8.6z" />
      </svg>
      <span
        className="activate-uses"
        data-testid={testid.activateUses(card.instanceId, ability)}
        data-uses={activation.usesLeft ?? "unlimited"}
        aria-hidden="true"
      >
        {usesText(activation.usesLeft)}
      </span>
    </button>
  );
}
