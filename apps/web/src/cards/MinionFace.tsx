// The board minion (docs/polish/6-cards.md, Surface B "MinionFace DOM"): art up to the frame, cost, name plate,
// attack, health, keyword chips, Divine Shield bubble, Taunt frame. The board's tests and e2e read
// `.stats`, `.stat-attack[data-attack]`, `.stat-health[data-health][data-max-health]` (text "health/max",
// BUILD M5-T1), `.stat-armor[data-armor]`, `.keywords .keyword[data-keyword][data-n][title]`, `.shield-icon`.
// The numbers are the view's (UnitView); the tones only compare them with the printed face.
// Keywords: each keeps a `[data-keyword]` badge, but only those not already drawn as a state get a chip,
// nor (R2577000) those with `onField: false` (keywordVisuals.ts), which leave the "+n" too; at most
// KEYWORD_CHIPS_MAX show (cards.css), the hover preview and inspect sheet list all. R438: a keyword that
// acts on the field also gets a shape treatment `data-keyword-fx="<kind>"` (keywordVisuals.ts, KeywordFx.tsx).
// Vanilla (§6.3, R115, R243): a unit a Vanilla took the text of wears a `.cf-vanilla` stamp; its chips are
// the view's. §10.8: Brittle's count and the Animated cog are keyword treatments; the rest ride
// CardStates.tsx's rail over the name plate: tuned mark (R386), enchantments (E39).
// "zzz" (R849000): `.cf-sick[data-sick]` where the view marks the unit `sick` (summoning sick with neither
// Rush nor Charge, `combat::is_asleep`), both seats. Never read `canAct` for it: false for every unit of the
// non-active player, and a summoning-sick unit may still switch (§4.1). A spent unit of the player acting
// wears only game/spent.ts's cue (#258). Whether a unit can attack is `legalActions`' (CLAUDE.md rule 7).
// ME-CN, R1301: a Chinese unit's name plate and keyword titles are chinese.ts's; chips keep their marks.
// R1839000/R1839002: a Dormant card wears `data-dormant` and DormantSeal.tsx's seal, broken on `awakened`
// until the board catches up (game/awakening.ts's `data-awakening`). R2647000: a token unit wears no
// mark and no Created badge (CardStates.tsx); its hover preview says "Token Unit" (R1177001).
// R1295000: the viewer's own unit in a game (`worn`) wears the device's card cosmetics (CardFace.tsx, cosmetics.ts).
// R3046000: a full-art unit (fullArt.ts `boardLookOf`: the card, its owner's pick and the setting say so, as they do for
// a full face) lays its art across the face, `data-full-art`, still, with the name plate a band below the art, the
// attack and health gems on the art's lower corners and the cost gem, chips, marks and states over it (cards.css).
// #3341: a unit that is not full art draws its art up to the frame as well (the panel's top and sides, down to half a gem
// above its foot), the name plate and the gems laid over its edges, and a rarity ring at its rim, not an oval in a box.
// R3013002: and its card's proficiency stamp, as CardFace does, at the minion's right edge over the name plate.
// R3340000: its art moves while it holds a motion slot, when its card wears an animated or signed level and when it is a Mythic,
// ranked by the pointer, focus and the attack in flight (`useArtMotion`, motionSlots.ts), and answers the pointer, a press and
// `attackDeclared` (artLive.ts, `data-art-hover`, `data-art-attacking`); its art wears cracks for the health it is missing, a
// glow in its card's colours when buffed and frost when frozen (`wear`, art/ArtWear.tsx), from the view's numbers and keywords alone.
// R3335000: a launched Starship's art wears its seat's Starship look, as CardFace's does.
// R3334000: a Dormant Starship with pieces idles in its shipyard over its picture (ShipyardIdle.tsx).

import { useRef, type ReactElement } from "react";

import { hasKeyword, keywordKey, type Keyword, type UnitView } from "@jackioh/shared";

import { CardArt } from "./art/index.ts";
import { damageLevel } from "./art/wear.ts";
import { shipArtOf } from "./art/stinkyscape.ts";
import { CardStates } from "./CardStates.tsx";
import { costDigits, hasCrest } from "./CardFace.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { useArtLive } from "./artLive.ts";
import { cosmeticAttributes, useArtMotion, useCosmeticAppearance, useStarshipLook } from "./cosmetics.ts";
import { slotRarityOf } from "./motionSlots.ts";
import { useFitText } from "./fit.ts";
import { artMotionOf, boardLookOf, boardNameOf, useCardLook } from "./fullArt.ts";
import { KEYWORD_MARK } from "./keywordMarks.ts";
import { Icon } from "./icons.tsx";
import { DormantSeal, dormantAttr } from "./DormantSeal.tsx";
import { StarshipHull } from "./StarshipHull.tsx";
import { KeywordFx } from "./KeywordFx.tsx";
import { KEYWORD_VISUALS, keywordFxAttributes, keywordFxPlan, type KeywordFxPlan } from "./keywordVisuals.ts";
import { foilFor, frameRarity } from "./frame.ts";
import type { FaceModel, StateBadgeKind } from "./model.ts";
import { ProficiencyStamp } from "./ProficiencyStamp.tsx";
import { useCardSettings } from "./settings.ts";
import { ShipyardIdle } from "./ShipyardIdle.tsx";
import { artPictureOf } from "./versions.ts";
import { useCardStamp } from "./stamps.ts";

