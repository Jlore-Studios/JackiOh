// What a card face shows (docs/polish/6-cards.md, Surface B), worked out in Rust: `jackioh-shared`'s `face`
// module (#3106), which the web reaches through the WebAssembly module's `face` body (`wasm/index.ts`). The
// model is final: its name, text, glossary, badges and words are in the reader's language already, and its
// text is spans a component draws. Nothing here is a rule or a wording (CLAUDE.md rule 7); the web lays the
// result out and keeps the art, the animation and the audio.
//
// A face with no `inPlay` is the card as printed (SPEC §10.10); with `inPlay`, as the view says it stands
// (R243). The language is the reader's (R711000): a face is rebuilt where it is drawn, never kept across a
// language switch. Type definitions are the generated ones (`wire/generated`, ts-rs).

import { getLocale } from "../i18n/i18n.ts";
import { face } from "../wasm/index.ts";
import type { FaceModel } from "../wire/generated/FaceModel.ts";
import type { FaceSource } from "../wire/generated/FaceSource.ts";
import type { RichText } from "../wire/generated/RichText.ts";
import type { Utf16Range } from "../wire/generated/Utf16Range.ts";

export type { FaceModel } from "../wire/generated/FaceModel.ts";
export type { FaceSource } from "../wire/generated/FaceSource.ts";
export type { InPlay } from "../wire/generated/InPlay.ts";
export type { FaceCost } from "../wire/generated/FaceCost.ts";
export type { FaceStats } from "../wire/generated/FaceStats.ts";
export type { RichText } from "../wire/generated/RichText.ts";
export type { Span } from "../wire/generated/Span.ts";
export type { StarshipPieceRef } from "../wire/generated/StarshipPieceRef.ts";
export type { StateBadge } from "../wire/generated/StateBadge.ts";
export type { StateBadgeKind } from "../wire/generated/StateBadgeKind.ts";
export type { TuneChange } from "../wire/generated/TuneChange.ts";
export type { RolledPower } from "../wire/generated/RolledPower.ts";

/** What a face is not: the answer a card call gives is always a face. */
function faceOf(answer: ReturnType<typeof face>): FaceModel {
  if (answer.answer !== "face") throw new Error(`the face model answered ${answer.answer} for a card`);
  return answer.face;
}

/**
 * The face of one card: as printed when `source` has no `inPlay` (the collection), as the view says it
 * stands when it has. `def` is the card's definition; without it the module's catalog, then the match's,
 * then nothing (the card is drawn by its id).
 */
export function faceModel(source: FaceSource): FaceModel {
  return faceOf(face({ call: "card", source, locale: getLocale() }));
}

/** Words from the engine or a card with their glossary terms in bold, as spans (a prompt's detail, a quest's text). */
export function richWords(text: string): RichText {
  const answer = face({ call: "text", text, locale: getLocale() });
  if (answer.answer !== "rich") throw new Error(`the face model answered ${answer.answer} for a text`);
  return answer.text;
}

/** R277, R388: what a newer text adds to an older and what it loses, by word, in UTF-16 offsets. */
export function wordDiff(before: string, after: string): { added: Utf16Range[]; removed: Utf16Range[] } {
  const answer = face({ call: "wordDiff", before, after });
  if (answer.answer !== "diff") throw new Error(`the face model answered ${answer.answer} for a diff`);
  return { added: answer.added, removed: answer.removed };
}
