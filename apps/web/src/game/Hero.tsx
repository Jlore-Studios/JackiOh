// A hero: health, armor and the Heroic Power (SPEC §3, BUILD M5-T1).
//
// The hero is a click target like any card — `hero-<side>` — so an attack can land on it, by a
// click or by a pointer drag the DragLayer resolves through the same testid. Whether it may be
// attacked is `props.highlight.legal`, and whether it glows green is `props.highlight.glow`;
// Taunt lives in the engine.
//
// The modifier badges (BUILD M5-T4 `modifierChanged`: "badge list equals the view's modifiers")
// come from `SideView.modifiers`, which R169 put in the view — `{ id, label }` per §10.1 modifier,
// on both seats. The list is rendered here and nowhere else, so `modifiers-<side>` (the testid the
// M5-T4 animation table resolves `modifierChanged` to) has exactly one element per seat. The
// container is rendered even when the list is empty, because the fade the table plays on it is
// the animation for the modifier that has just *left*.
//
// Nothing is derived here: the label is the view's, the order is the view's, and a modifier the
// view does not carry is not drawn. Reconstructing badges from the event window would be a guess —
// it is the last N events (§10.8), so a badge could appear and never leave.
//
// The Heroic Power (R43, R384, R510, patch v0.2.1). The view carries the selected power and may
// retain its other powers for rules and history, but the game presents only the selected one. It is
// drawn as a Hearthstone hero power is, on the hero: a round crest with the power's own art
// (cards/art/powerArt.ts, keyed by the stored name the view gives it, R103), its X in a mana gem on
// the crest's rim, a gold ring when it runs its Radiant face, and its printed title beside it
// (Armor Up's Radiant face is Tank Up). The tooltip and the accessible name are the power as the
// catalog prints it, "Activate: Spend (X): <Title>: <clause>.", its `{shot}` filled with the number
// the view gives it (cards/inPlay.ts). A spent power is drawn greyed, as Hearthstone turns its over.
//
// The viewer's selected power is a button that reports `{ on: "activate", instanceId }`, the click every
// Activate control reports (ActivateControl.tsx), so `actions.ts` builds a power exactly as it builds
// any activation: the one `activate` `legalActions` lists for it is sent at once, and a power with a
// target to declare (Ping, R606) waits for it on the board, clicked or dragged to (game/drag). The
// control keeps the `power` testid the e2e specs press. The opponent's selected power is a tag with
// the same crest, which nothing presses. Whether a button is live is
// `props.highlight.legal`; `usedThisTurn` is drawn, never obeyed. A power flashes on the `activated`
// row, which plays on its card (`card-<instanceId>`).

import type { CSSProperties, ReactElement } from "react";

import { fillPowerParams, POWER_ART_BOX, powerArtOf, powerLine, powerTitle } from "../cards/index.ts";
import { ACTIVATED_EVENT } from "./ActivateControl.tsx";
import { animTestid } from "./animations.ts";
import { cx, isLegal, isSelected, legalAttr, PopLayer, type Pops } from "./Card.tsx";
import { useCardInfo } from "./catalog.ts";
import {
  sideView,
  testid,
  type AnimatingMap,
  type BoardControl,
  type ClickTarget,
  type Highlight,
  type Side,
} from "./contract.ts";
import { glowAttr } from "./glow.ts";
import type { HeroPowerView, PlayerView } from "@jackioh/shared";

export type HeroProps = {
  view: PlayerView;
  side: Side;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  onControl?: (control: BoardControl) => void;
  pops?: Pops;
};

/** What a spent power's tooltip and accessible name add (`usedThisTurn`, drawn and never obeyed). */
export const POWER_USED_NOTE = "Used this turn.";

/**
 * The power as the catalog prints it, "Activate: Spend (X): <Title>: <clause>.", on the face it runs,
 * its declared numbers filled with the ones the view gives it (Steady Shot's `{shot}`, R608) and else
 * the card's printed ones. A stored name the client's table does not know reads as itself.
 */
export function usePowerWords(power: HeroPowerView): string {
  const info = useCardInfo(power.defId, power.radiant);
  const line = powerLine({ name: power.name, x: power.x }, power.radiant);
  if (line === null) return `${power.name} (${String(power.x)})`;
  return fillPowerParams(line, power.radiant, info.def?.params, power.params);
}

/**
 * A power's face on the hero: the crest with its own art (keyed by the stored name), the X in a mana
 * gem on its rim, and the printed title (the Radiant face's on a Radiant power). Decoration only:
 * its button or tag carries the words.
 */
function PowerFace({ power }: { power: HeroPowerView }): ReactElement {
  const art = powerArtOf(power.name);
  const style = { "--power-light": art.light, "--power-dark": art.dark } as CSSProperties;
  return (
    <>
      <span className="power-crest" data-power-art={power.name} data-glyph={art.glyph} style={style} aria-hidden="true">
        <svg className="power-glyph" viewBox={`0 0 ${String(POWER_ART_BOX)} ${String(POWER_ART_BOX)}`} focusable="false">
          <path d={art.path.d} fillRule={art.path.rule} />
        </svg>
        <span className="power-x" data-x={power.x}>
          {power.x}
        </span>
      </span>
      <span className="power-title">{powerTitle(power.name, power.radiant) ?? power.name}</span>
    </>
  );
}

