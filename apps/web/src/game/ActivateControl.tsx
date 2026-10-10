// Activate control (B3.2, SPEC §6.2, R384; presentation R510).
// It renders view data only (CLAUDE.md rule 7): `highlight.legal` decides a press, while `usable`
// and `reason` are displayed only. A press reports `{ on: "activate", instanceId, ability? }`;
// `ability` is named only for multiple entries (`namedAbility`).
// It flashes for its `activated` animation row; reduced motion uses a static ring.

import type { KeyboardEvent, MouseEvent, ReactElement } from "react";

import { fillParams, type ActivationView, type CardView } from "@jackioh/shared";

import { useCardInfo, type CardInfo } from "./catalog.ts";
import { NO_HIGHLIGHT, namedAbility, testid, type AnimatingMap, type ClickTarget, type Highlight } from "./contract.ts";
import { glowAttr } from "./glow.ts";

import "./activate.css";

export const ACTIVATED_EVENT = "activated";

export const UNLIMITED_USES = "∞";

export const NOT_NOW = "can't be activated right now";

/** The attribute a drag reads a control's card off (game/drag/targets.ts). */
export const ACTIVATE_FOR_ATTRIBUTE = "data-activate-for";
/** The attribute a drag reads the ability a control names off, when it names one. */
export const ACTIVATE_ABILITY_ATTRIBUTE = "data-activate-ability";

export function usesText(usesLeft: number | null): string {
  return usesLeft === null ? UNLIMITED_USES : String(usesLeft);
}

function usesWords(usesLeft: number | null): string {
  if (usesLeft === null) return "unlimited uses this turn";
  return usesLeft === 1 ? "1 use left this turn" : `${String(usesLeft)} uses left this turn`;
}

/** The words as one sentence, ending in a single full stop however the label ended. */
function sentence(text: string): string {
  return `${text.replace(/[\s.]+$/, "")}.`;
}

function capitalised(text: string): string {
  return text.length === 0 ? text : `${text.charAt(0).toUpperCase()}${text.slice(1)}`;
}

/** The ability's words with the card's declared numbers (the view's `params`) filled in, B3.4, so no raw `{key}` reaches the screen. */
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
      aria-label={`Activate ${props.name}: ${sentence(props.label)} ${sentence(capitalised(uses))}${why === null ? "" : ` ${sentence(why)}`}`}
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
