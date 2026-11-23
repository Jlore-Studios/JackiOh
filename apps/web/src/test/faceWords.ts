// The words a face's states and changes are said in, asked of the face model (jackioh-shared `face/state.rs`,
// `face/tuning.rs`) the way a test names them: each helper builds the smallest face that has the state and
// reads the answer off it. The web keeps no wording of its own for any of them.

import type { CardDef, CardType, Enchantment, QuestView, Tuning } from "@jackioh/shared";

import { faceModel, type FaceModel, type InPlay, type StateBadgeKind } from "../cards/model.ts";
import { legacyText, type TextRange, type TunedRange } from "./faceData.ts";
import type { FaceTuning } from "../wire/generated/FaceTuning.ts";

export const BERSERK_WORDS = "Berserk: at the start and end of its controller's turn it attacks its own hero";
export const FROZEN_WORDS = "Frozen: can't attack, switch position or use its Activate; it thaws at the end of the turn it could have acted in";
export const FROZEN_FOREVER_WORDS = "Frozen forever: can't attack, switch position or use its Activate, and never thaws";
export const DORMANT_WORDS = "Dormant: it can't attack or be attacked and nothing reaches it, until an effect awakens it";
export const DORMANT_NEXT_TURN_WORDS = "Dormant: it awakens at the start of its controller's next turn; until then nothing reaches it";
export const FROZEN_GLYPH = "❄︎";
export const EVOLVED_WORDS =
  "Evolved: its Evolve has happened, or happens as it enters the field; it evolves only once";
// R3536005: the star (U+2726) in text presentation (U+FE0E), as the shared face model draws it.
export const EVOLVED_GLYPH = "\u2726\uFE0E";
export const DORMANT_GLYPH = "☾︎";
export const STARSHIP_GLYPH = "☄︎";
export const TEMPORARY_GLYPH = "⌛︎";
export const PILE_WORDS = "Cards buried under this pile";

const HOST = "core-001";

/** A face of an empty definition with `inPlay` on it. */
function host(inPlay: InPlay, extra: { radiant?: boolean; type?: CardType } = {}): FaceModel {
  return faceModel({ defId: HOST, radiant: extra.radiant ?? false, ...(extra.type === undefined ? {} : { type: extra.type }), inPlay });
}

function words(kind: StateBadgeKind, face: FaceModel): string {
  const badge = face.badges.find((one) => one.kind === kind);
  if (badge === undefined) throw new Error(`the face has no ${kind} badge`);
  return badge.words;
}

export const stateBadges = (face: FaceModel): FaceModel["badges"] => face.badges;
export const brittleWords = (count: number): string => words("brittle", host({ brittle: count }));
export const temporaryWords = (count: number): string =>
  words("temporary", faceModel({ defId: HOST, radiant: false, inPlay: { inHand: true, handKeywords: [{ kind: "Temporary", n: count }] } }));
export const dormantWords = (turns: number | null): string => words("dormant", host({ dormant: { turns } }));
export const starshipWords = (pieces: number, launched: boolean): string =>
  words("starship", host({ starship: { pieces, name: launched ? "Voyager" : null } }));
export const animatedWords = (animated: { home?: number }, type: CardType): string => words("animated", host({ animated }, { type }));
export function enchantmentWords(enchantment: Enchantment, radiant = false): string {
  const badge = host({ enchantments: [enchantment] }, { radiant }).badges[0];
  if (badge === undefined) throw new Error("the face has no enchantment badge");
  return badge.words;
}
export const questProgress = (quest: { progress: number; goal: number }): string => `${String(quest.progress)}/${String(quest.goal)}`;
export const questWords = (quest: QuestView["open"][number]): string => words("quest", host({ quest: { open: [quest], auras: [] } }));
export const stageWords = (stages: NonNullable<QuestView["stages"]>): string => words("questStage", host({ quest: { open: [], auras: [], stages } }));

/** What changed on a card, as the face model works it out from its view's `tuning` and `params`. */
export function faceTuning(def: CardDef, radiant: boolean, tuning: Tuning | undefined, values: Readonly<Record<string, number>> | undefined): FaceTuning | null {
  return faceModel({
    defId: def.id,
    def,
    radiant,
    inPlay: { ...(tuning === undefined ? {} : { tuning }), ...(values === undefined ? {} : { params: { ...values } }) },
  }).tuning;
}

export const glossaryFor = (face: FaceModel): FaceModel["glossary"] => face.glossary;

/** R2450000: what a Quest card's rules box prints, with the marks and moved numbers in it, or null when it prints its text as it is. */
export function questFaceText(face: FaceModel): { full: string; marks: TextRange[]; tuned?: TunedRange[]; less: boolean } | null {
  return face.questBox === null ? null : { ...legacyText(face.questBox.text), less: face.questBox.less };
}