/** A power's tooltip: its words, and that it is spent when it is. */
function powerTooltip(words: string, power: HeroPowerView): string {
  return power.usedThisTurn ? `${words} ${POWER_USED_NOTE}` : words;
}

export default function Hero(props: HeroProps): ReactElement {
  const { view, side } = props;
  const seat = sideView(view, side);
  const hero = seat.hero;

  const testId = testid.hero(side);
  const legal = isLegal(props.highlight, testId);
  const selected = isSelected(props.highlight, testId);
  const target: ClickTarget = { on: "hero", side };

  const modifiersId = animTestid.modifiers(side);
  const modifiers = seat.modifiers ?? [];

  return (
    <div
      className={cx("hero", `hero-${side}`)}
      data-testid={testId}
      data-side={side}
      data-legal={legalAttr(legal)}
      data-selected={selected ? "true" : undefined}
      data-animating={props.animating?.get(testId)}
      data-glow={glowAttr(props.highlight, testId)}
      aria-disabled={legal ? undefined : "true"}
      aria-label={side === "you" ? "Your hero" : "Opponent hero"}
      tabIndex={legal ? 0 : undefined}
      onClick={() => {
        if (!legal) return;
        props.onClick?.(target);
      }}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        if (!legal) return;
        props.onClick?.(target);
      }}
    >
      <span className="hero-seat">{side === "you" ? "You" : "Opponent"}</span>
      {/* A hero past lethal reads 0, as Hearthstone draws it: "-6" is overkill, not health. The
          true number stays in `data-health`. */}
      <span className="hero-health" data-health={hero.health} title="Health">
        {Math.max(0, hero.health)}
      </span>
      {hero.armor > 0 && (
        <span className="hero-armor" data-armor={hero.armor} title="Hero armor">
          {hero.armor}
        </span>
      )}

      {hero.power !== null &&
        (side === "you" ? (
          // The one `power` testid in the DOM.
          <PowerButton power={hero.power} testId={testid.power} props={props} />
        ) : (
          <PowerTag power={hero.power} />
        ))}

      {/* R169: one badge per `SideView.modifiers` entry, in the view's order. Always present, so
          `modifierChanged` has an element to fade even when the badge that changed is the one that
          has just gone. */}
      <span
        className="modifiers"
        data-testid={modifiersId}
        data-count={modifiers.length}
        data-animating={props.animating?.get(modifiersId)}
        aria-label={side === "you" ? "Your modifiers" : "Opponent modifiers"}
      >
        {modifiers.map((modifier) => (
          <span
            key={modifier.id}
            className="modifier-badge"
            data-modifier-id={modifier.id}
            title={modifier.label}
          >
            {modifier.label}
          </span>
        ))}
      </span>

      <PopLayer pops={props.pops} />
    </div>
  );
}

/**
 * The viewer's selected Heroic Power. A press reports the power's instance as an activation
 * (`{ on: "activate" }`), never the `power` board control, so a power with targets is built like any
 * activation; `data-instance-id` is also what a drag from it reads (game/drag/targets.ts).
 */
function PowerButton({
  power,
  testId,
  props,
}: {
  power: HeroPowerView;
  testId: string;
  props: HeroProps;
}): ReactElement {
  const words = usePowerWords(power);
  const live = isLegal(props.highlight, testId);
  const flashing = props.animating?.get(testid.card(power.instanceId)) === ACTIVATED_EVENT;
  const tooltip = powerTooltip(words, power);
  return (
    <button
      type="button"
      className="power-button"
      data-testid={testId}
      data-instance-id={power.instanceId}
      data-power={power.name}
      data-radiant={power.radiant ? "true" : undefined}
      data-legal={legalAttr(live)}
      data-selected={isSelected(props.highlight, testId) ? "true" : undefined}
      data-animating={props.animating?.get(testId)}
      data-glow={glowAttr(props.highlight, testId)}
      data-used={power.usedThisTurn ? "true" : "false"}
      data-x={power.x}
      data-flash={flashing ? ACTIVATED_EVENT : undefined}
      aria-disabled={live ? undefined : "true"}
      aria-label={`Heroic Power: ${tooltip}`}
      disabled={!live}
      title={tooltip}
      onClick={(event) => {
        event.stopPropagation();
        if (!live) return;
        props.onClick?.({ on: "activate", instanceId: power.instanceId });
      }}
      onKeyDown={(event) => {
        // Enter and Space press the button natively; the hero it sits on must not take them too
        // (its own handler would cancel the press), as an Activate control's card does not.
        if (event.key === "Enter" || event.key === " ") event.stopPropagation();
      }}
    >
      <PowerFace power={power} />
    </button>
  );
}

/** The opponent's selected Heroic Power: the same crest, which nothing presses. */
function PowerTag({ power }: { power: HeroPowerView }): ReactElement {
  const tooltip = powerTooltip(usePowerWords(power), power);
  return (
    <span
      className="power-tag"
      data-instance-id={power.instanceId}
      data-power={power.name}
      data-radiant={power.radiant ? "true" : undefined}
      data-used={power.usedThisTurn ? "true" : "false"}
      data-x={power.x}
      role="img"
      aria-label={`Opponent's Heroic Power: ${tooltip}`}
      title={tooltip}
    >
      <PowerFace power={power} />
    </span>
  );
}
