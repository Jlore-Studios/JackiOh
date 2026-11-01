// The board minion (docs/polish/6-cards.md, Surface B "MinionFace DOM"): oval portrait, cost, name plate,
// attack, health, keyword chips, Divine Shield bubble, Taunt frame. The board's tests and e2e read
// `.stats`, `.stat-attack[data-attack]`, `.stat-health[data-health][data-max-health]` (text "health/max",
// BUILD M5-T1), `.stat-armor[data-armor]`, `.keywords .keyword[data-keyword][data-n][title]`, `.shield-icon`.
// The numbers are the view's (UnitView); the tones only compare them with the printed face.
// Keywords: each keeps a `[data-keyword]` badge, but only those not already drawn as a state get a chip,
// nor (R2577000) those with `onField: false` (keywordVisuals.ts), which leave the "+n" too; at most
// KEYWORD_CHIPS_MAX show (cards.css), the hover preview and inspect sheet list all. R438: a keyword that
// acts on the field also gets a shape treatment `data-keyword-fx="<kind>"` (keywordVisuals.ts, KeywordFx.tsx).
// Vanilla (§6.3, R115, R243): a unit a Vanilla took the text of is marked in the view and wears a
// `.cf-vanilla` stamp; its chips are the view's. §10.8: Brittle's count and the Animated cog are keyword
// treatments; the rest ride CardStates.tsx's rail over the name plate: tuned mark (R386), enchantments (E39).
// "zzz" (R849000): `.cf-sick[data-sick]` where the view marks the unit `sick` (summoning sick with neither
// Rush nor Charge, `combat::is_asleep`), both seats. Never read `canAct` for it: false for every unit of the
// non-active player, and a summoning-sick unit may still switch (§4.1). A spent unit of the player acting
// wears only game/spent.ts's cue (#258). Whether a unit can attack is `legalActions`' (CLAUDE.md rule 7).
// ME-CN, R1301: a Chinese unit's name plate and keyword titles are chinese.ts's; chips keep their marks.
// R1839000/R1839002: a Dormant card wears `data-dormant` and DormantSeal.tsx's seal, broken on `awakened`
// until the board catches up (game/awakening.ts's `data-awakening`). R2647000: a token unit wears no
// mark and no Created badge (CardStates.tsx); its hover preview says "Token Unit" (R1177001).
// R1295000: the viewer's own unit in a game (`worn`) wears the device's card cosmetics (CardFace.tsx, cosmetics.ts).

import { useRef, type ReactElement } from "react";

import { hasKeyword, keywordKey, type Keyword, type UnitView } from "@jackioh/shared";

import { CardArt } from "./art/index.ts";
import type { StateBadgeKind } from "./cardState.ts";
import { CardStates } from "./CardStates.tsx";
import { costDigits, hasCrest } from "./CardFace.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { CHINESE_COMMA, chineseKeyword } from "./chinese.ts";
import { useCosmeticAttributes } from "./cosmetics.ts";
import { useFitText } from "./fit.ts";
import { KEYWORD_MARK } from "./glossary.ts";
import { Icon } from "./icons.tsx";
import { DormantSeal, dormantAttr } from "./DormantSeal.tsx";
import { KeywordFx } from "./KeywordFx.tsx";
import { KEYWORD_VISUALS, keywordFxAttributes, keywordFxPlan, type KeywordFxPlan } from "./keywordVisuals.ts";
import { foilFor, frameRarity, type FaceModel } from "./model.ts";
import { useCardSettings } from "./settings.ts";

import "./cards.css";
import "./keywords.css";
import "./cosmetics.css";

export type MinionFaceProps = {
  face: FaceModel;
  unit: UnitView;
  className?: string;
  /** R1295000: the viewer's own unit in a game wears the device's cosmetics (cosmetics.ts). */
  worn?: boolean;
};

/** The states the minion's keyword treatments already draw (Brittle's cracks, the Animated cog). */
const DRAWN_BY_TREATMENTS: readonly StateBadgeKind[] = ["brittle", "animated"];

/** How many keyword chips a minion shows before the last becomes a "+n" count. */
export const KEYWORD_CHIPS_MAX = 3;

/** R849000: the zzz's words. */
export const SICK_WORDS = "Summoning sick: it can't attack this turn";

/** Keywords the minion draws as a state rather than a chip: the Taunt frame, the Divine Shield bubble, the Armor plate. */
export function drawnAsState(keyword: Keyword, armor: number): boolean {
  if (keyword.kind === "Taunt" || keyword.kind === "Divine Shield") return true;
  return keyword.kind === "Armor" && armor > 0;
}

/** The keywords that get a visible chip, and the ones folded into the "+n" count; a keyword that does nothing on the field (R2577000) is neither. */
export function keywordChips(keywords: readonly Keyword[], armor: number): { shown: Keyword[]; folded: Keyword[] } {
  const chips = keywords.filter((keyword) => KEYWORD_VISUALS[keyword.kind].onField && !drawnAsState(keyword, armor));
  const room = chips.length <= KEYWORD_CHIPS_MAX ? chips.length : KEYWORD_CHIPS_MAX - 1;
  return { shown: chips.slice(0, room), folded: chips.slice(room) };
}

