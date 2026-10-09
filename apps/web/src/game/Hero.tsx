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
// The Heroic Powers (R43, R384, R510, R752). The view carries every Heroic Power a player controls,
// each separately once per turn, with its `instanceId`, its name on the card (`title`) and its X. Each of the viewer's own is a button that
// reports `{ on: "activate", instanceId }`, the click every Activate control reports
// (ActivateControl.tsx), so `actions.ts` builds a power exactly as it builds any activation:
// whichever of `activatePower` (R43's alias) or `activate` `legalActions` lists for it, sent at
// once when there is one, or waiting for its target on the board (and draggable to it, game/drag)
// when there are several. The first keeps the `power` testid the e2e specs press; any further one
// is `power-<instanceId>`. The opponent's powers are tags. Whether a button is live is
// `props.highlight.legal`; `usedThisTurn` is drawn, never obeyed. A power flashes on the
// `activated` row, which plays on its card (`card-<instanceId>`).
//
// The portrait is something to look at (R1330, issue #544): a click on the hero when it is not a legal
// target, a long-press of a touch on the portrait, or Enter or Space on the portrait's own button (a
// control named for the hero, laid over the art and given no testid, so a drag or a click still
// resolves to `hero-<side>`) opens the hero's inspect view (`emotes/HeroInspect.tsx`), which holds the
// emote menu or the mute item that the click opened before. The portrait answers with a short squash
// and glint, none under Reduce Motion (R1331). A target is still a target first.

import type { ReactElement } from "react";

import { DEFAULT_PORTRAIT } from "@jackioh/shared";

import { useInspectTrigger } from "../cards/inspect/index.ts";
import { HeroInspect } from "../emotes/HeroInspect.tsx";
import { HeroPortrait, usePortraitReaction } from "../emotes/Portrait.tsx";
import { PORTRAIT_DEFS } from "../emotes/portraits.ts";
import { EmoteShow as EmoteShowEl } from "../emotes/ui.tsx";
import { useSetting } from "../settings/store.ts";
import { ACTIVATED_EVENT } from "./ActivateControl.tsx";
import { animTestid } from "./animations.ts";
import { cx, isLegal, isSelected, legalAttr, PopLayer, type Pops } from "./Card.tsx";
import {
  sideView,
  testid,
  type AnimatingMap,
  type BoardControl,
  type ClickTarget,
  type HeroEmotes,
  type Highlight,
  type Side,
} from "./contract.ts";
import { glowAttr } from "./glow.ts";
import { useOsReducedMotion } from "./useOsReducedMotion.ts";
import type { HeroPowerView, PlayerView } from "@jackioh/shared";

export type { HeroEmotes } from "./contract.ts";

export type HeroProps = {
  view: PlayerView;
  side: Side;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  onControl?: (control: BoardControl) => void;
  pops?: Pops;
  emotes?: HeroEmotes;
};

/** The portrait's long-press has no overlay of its own: it opens the hero's view through Game. */
const NO_OVERLAY = (): null => null;

