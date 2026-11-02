// A hero: health, armor and the Heroic Power (SPEC §3, BUILD M5-T1).
//
// A click target like any card, `hero-<side>`, for a click or a DragLayer drag; whether it may be attacked is
// `props.highlight.legal` (Taunt lives in the engine). Modifier badges are `SideView.modifiers` (R169, §10.1,
// both seats), rendered here only, one `modifiers-<side>` element per seat; one rebuilt from the event window
// (§10.8) could appear and never leave. Heroic Powers (R43, R384, R510, R752) are each once per turn: the first keeps
// the `power` testid e2e presses, others `power-<instanceId>`; the opponent's are tags; `usedThisTurn` is drawn,
// never obeyed; the flash plays on `card-<instanceId>`. Portrait (R1449): a click, Enter or Space on a non-target
// toggles the emote menu (R643: the opponent's is a one-item mute); a hold opens `emotes/HeroInspect.tsx` (R1330).
// R847: "Rift turn used" (`SideView.riftExtraTurn`); owed extra turns are the modifier badge (R846). R1840005: a
// seat's Credits (spent by R1853003's Marketplace, earned by R2250001's Resource) show as `market-credits-<side>`,
// even at 0; a Resource is a hand card (#2250, R2250000), so the hand draws it.

import type { ReactElement } from "react";

import { DEFAULT_PORTRAIT } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { useInspectTrigger } from "../cards/inspect/index.ts";
import { HeroInspect } from "../emotes/HeroInspect.tsx";
import { HeroPortrait, usePortraitReaction } from "../emotes/Portrait.tsx";
import { PORTRAIT_DEFS } from "../emotes/portraits.ts";
import { EmoteMenu, EmoteShow as EmoteShowEl, MuteMenu } from "../emotes/ui.tsx";
import { useSetting } from "../settings/store.ts";
import { ACTIVATED_EVENT } from "./ActivateControl.tsx";
import { animTestid } from "./animations.ts";
import { ModifierBadge } from "./ModifierBadge.tsx";
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
import { secretBadgeText } from "./secrets.ts";
import { t } from "../i18n/i18n.ts";
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

/** The portrait's hold has no overlay of its own: it opens the hero's view through Game. */
const NO_OVERLAY = (): null => null;

/** R847: the badge a side wears once a Temporal Rift has granted it its one extra turn, and its tooltip. */
export const RIFT_SPENT_LABEL = "Rift turn used";
export const RIFT_SPENT_NOTE = "Temporal Rift has granted this player's one extra turn this game; another Rift grants none";

/** R1449: the keys that open the inspect view from the portrait's button, a hold's keyboard twin. */
function isInspectKey(event: { key: string; shiftKey: boolean }): boolean {
  return event.key === "ContextMenu" || (event.shiftKey && event.key === "F10");
}

