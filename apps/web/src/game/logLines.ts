// The game log's lines, written in Rust: `jackioh-shared`'s `log` module (#3108), which the web reaches
// through the WebAssembly module's `log` body (`wasm/index.ts`). A line is final: its words are in the
// reader's language already, as the spans a card face's text is made of, and it says which card it is about
// when it names one the viewer may read (R97). Nothing here is a rule or a wording (CLAUDE.md rule 7).
//
// The language is the reader's (R711000): lines are made where they are drawn, and a history keeps the ones
// it was shown in the words it was shown them in (useLogHistory.ts).

import type { PlayerView } from "@jackioh/shared";

import { getLocale, type Locale } from "../i18n/i18n.ts";
import { log } from "../wasm/index.ts";
import type { LogLine } from "../wire/generated/LogLine.ts";

export type { LogLine } from "../wire/generated/LogLine.ts";

/**
 * The line of every event in the view's window, in order; null where the log leaves an event out (R2783000's
 * markers, `cardResolved`, `manaSpent`: the animation layer's bookkeeping).
 */
export function logLines(view: PlayerView, locale: Locale = getLocale()): readonly (LogLine | null)[] {
  const answer = log({ call: "view", view, locale });
  if (answer.answer !== "lines") throw new Error(`the log answered ${answer.answer} for a view`);
  return answer.lines;
}

/** R745: the line that says events may be missing where a window shares none with the one before. */
export function gapLine(): LogLine {
  const answer = log({ call: "gap", locale: getLocale() });
  if (answer.answer !== "line") throw new Error(`the log answered ${answer.answer} for a gap`);
  return answer.line;
}