export default function Hero(props: HeroProps): ReactElement {
  const { view, side } = props;
  const seat = sideView(view, side);
  const hero = seat.hero;
  const emotes = props.emotes;
  const portrait = emotes?.portrait ?? DEFAULT_PORTRAIT;
  const menu = emotes?.menu ?? null;

  const testId = testid.hero(side);
  const legal = isLegal(props.highlight, testId);
  const selected = isSelected(props.highlight, testId);
  const target: ClickTarget = { on: "hero", side };

  const modifiersId = animTestid.modifiers(side);
  const modifiers = seat.modifiers ?? [];

  // R1331: the portrait reacts as its view opens, and not at all under either Reduce Motion switch.
  const panelReduces = useSetting("reduceMotion");
  const osReduces = useOsReducedMotion();
  const reacting = usePortraitReaction(menu !== null, panelReduces || osReduces);
  // R1330: a touch held on the portrait opens the view, unless the hero is a target (a hold there
  // is the player aiming). Hover is off: a portrait has nothing to preview. The trigger's capture
  // swallows the click that lifting the finger sends, which would close the view just opened.
  const press = useInspectTrigger(
    emotes === undefined ? null : { key: `hero-portrait-${side}`, render: NO_OVERLAY },
    { hover: false, longPress: !legal, onLongPress: () => emotes?.onPortrait() },
  );

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
        // Issue §2: targeting always wins. A legal hero is a target, so the click lands on it; a
        // non-legal one opens its inspect view instead (R1330), which holds the emote menu (yours)
        // or the mute item (theirs).
        if (legal) {
          props.onClick?.(target);
        } else {
          emotes?.onPortrait();
        }
      }}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        // A key on a control inside the hero is that control's: the portrait's open button and the
        // Heroic Power's must keep their own Enter and Space.
        if (event.target !== event.currentTarget) return;
        event.preventDefault();
        if (!legal) return;
        props.onClick?.(target);
      }}
    >
      {/* Issue §1: the portrait is the hero's art, with health and armor badged on it. The badges
          are the same `hero-health`/`hero-armor` elements, moved inside the oval. */}
      <HeroPortrait portrait={portrait} health={hero.health} armor={hero.armor} reacting={reacting}>
        {emotes !== undefined && (
          <button
            type="button"
            className="hero-portrait-open"
            aria-label={`Inspect ${PORTRAIT_DEFS[portrait].def.name}, ${side === "you" ? "your hero" : "the opponent's hero"}`}
            aria-haspopup="dialog"
            aria-expanded={menu !== null}
            // A legal hero is the one tab stop (its own tabIndex), the button reached only by a pointer.
            tabIndex={legal ? -1 : undefined}
            {...press.handlers}
          />
        )}
        {emotes?.show !== null && emotes?.show !== undefined && (
          <EmoteShowEl key={emotes.show.key} show={emotes.show} />
        )}
      </HeroPortrait>
      <span className="hero-seat">{side === "you" ? "You" : "Opponent"}</span>
      {(seat.jade ?? 0) > 0 && (
        <span className="hero-jade" data-jade={seat.jade} title="Jade Counter">
          {seat.jade}
        </span>
      )}

      {hero.power !== null &&
        (side === "you" ? (
          // The one `power` testid in the DOM.
          <PowerButton power={hero.power} testId={testid.power} props={props} />
        ) : (
          <span className="power-tag" data-used={hero.power.usedThisTurn ? "true" : "false"} data-x={hero.power.x}>
            {hero.power.title}
            <span className="power-x">{hero.power.x}</span>
          </span>
        ))}

      {/* Any further power this player controls (R43), each its own control on your side. */}
      {(hero.powers ?? [])
        .filter((power) => power.instanceId !== hero.power?.instanceId)
        .map((power) =>
          side === "you" ? (
            <PowerButton key={power.instanceId} power={power} testId={testid.powerOf(power.instanceId)} extra props={props} />
          ) : (
            <span
              key={power.instanceId}
              className="power-tag power-extra"
              data-instance-id={power.instanceId}
              data-used={power.usedThisTurn ? "true" : "false"}
              data-x={power.x}
            >
              {power.title}
              <span className="power-x">{power.x}</span>
            </span>
          ),
        )}

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

      {emotes !== undefined && menu !== null && (
        <HeroInspect
          side={side}
          portrait={portrait}
          health={hero.health}
          armor={hero.armor}
          menu={menu}
          hand={emotes.hand}
          gate={emotes.gate}
          muted={emotes.muted}
          onPick={emotes.onPick}
          onMute={emotes.onMute}
          onClose={emotes.onCloseMenu}
        />
      )}
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
  extra = false,
  props,
}: {
  power: HeroPowerView;
  testId: string;
  extra?: boolean;
  props: HeroProps;
}): ReactElement {
  const live = isLegal(props.highlight, testId);
  const flashing = props.animating?.get(testid.card(power.instanceId)) === ACTIVATED_EVENT;
  return (
    <button
      type="button"
      className={cx("power-button", extra && "power-extra")}
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
      title={`${power.title}: Activate, spend (${power.x})`}
      onClick={(event) => {
        event.stopPropagation();
        if (!live) return;
        props.onClick?.({ on: "activate", instanceId: power.instanceId });
      }}
    >
      {power.title}
      <span className="power-x">{power.x}</span>
    </button>
  );
}