export default function Hero(props: HeroProps): ReactElement {
  const { view, side } = props;
  const seat = sideView(view, side);
  const hero = seat.hero;
  const emotes = props.emotes;
  const portrait = emotes?.portrait ?? DEFAULT_PORTRAIT;
  const menu = emotes?.menu ?? null;
  const inspect = emotes?.inspect === true;
  const portraitName = readerName(PORTRAIT_DEFS[portrait].def);

  const testId = testid.hero(side);
  const legal = isLegal(props.highlight, testId);
  const selected = isSelected(props.highlight, testId);
  const target: ClickTarget = { on: "hero", side };

  const modifiersId = animTestid.modifiers(side);
  const modifiers = seat.modifiers ?? [];

  // R1331: the portrait reacts as its menu or its view opens, and not at all under either Reduce
  // Motion switch.
  const panelReduces = useSetting("reduceMotion");
  const osReduces = useOsReducedMotion();
  const reacting = usePortraitReaction(menu !== null, panelReduces || osReduces);
  // R1449: a touch or mouse held on the portrait opens the view, unless the hero is a target (a hold there
  // is aiming); no hover. The trigger's capture swallows the click a lift sends, which would toggle the menu;
  // menus stop their own pointerdowns. A right-click, the context-menu key and Shift+F10 are the hold's twins;
  // Game ignores all three while anything is targeted (the drag layer takes the right-click to cancel).
  const press = useInspectTrigger(
    emotes === undefined ? null : { key: `hero-portrait-${side}`, render: NO_OVERLAY },
    {
      hover: false,
      longPress: !legal,
      holdAnyPointer: true,
      onLongPress: () => emotes?.onPortraitHold(),
      onContextMenu: () => emotes?.onPortraitHold(),
    },
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
      // No aria-disabled while the hero is not a target, as a card carries: the hero holds controls
      // of its own (the portrait's button, its menu, the Heroic Power), and a disabled ancestor would
      // have assistive tech read every one of them as disabled (R1449). Not being a tab stop says it.
      aria-label={side === "you" ? t("Your hero") : t("Opponent hero")}
      tabIndex={legal ? 0 : undefined}
      onClick={() => {
        // Issue §2: targeting always wins. A legal hero is a target, so the click lands on it; a
        // non-legal one toggles its menu on the portrait instead (R1449).
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
      <HeroPortrait
        portrait={portrait}
        health={hero.health}
        armor={hero.armor}
        luck={seat.luck}
        reacting={reacting}
        press={emotes === undefined ? undefined : press.handlers}
      >
        {emotes !== undefined && (
          <button
            type="button"
            className="hero-portrait-open"
            aria-label={
              side === "you"
                ? t("Emotes for {name}, your hero; hold, or Shift+F10, to inspect", { name: portraitName })
                : t("Mute emotes for {name}, the opponent's hero; hold, or Shift+F10, to inspect", { name: portraitName })
            }
            aria-haspopup="menu"
            aria-expanded={menu !== null}
            title={
              side === "you"
                ? t("{name}: click for emotes, hold to inspect", { name: portraitName })
                : t("{name}: click for the mute, hold to inspect", { name: portraitName })
            }
            // A legal hero is the one tab stop (its own tabIndex), the button reached only by a pointer.
            tabIndex={legal ? -1 : undefined}
            onKeyDown={(event) => {
              if (!isInspectKey(event)) return;
              event.preventDefault();
              if (!legal) emotes.onPortraitHold();
            }}
          />
        )}
        {emotes?.show !== null && emotes?.show !== undefined && (
          <EmoteShowEl key={emotes.show.key} show={emotes.show} />
        )}
        {/* R1449: the click's menu, hung off the portrait as it was before R1330. A hold puts the
            same menu in the inspect view below instead. */}
        {emotes !== undefined && menu === "emotes" && !inspect && (
          <EmoteMenu side={side} hand={emotes.hand} gate={emotes.gate} onPick={emotes.onPick} onClose={emotes.onCloseMenu} />
        )}
        {emotes !== undefined && menu === "mute" && !inspect && (
          <MuteMenu muted={emotes.muted} onMute={emotes.onMute} onClose={emotes.onCloseMenu} />
        )}
      </HeroPortrait>
      <span className="hero-seat">{side === "you" ? t("You") : t("Opponent")}</span>
      {(seat.jade ?? 0) > 0 && (
        <span className="hero-jade" data-jade={seat.jade} title={t("Jade Counter")}>
          {seat.jade}
        </span>
      )}
      {/* R1840005: the Credits, 0 included, once the view carries them. */}
      {seat.marketCredits !== undefined && (
        <span
          className="hero-credits"
          data-testid={`market-credits-${side}`}
          data-credits={seat.marketCredits}
          title={t("Credits")}
        >
          {seat.marketCredits}
        </span>
      )}

      {hero.power !== null &&
        (side === "you" ? (
          // The one `power` testid in the DOM.
          <PowerButton power={hero.power} testId={testid.power} props={props} />
        ) : (
          <span
            className="power-tag"
            data-used={hero.power.usedThisTurn ? "true" : "false"}
            data-x={hero.power.x}
            title={hero.power.usedThisTurn ? usedTitle(hero.power) : undefined}
          >
            <UsedMark power={hero.power} />
            {t(hero.power.title)}
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
              title={power.usedThisTurn ? usedTitle(power) : undefined}
            >
              <UsedMark power={power} />
              {t(power.title)}
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
        // WCAG 4.1.2 (#757): a name needs a role.
        role="group"
        aria-label={side === "you" ? t("Your modifiers") : t("Opponent modifiers")}
      >
        {/* #2321: each opens its whole label on a tap, which a touch screen's missing hover cannot. */}
        {modifiers.map((modifier) => (
          <ModifierBadge key={modifier.id} modifier={modifier} />
        ))}
      </span>

      {/* R860: one badge per secret this seat holds, in the view's order. The badge names the
          choice only where the view carries it — the owner's seat, or anyone once revealed —
          and the opponent's badge says only "Secret". */}
      {(seat.secrets?.length ?? 0) > 0 && (
        <span className="modifiers secrets" data-testid={`secrets-${side}`}>
          {(seat.secrets ?? []).map((secret) => (
            <span
              key={secret.id}
              className="modifier-badge secret-badge"
              data-secret-id={secret.id}
              data-choice={secret.choice}
            >
              {secretBadgeText(secret.choice)}
            </span>
          ))}
        </span>
      )}

      {/* R847: the once-a-game Temporal Rift flag, public on both seats. Outside `modifiers-<side>`,
          whose badges are the view's modifiers (R169). */}
      {seat.riftExtraTurn === true && (
        <span className="modifiers rift" data-testid={`rift-spent-${side}`}>
          <span className="modifier-badge rift-badge" data-rift-extra-turn="true" title={t(RIFT_SPENT_NOTE)}>
            {t(RIFT_SPENT_LABEL)}
          </span>
        </span>
      )}

      <PopLayer pops={props.pops} />

      {emotes !== undefined && menu !== null && inspect && (
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
 * R1639000: the mark a used Heroic Power wears, first in its chip so the chip's clip never takes it. The
 * space keeps the name "Used Ranching" (a flex box draws none; Board.tsx's Pile).
 */
function UsedMark({ power }: { power: HeroPowerView }): ReactElement | null {
  if (!power.usedThisTurn) return null;
  return (
    <>
      <span className="power-used">{t("Used")}</span>{" "}
    </>
  );
}

function usedTitle(power: HeroPowerView): string {
  return t("{power}: used this turn", { power: t(power.title) });
}

/**
 * One of the viewer's Heroic Powers. A press reports its instance as an activation, never the `power` board
 * control; `data-instance-id` is also what a drag from it reads (game/drag/targets.ts).
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
      title={power.usedThisTurn ? usedTitle(power) : t("{power}: Activate, spend ({x})", { power: t(power.title), x: power.x })}
      onClick={(event) => {
        event.stopPropagation();
        if (!live) return;
        props.onClick?.({ on: "activate", instanceId: power.instanceId });
      }}
    >
      <UsedMark power={power} />
      {t(power.title)}
      <span className="power-x">{power.x}</span>
    </button>
  );
}
