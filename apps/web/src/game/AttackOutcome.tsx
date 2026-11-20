// R3615000: the hovered attack's preview, one tentative hit marker on each card that takes damage
// (amending R3477000's chip, which wore both numbers on the target).
//
// Picking or lifting an attacker shows nothing yet; hovering a legal target with a forecast entry shows
// that one attack: the target wears its own damage (`dealt`) and the attacker wears what it takes back
// (`taken`). A hero target never strikes back (R3100000), so the attacker shows nothing for one, and a
// strike back of 0 shows no marker on the attacker. Every number is the engine's forecast entry as sent
// (attackOutcome.ts, CLAUDE.md rule 7).
//
// Each marker reuses the real damage hit's design, toned down so it reads as tentative: the `fx-splat`
// starburst with its signed amount when effects are on (`fx/fx.css`, `fx/dom.ts`), the `.damage-pop`
// number when effects are off (`game/animations.css`, `game/board.css`). Same shape, size, font and
// placement, through a `data-tentative` modifier on the same classes (attackOutcome.css); lowered opacity
// and a ghosted fill so it cannot be mistaken for a hit that already landed, and no animation, so it
// holds still while hovered. The skull rides on whichever card the forecast says is destroyed (`kills`
// on the target, `survives: false` on the attacker) and the struck-through shield mark when the blow pops
// one (`pops`). The marker takes no click of its own (a press on it is the card's: nothing inside carries
// a testid, and the wrapper takes no pointer) and sits out of flow, so it moves nothing. The forecast
// reads the face-up board alone, so its title says a face-down trap can still change the outcome.
//
// R4290000: the same attack's other hits wear the target's marker too: each unit a Cleave attack strikes
// beside the target its splash (the entry's `splash`), and the hero a Trample attack on a unit reaches its
// overflow (the entry's `overflow`), each with the skull and the shield mark as the target's.

import type { AttackForecast, SplashHit, TrampleOverflow } from "@jackioh/shared";
import type { ReactElement } from "react";

import { KEYWORD_MARK } from "../cards/keywordMarks.ts";
import { useFxSettings } from "../fx/settings.ts";
import { t } from "../i18n/i18n.ts";
import { reducedMotionNow } from "../settings/motion.ts";
import { useSetting } from "../settings/store.ts";
import { testid } from "./contract.ts";
import "./attackOutcome.css";

/** What each marker's title and accessible name end on: the forecast is no promise. */
export const ATTACK_OUTCOME_NOTE = "What the face-up board predicts. A face-down trap can still change it.";

const SKULL = "☠";
/** The chip's minus (U+2212), kept in every variant: color never carries meaning on its own. */
const MINUS = "−";

/**
 * R3615000: whether the preview draws the splat variant. Exactly `FxLayer.tsx`'s own switch, so the
 * preview follows it: the layer hides `.damage-pop` while effects are on, and the preview renders only
 * the splat then, so exactly one style of marker is ever visible.
 */
function usePreviewFxOn(): boolean {
  const [settings] = useFxSettings();
  // The panel's "Reduce motion" is read through its hook so a change re-renders the preview.
  const panelReduces = useSetting("reduceMotion");
  const reduces = panelReduces || reducedMotionNow(settings);
  return !reduces && settings.intensity !== "off";
}

/** The tentative hit itself: the splat's markup and classes with the tentative modifier, or `.damage-pop`'s. */
function PreviewMark({ amount, skull }: { amount: number; skull?: "kills" | "falls" }): ReactElement {
  const fxOn = usePreviewFxOn();
  if (fxOn) {
    // The splat keeps its exact real markup (class plus `data-amount` only, no text nodes, as
    // `fx/dom.ts` does); the skull rides beside it so the starburst's own rules still match.
    return (
      <>
        <span className="fx-splat" data-amount={`${MINUS}${amount}`} data-tentative="true" aria-hidden="true" />
        {skull !== undefined && (
          <span className="attack-outcome-skull" data-mark={skull} aria-hidden="true">
            {SKULL}
          </span>
        )}
      </>
    );
  }
  return (
    <span className="damage-pop" data-tentative="true" aria-hidden="true">
      {MINUS}
      {amount}
      {skull !== undefined && <span data-mark={skull}>{SKULL}</span>}
    </span>
  );
}

function ShieldMark(): ReactElement {
  return (
    <span className="attack-outcome-shield" data-mark="pops" aria-hidden="true">
      {KEYWORD_MARK["Divine Shield"]}
    </span>
  );
}

/** One hit's marker on the card that takes it: what it deals, whether it destroys, whether it breaks a shield. */
function BlowPreview({
  testId,
  dealt,
  kills,
  pops,
}: {
  testId: string;
  dealt: number;
  kills: boolean;
  pops: boolean;
}): ReactElement {
  const words = [t("Deals {n}", { n: dealt })];
  if (kills) words.push(t("Destroys it"));
  if (pops) words.push(t("Breaks its shield"));
  const note = t(ATTACK_OUTCOME_NOTE);
  return (
    <span
      className="attack-outcome-target"
      role="img"
      data-testid={testId}
      data-tentative="true"
      data-dealt={dealt}
      data-kills={kills ? "true" : undefined}
      data-pops={pops ? "true" : undefined}
      title={note}
      aria-label={[...words, note].join(". ")}
    >
      <PreviewMark amount={dealt} skull={kills ? "kills" : undefined} />
      {pops && <ShieldMark />}
    </span>
  );
}

/** R3615000: the hovered target's marker, wearing the damage the blow deals it. */
export function AttackTargetPreview({ outcome, target }: { outcome: AttackForecast; target: string }): ReactElement {
  return (
    <BlowPreview testId={testid.attackOutcome(target)} dealt={outcome.dealt} kills={outcome.kills} pops={outcome.pops} />
  );
}

/** R4290000: the marker on a unit beside the hovered target, wearing the Cleave hit the attack deals it. */
export function AttackSplashPreview({ hit, unit }: { hit: SplashHit; unit: string }): ReactElement {
  return <BlowPreview testId={testid.attackOutcomeSplash(unit)} dealt={hit.dealt} kills={hit.kills} pops={hit.pops} />;
}

/** R4290000: the marker on the hero the hovered attack tramples onto, wearing its overflow. */
export function AttackOverflowPreview({ overflow, hero }: { overflow: TrampleOverflow; hero: string }): ReactElement {
  return (
    <BlowPreview testId={testid.attackOutcomeOverflow(hero)} dealt={overflow.dealt} kills={overflow.kills} pops={false} />
  );
}

/** R3615000: the hovering attacker's own marker, wearing what the hovered target strikes back. */
export function AttackTakenPreview({
  outcome,
  attacker,
}: {
  outcome: AttackForecast;
  attacker: string;
}): ReactElement {
  const words = [t("Takes {n} back", { n: outcome.taken })];
  if (!outcome.survives) words.push(t("Your Unit is destroyed"));
  const note = t(ATTACK_OUTCOME_NOTE);
  return (
    <span
      className="attack-outcome-taken"
      role="img"
      data-testid={testid.attackOutcomeTaken(attacker)}
      data-tentative="true"
      data-taken={outcome.taken}
      data-falls={!outcome.survives ? "true" : undefined}
      title={note}
      aria-label={[...words, note].join(". ")}
    >
      <PreviewMark amount={outcome.taken} skull={!outcome.survives ? "falls" : undefined} />
    </span>
  );
}