function KeywordIcons({ keywords, armor, chinese }: { keywords: readonly Keyword[]; armor: number; chinese: boolean }): ReactElement | null {
  if (keywords.length === 0) return null;
  const { shown, folded } = keywordChips(keywords, armor);
  // R1301: a Chinese unit's keywords are named in the table's words.
  const words = chinese ? chineseKeyword : keywordKey;
  return (
    <span className="keywords">
      {keywords.map((keyword, index) => (
        <span
          key={`${keywordKey(keyword)}-${index}`}
          className="keyword keyword-icon"
          data-keyword={keyword.kind}
          data-n={"n" in keyword ? keyword.n : undefined}
          data-chip={shown.includes(keyword) ? undefined : "hidden"}
          title={words(keyword)}
        >
          {KEYWORD_MARK[keyword.kind]}
          {"n" in keyword ? ` ${keyword.n}` : ""}
        </span>
      ))}
      {folded.length > 0 && (
        <span className="cf-kw-more" title={folded.map(words).join(chinese ? CHINESE_COMMA : ", ")}>
          +{folded.length}
        </span>
      )}
    </span>
  );
}

/** The attributes of `kind`'s treatment, when the plan draws one. */
function fxAttributesOf(plan: readonly KeywordFxPlan[], kind: Keyword["kind"]): Record<string, string> {
  const entry = plan.find((candidate) => candidate.kind === kind);
  return entry === undefined ? {} : keywordFxAttributes(entry);
}

export function MinionFace({ face, unit, className, worn = false }: MinionFaceProps): ReactElement {
  useLocale();
  const settings = useCardSettings();
  const cosmetics = useCosmeticAttributes(face, worn, settings.animatedFoil);
  const plan = keywordFxPlan(unit);
  const dormant = dormantAttr(face.dormant);
  const nameRef = useRef<HTMLSpanElement>(null);
  useFitText(nameRef, face.name);

  const attackTone = face.stats?.attackTone ?? "base";
  const healthTone = face.stats?.healthTone ?? "base";

  return (
    <span
      className={className === undefined ? "cf cf--minion" : `cf cf--minion ${className}`}
      data-layout="minion"
      data-card-type={face.type}
      data-rarity={frameRarity(face) ?? undefined}
      data-foil={foilFor(face, settings.animatedFoil)}
      data-radiant-face={face.radiant ? "true" : undefined}
      data-taunt={hasKeyword(unit.keywords, "Taunt") ? "true" : undefined}
      data-flying={hasKeyword(unit.keywords, "Flying") && unit.position !== "DEF" ? "true" : undefined}
      data-vanilla={unit.vanilla === true ? "true" : undefined}
      data-tuned={face.tuning?.verdict}
      data-dormant={dormant}
      {...cosmetics}
    >
      <span className="cf-scale">
        <span className="cf-portrait">
          <CardArt
            defId={face.defId}
            name={face.englishName ?? face.name}
            radiant={face.radiant}
            tags={face.tags}
            type={face.type}
            shape="oval"
            starship={face.starship}
          />
        </span>

        <KeywordFx plan={plan} />

        {hasCrest(face) && (
          <span className="cf-crest">
            <Icon name="crest" />
          </span>
        )}

        <span className="cost-gem" data-cost={face.cost.value} data-tone={face.cost.tone} data-digits={costDigits(face.cost.text)}>
          {face.cost.text}
        </span>

        {face.element != null && (
          <span className="cf-element" data-testid="card-element" title={t("Element {element}", { element: t(face.element) })}>
            {t(face.element)}
          </span>
        )}

        <span className="card-name" ref={nameRef}>
          {face.name}
        </span>

        {unit.vanilla === true && (
          <span className="cf-vanilla" title={t("Vanilla: its text is gone")}>
            <span className="cf-vanilla-word">{t("Vanilla")}</span>
            <span className="cf-vanilla-mark" aria-hidden="true">
              V
            </span>
          </span>
        )}

        {/* R849000: summoning sick with neither Rush nor Charge (`UnitView.sick`), Hearthstone's zzz. */}
        {unit.sick === true && (
          <span className="cf-sick" data-sick="true" role="img" aria-label={t(SICK_WORDS)} title={t(SICK_WORDS)}>
            <span aria-hidden="true">zzz</span>
          </span>
        )}

        <span className="stats">
          <span className="stat stat-attack" data-attack={unit.attack} data-tone={attackTone} data-tuned={face.tuning?.attack}>
            {unit.attack}
          </span>
          <span
            className="stat stat-health"
            data-health={unit.health}
            data-max-health={unit.maxHealth}
            data-tone={healthTone}
            data-tuned={face.tuning?.health}
          >
            {unit.health}
            <span className="cf-max">/{unit.maxHealth}</span>
          </span>
          {unit.armor > 0 && (
            <span className="stat stat-armor" data-armor={unit.armor} {...fxAttributesOf(plan, "Armor")}>
              {unit.armor}
            </span>
          )}
        </span>

        <KeywordIcons keywords={unit.keywords} armor={unit.armor} chinese={face.chinese === true} />

        {/* #2435, R1839000: a Dormant card greyed and bound, under the state rail (DormantSeal.tsx). */}
        {dormant === undefined ? null : <DormantSeal />}

        <CardStates face={face} omit={DRAWN_BY_TREATMENTS} />

        {/* The `divineShieldLost` animation removes this by the keyword leaving the view. */}
        {hasKeyword(unit.keywords, "Divine Shield") && (
          <span className="shield-icon" data-icon="shield" aria-label={t("Divine Shield")} {...fxAttributesOf(plan, "Divine Shield")} />
        )}

      </span>
    </span>
  );
}
