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
// The selected Heroic Power (R43, R384, R510). The view can retain every power a player controls,
// but the hero surface deliberately shows only `hero.power`: the selected one. Its button reports
// `{ on: "activate", instanceId }`, the click every Activate control reports (ActivateControl.tsx),
// so `actions.ts` builds it once or waits for its target (and game/drag can aim it) exactly as for a
// card activation. The power keeps the `power` testid the e2e specs press. Whether a button is
// live is `props.highlight.legal`; `usedThisTurn` is drawn, never obeyed. A power flashes on the
// `activated` row, which plays on its card (`card-<instanceId>`).

import type { ReactElement } from "react";

import { ACTIVATED_EVENT } from "./ActivateControl.tsx";
import { animTestid } from "./animations.ts";
import { cx, isLegal, isSelected, legalAttr, PopLayer, type Pops } from "./Card.tsx";
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

type PowerArt = { key: string; mark: string };

/** Public names select visual motifs only; the power's effect and legality still come solely from the view. */
const POWER_ART: Readonly<Record<string, PowerArt>> = {
  "Expedition Map": { key: "map", mark: "⌖" },
  "Life Tap": { key: "tap", mark: "✦" },
  "Steady Shot": { key: "shot", mark: "➶" },
  Ranching: { key: "ranch", mark: "♞" },
  "Cat Cafe": { key: "cat", mark: "♟" },
  Ping: { key: "ping", mark: "✧" },
  "Witness Value": { key: "witness", mark: "◈" },
  Stitching: { key: "stitch", mark: "✚" },
  "Armor Up": { key: "armor", mark: "⬟" },
  "Tank Up": { key: "tank", mark: "⬢" },
  "Die Insect": { key: "insect", mark: "✹" },
  "KY Brainstorm": { key: "brain", mark: "☄" },
  Pluck: { key: "pluck", mark: "❋" },
  "Terminus Tricks": { key: "tricks", mark: "◆" },
};

const DEFAULT_POWER_ART: PowerArt = { key: "default", mark: "✦" };

function PowerEmblem({ power }: { power: HeroPowerView }): ReactElement {
  const art = POWER_ART[power.name] ?? DEFAULT_POWER_ART;
  return (
    <span className="power-emblem" data-power-art={art.key} aria-hidden="true">
      {art.mark}
    </span>
  );
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
          <span className="power-tag" data-used={hero.power.usedThisTurn ? "true" : "false"} data-x={hero.power.x}>
            <PowerEmblem power={hero.power} />
            {hero.power.name}
            <span className="power-x">{hero.power.x}</span>
          </span>
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
 * One of the viewer's Heroic Powers. A press reports the power's instance as an activation
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
  const live = isLegal(props.highlight, testId);
  const flashing = props.animating?.get(testid.card(power.instanceId)) === ACTIVATED_EVENT;
  return (
    <button
      type="button"
      className="power-button"
      data-testid={testId}
      data-instance-id={power.instanceId}
      data-legal={legalAttr(live)}
      data-selected={isSelected(props.highlight, testId) ? "true" : undefined}
      data-animating={props.animating?.get(testId)}
      data-glow={glowAttr(props.highlight, testId)}
      data-used={power.usedThisTurn ? "true" : "false"}
      data-x={power.x}
      data-flash={flashing ? ACTIVATED_EVENT : undefined}
      aria-disabled={live ? undefined : "true"}
      disabled={!live}
      title={`${power.name} (X ${power.x})`}
      onClick={(event) => {
        event.stopPropagation();
        if (!live) return;
        props.onClick?.({ on: "activate", instanceId: power.instanceId });
      }}
    >
      <PowerEmblem power={power} />
      {power.name}
      <span className="power-x">{power.x}</span>
    </button>
  );
}
