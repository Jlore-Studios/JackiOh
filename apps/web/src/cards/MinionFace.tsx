// The board minion (docs/polish/6-cards.md, Surface B "MinionFace DOM"): an oval portrait with its
// cost, a name plate, attack and health, keyword chips, the Divine Shield bubble and a Taunt frame
// — Hearthstone's minion on the board.
//
// Every element the board's tests and the e2e specs read lives here under its old name:
// `.stats`, `.stat-attack[data-attack]`, `.stat-health[data-health][data-max-health]` whose text is
// still "health/max" (BUILD M5-T1's "current over max"), `.stat-armor[data-armor]`,
// `.keywords .keyword[data-keyword][data-n][title]` and `.shield-icon`. The numbers are the
// view's (UnitView) and nothing else; the tones only compare them with the printed face.
//
// Keywords. Every keyword keeps its `[data-keyword]` badge in the DOM, but only the ones the minion
// does not already draw as a state get a visible chip: Taunt is the shield frame, Divine Shield the
// bubble, Armor the steel plate by the health gem. At most KEYWORD_CHIPS_MAX chips show, legible
// at a pixel floor (cards.css); past that the last one becomes a "+n" count, and the hover preview
// and the inspect sheet list them all.
//
// Keyword visuals (R438). Beside its chip, every keyword has a treatment on the minion drawn by
// shape: a frame round the portrait, a veil over its art, a glyph in the top row, or Armor's plate
// (keywordVisuals.ts has the map, the layers' caps and the cap on loops; KeywordFx.tsx draws them;
// keywords.css their look and motion). Each carries `data-keyword-fx="<kind>"`: Taunt's shield,
// which used to be the portrait's ::before, is now its own treatment, and the Divine Shield bubble
// and the Armor plate carry the attribute on their existing elements.
//
// Vanilla. A unit a Vanilla took the text of (§6.3, R115) is marked Vanilla in the view
// (`UnitView.vanilla`, R243), since its definition still names what it lost: the minion wears a
// plain "Vanilla" stamp at its portrait's corner (`.cf-vanilla`, a "V" on a minion too small for
// the word), and its hover preview's rules box says the text is gone. Its keyword chips are the view's, which already leave the lost ones out.
//
// Patch v0.2.0's states (SPEC §10.8). Brittle's count and the Animated cog are keyword treatments
// above (the view's `brittle` count draws the cracks even when no Brittle keyword lists it). The rest
// ride a small rail of badges just over the name plate (CardStates.tsx): the tuned mark (▲ Buffed,
// ▼ Nerfed, ◆ Tuned, R386) and the enchantments (E39). A tuned stat carries `data-tuned` and a ▲ or
// ▼ pip beside its tone (cardstate.css), the number itself unchanged.
//
// The face draws no "zzz" itself. `canAct` is false for every unit whose controller is not the
// active player, and a summoning-sick unit may still switch (§4.1), so read bare it can neither say
// "this unit is asleep" nor "this one can attack". The board's card root draws the "can't act yet"
// cue only where it means something, a unit of the player acting now with no action left
// (game/spent.ts, #258). Whether a unit can attack is `legalActions`', drawn by the board's
// highlight (task 7's green), never read off the view here (CLAUDE.md rule 7).
//
// ME-CN, R1301: a Chinese unit's name plate is the model's Chinese name and its keyword titles are
// the table's words (chinese.ts); its chips keep their marks, and its art the English name's motif.

import { useRef, type ReactElement } from "react";

import { hasKeyword, keywordKey, type Keyword, type UnitView } from "@jackioh/shared";

import { CardArt } from "./art/index.ts";
import type { StateBadgeKind } from "./cardState.ts";
import { CardStates } from "./CardStates.tsx";
import { costDigits, hasCrest } from "./CardFace.tsx";
import { CHINESE_COMMA, chineseKeyword } from "./chinese.ts";
import { useFitText } from "./fit.ts";
import { KEYWORD_MARK } from "./glossary.ts";
import { Icon } from "./icons.tsx";
import { KeywordFx } from "./KeywordFx.tsx";
import { keywordFxAttributes, keywordFxPlan, type KeywordFxPlan } from "./keywordVisuals.ts";
import { foilFor, frameRarity, type FaceModel } from "./model.ts";
import { useCardSettings } from "./settings.ts";

import "./cards.css";
import "./keywords.css";

export type MinionFaceProps = { face: FaceModel; unit: UnitView; className?: string };

/** The states the minion's keyword treatments already draw (Brittle's cracks, the Animated cog). */
const DRAWN_BY_TREATMENTS: readonly StateBadgeKind[] = ["brittle", "animated"];

/** How many keyword chips a minion shows before the last becomes a "+n" count. */
export const KEYWORD_CHIPS_MAX = 3;

/** Keywords the minion draws as a state rather than a chip: the Taunt frame, the Divine Shield bubble, the Armor plate. */
export function drawnAsState(keyword: Keyword, armor: number): boolean {
  if (keyword.kind === "Taunt" || keyword.kind === "Divine Shield") return true;
  return keyword.kind === "Armor" && armor > 0;
}

/** The keywords that get a visible chip, and the ones folded into the "+n" count. */
export function keywordChips(keywords: readonly Keyword[], armor: number): { shown: Keyword[]; folded: Keyword[] } {
  const chips = keywords.filter((keyword) => !drawnAsState(keyword, armor));
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

export function MinionFace({ face, unit, className }: MinionFaceProps): ReactElement {
  const settings = useCardSettings();
  const plan = keywordFxPlan(unit);
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
      data-vanilla={unit.vanilla === true ? "true" : undefined}
      data-tuned={face.tuning?.verdict}
    >
      <span className="cf-scale">
        <span className="cf-portrait">
          <CardArt defId={face.defId} name={face.englishName ?? face.name} radiant={face.radiant} tags={face.tags} type={face.type} shape="oval" />
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

        <span className="card-name" ref={nameRef}>
          {face.name}
        </span>

        {unit.vanilla === true && (
          <span className="cf-vanilla" title="Vanilla: its text is gone">
            <span className="cf-vanilla-word">Vanilla</span>
            <span className="cf-vanilla-mark" aria-hidden="true">
              V
            </span>
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

        <CardStates face={face} omit={DRAWN_BY_TREATMENTS} />

        {/* The `divineShieldLost` animation removes this by the keyword leaving the view. */}
        {hasKeyword(unit.keywords, "Divine Shield") && (
          <span className="shield-icon" data-icon="shield" aria-label="Divine Shield" {...fxAttributesOf(plan, "Divine Shield")} />
        )}

      </span>
    </span>
  );
}