import "./cards.css";
import "./keywords.css";
import "./cosmetics.css";

export type MinionFaceProps = {
  face: FaceModel;
  unit: UnitView;
  className?: string;
  /** R1295000: the viewer's own unit in a game wears the device's cosmetics (cosmetics.ts). */
  worn?: boolean;
  /**
   * #2932: the keyword marks are drawn elsewhere (`game/FieldBadges.tsx`, hanging off a phone's field
   * card), so the face keeps its whole width for the name, art, cost, attack and health.
   */
  keywordsOffFace?: boolean;
  /** R3340000: this unit's `attackDeclared` is playing (Card.tsx): its art swings toward its target and it ranks first for a motion slot. */
  attacking?: boolean;
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
  // R3546004: a kind with no visual (GIGA, until S26's chip) draws nothing, not a throw.
  const chips = keywords.filter((keyword) => KEYWORD_VISUALS[keyword.kind]?.onField === true && !drawnAsState(keyword, armor));
  const room = chips.length <= KEYWORD_CHIPS_MAX ? chips.length : KEYWORD_CHIPS_MAX - 1;
  return { shown: chips.slice(0, room), folded: chips.slice(room) };
}

/** The keyword marks of a unit (`.keywords > .keyword-icon`, with their "+n" fold); #2932: a phone draws them off the face (`game/FieldBadges.tsx`). */
export function KeywordIcons({
  keywords,
  words,
  join,
  armor,
}: {
  keywords: readonly Keyword[];
  /** Each keyword in the face's words, in `keywords`' order (`FaceModel.keywordWords`; R1301: a Chinese unit's are the table's). */
  words: readonly string[];
  /** What joins a list of them (`FaceModel.keywordJoin`). */
  join: string;
  armor: number;
}): ReactElement | null {
  if (keywords.length === 0) return null;
  const { shown, folded } = keywordChips(keywords, armor);
  const wordsOf = (keyword: Keyword): string => words[keywords.indexOf(keyword)] ?? keywordKey(keyword);
  return (
    <span className="keywords">
      {keywords.map((keyword, index) => (
        <span
          key={`${keywordKey(keyword)}-${index}`}
          className="keyword keyword-icon"
          role="img"
          aria-label={wordsOf(keyword)}
          data-keyword={keyword.kind}
          data-n={"n" in keyword ? keyword.n : undefined}
          data-chip={shown.includes(keyword) ? undefined : "hidden"}
          title={wordsOf(keyword)}
        >
          {KEYWORD_MARK[keyword.kind]}
          {"n" in keyword ? ` ${keyword.n}` : ""}
        </span>
      ))}
      {folded.length > 0 && (
        <span
          className="cf-kw-more"
          role="img"
          aria-label={folded.map(wordsOf).join(join)}
          title={folded.map(wordsOf).join(join)}
        >
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

export function MinionFace({ face, unit, className, worn = false, keywordsOffFace = false, attacking = false }: MinionFaceProps): ReactElement {
  useLocale();
  const settings = useCardSettings();
  const stamp = useCardStamp(face, worn);
  // R3335000: a launched Starship wears the viewer's own Starship picks, or its seat's below a `TheirCopy`.
  const starshipLook = useStarshipLook(worn);
  const plan = keywordFxPlan(unit);
  // R3046000, R3339000: the look a full-art unit wears, still (null: the art up to the frame).
  const cardLook = useCardLook(face.defId, worn);
  const look = boardLookOf(face, cardLook);
  const { artFirst, staticName } = boardNameOf(face, cardLook, settings.boardNames);
  const dormant = dormantAttr(face.dormant);
  const nameRef = useRef<HTMLSpanElement>(null);
  // The hover plate is fitted as the static plate is: the name it shows is whole, or ends in an ellipsis at the floor.
  useFitText(nameRef, staticName ? face.name : `hover ${face.name}`);

  const attackTone = face.stats?.attackTone ?? "base";
  const healthTone = face.stats?.healthTone ?? "base";

  // R3340000: its art moves for an animated or signed level and for a Mythic, in a slot ranked by the pointer, focus or an
  // attack in flight first, then its rarity (motionSlots.ts); a hovered or attacking unit claims one to answer in.
  const faceRef = useRef<HTMLSpanElement>(null);
  const rarity = frameRarity(face);
  const cosmeticAppearance = useCosmeticAppearance(face, worn);
  const artMoves = cardLook.animated || rarity === "Mythic";
  const live = useArtLive(faceRef, true);
  const engaged = live.hovered || attacking;
  // R3334000, R3340000: the shipyard idle is another use of this face's one ranked, visible motion decision, not a
  // second unranked claim of its own. A populated Dormant Starship is Legendary, so it remains eligible at Fx Low.
  const shipyardIdle = face.starship !== null && !face.starship.launched && face.starship.list.length > 0;
  const faceMotion = useArtMotion(
    (artMoves || engaged || shipyardIdle || cosmeticAppearance.animates) && settings.animatedFoil && live.onScreen,
    { hovered: engaged, rarity: slotRarityOf(rarity) },
  );
  const motion = faceMotion.moving && (artMoves || engaged) ? artMotionOf(face.defId) : null;
  const cosmetics = cosmeticAttributes(cosmeticAppearance, faceMotion.moving);
  // R3340000: what its art wears: cracks for the health it is missing, a glow when buffed, frost when frozen (the view's own).
  const damage = damageLevel(unit.health, unit.maxHealth);
  const buffed = attackTone === "buffed" || healthTone === "buffed";
  const frozen = unit.frozen === true;
  const wear = damage > 0 || buffed || frozen ? { damage, buffed, frozen } : null;

  return (
    <span
      ref={faceRef}
      className={className === undefined ? "cf cf--minion" : `cf cf--minion ${className}`}
      data-layout="minion"
      data-card-type={face.type}
      data-rarity={frameRarity(face) ?? undefined}
      data-foil={foilFor(face, settings.animatedFoil, settings.rarityFoil)}
      data-radiant-face={face.radiant ? "true" : undefined}
      data-full-art={look === null ? undefined : "true"}
      data-art-first={artFirst ? "true" : undefined}
      data-version={look?.version ?? undefined}
      data-art-motion={artMoves || engaged ? (faceMotion.moving ? "animated" : "static") : undefined}
      data-art-hover={live.hovered ? "true" : "false"}
      data-art-response={faceMotion.enabled ? "true" : undefined}
      data-art-attacking={attacking ? "true" : undefined}
      data-taunt={hasKeyword(unit.keywords, "Taunt") ? "true" : undefined}
      data-flying={hasKeyword(unit.keywords, "Flying") && unit.position !== "DEF" ? "true" : undefined}
      data-vanilla={unit.vanilla === true ? "true" : undefined}
      data-tuned={face.tuning?.verdict}
      data-dormant={dormant}
      // R3536005: an Evolved unit glows green (cardstate.css).
      data-evolved={unit.evolved === true ? "true" : undefined}
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
            shape={artFirst || look !== null ? "full" : "frame"}
            starship={face.starship}
            look={starshipLook}
            ship={shipArtOf(face.defId, unit)}
            picture={look !== null ? artPictureOf(look.version, face.set) : artFirst ? artPictureOf(cardLook.version, face.set) : "normal"}
            motion={motion}
            live={motion !== null}
            wear={wear}
          />
          {/* R3334000, R3340000: the Dormant Starship's shipyard uses this face's one ranked, on-screen motion slot. */}
          {face.starship != null && !face.starship.launched ? <ShipyardIdle list={face.starship.list} moving={faceMotion.moving} /> : null}
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

        {/* R3339000: the name is a plate that stays (smaller and translucent over an oval window's foot), or the hover plate:
            transparent until hover or focus, in the accessibility tree all the while, so the name is read as it was. */}
        {staticName ? (
          <span className={look === null ? "card-name card-name-compact-translucent" : "card-name"} ref={nameRef}>
            {face.name}
          </span>
        ) : (
          <span className="card-name card-name-hover" ref={nameRef}>
            {face.name}
          </span>
        )}

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

        {keywordsOffFace ? null : <KeywordIcons keywords={unit.keywords} words={face.keywordWords} join={face.keywordJoin} armor={unit.armor} />}

        {/* #2435, R1839000: a Dormant card greyed and bound, under the state rail (DormantSeal.tsx). */}
        {dormant === undefined ? null : <DormantSeal />}

        {/* R2801000: a Dormant Starship's hull, a plate bolted on for each piece fused into it; R3333002: its module's icon. */}
        {face.starship != null && !face.starship.launched ? <StarshipHull list={face.starship.list} /> : null}

        <CardStates face={face} omit={DRAWN_BY_TREATMENTS} />

        {stamp !== null && <ProficiencyStamp level={stamp} />}

        {/* The `divineShieldLost` animation removes this by the keyword leaving the view. */}
        {hasKeyword(unit.keywords, "Divine Shield") && (
          <span className="shield-icon" data-icon="shield" aria-label={t("Divine Shield")} {...fxAttributesOf(plan, "Divine Shield")} />
        )}

      </span>
    </span>
  );
}
