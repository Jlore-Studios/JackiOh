// What one patch did to one card, as both screens print it (R388, R507).
//
// A value that changed reads "Cost: (2) Cost → (1) Cost", "Stats: 2/3 → 3/3" (R432's cost style).
// A face's text that changed prints whole, as the patch left it, with the words and numbers it did
// not have before marked the way R277 marks a Radiant face — bold and underlined, so the mark does
// not rest on colour alone — but in the patch mark's teal and with a double underline, so it is never
// read as the Radiant gold or as a reference's dotted line (R279). Under it, when the patch also took
// words away, the text as it was, with those words struck through ("Was"). Assistive technology
// hears "new:" and "removed:" before each mark (patches.css), since `<ins>` and `<del>` alone are not
// announced by every screen reader.

import type { ReactElement, ReactNode } from "react";

import { CHINESE_COMMA, readerKeyword, readerTerms } from "../cards/chinese.ts";
import type { TextRange } from "../cards/radiantDiff.ts";
import { isZhCn, t } from "../i18n/i18n.ts";
import { NONE, NOT_RECORDED, type CardDelta, type FieldChange, type TextChange } from "./diff.ts";
import { patchTestid } from "./testids.ts";

/** A text with some stretches wrapped: `<ins>` for what a patch added, `<del>` for what it took away. */
export function MarkedText({
  text,
  ranges,
  kind,
}: {
  text: string;
  ranges: readonly TextRange[];
  kind: "added" | "removed";
}): ReactElement {
  const out: ReactNode[] = [];
  let at = 0;
  ranges.forEach((range, n) => {
    if (range.start > at) out.push(text.slice(at, range.start));
    const words = text.slice(range.start, range.end);
    out.push(
      kind === "added" ? (
        <ins key={n} className="patch-mark" data-mark="patch">
          {words}
        </ins>
      ) : (
        <del key={n} className="patch-cut" data-mark="patch-removed">
          {words}
        </del>
      ),
    );
    at = range.end;
  });
  if (at < text.length) out.push(text.slice(at));
  return <>{out}</>;
}

/** A face's text as the patch left it, marked, and as it was where the patch removed words. */
export function TextChangeView({ change }: { change: TextChange }): ReactElement {
  return (
    <span className="patch-text">
      {change.after === "" ? (
        <span className="patch-text__now patch-text__none">{t("No text")}</span>
      ) : (
        <span className="patch-text__now">
          <MarkedText text={change.after} ranges={change.added} kind="added" />
        </span>
      )}
      {change.removed.length === 0 ? null : (
        <span className="patch-text__was">
          <span className="patch-text__tag">{t("Was")}</span>{" "}
          <MarkedText text={change.before} ranges={change.removed} kind="removed" />
        </span>
      )}
    </span>
  );
}

/** "(2) Cost → (1) Cost", with the arrow spoken as "becomes". */
export function ValueFlow({ before, after }: { before: string; after: string }): ReactElement {
  return (
    <span className="patch-values">
      <span className="patch-values__was">{before}</span>
      <span className="patch-values__arrow" aria-hidden="true">
        {" → "}
      </span>
      <span className="patch-sr"> {t("becomes")} </span>
      <span className="patch-values__now">{after}</span>
    </span>
  );
}

/** The plain words a value change can hold (diff.ts), which the reader's language has entries for. */
const VALUE_WORDS: ReadonlySet<string> = new Set([NONE, NOT_RECORDED, "yes", "no", "printed", "the base face, doubled"]);

function pick(table: Readonly<Record<string, string>> | undefined, word: string): string {
  return table !== undefined && Object.hasOwn(table, word) ? (table[word] ?? word) : word;
}

function listOf(text: string, word: (item: string) => string): string {
  if (text === NONE) return t(text);
  return text
    .split(", ")
    .map(word)
    .join(isZhCn() ? CHINESE_COMMA : ", ");
}

/** One side of a changed value as the reader reads it: the frame's words in their language, the data as it is. */
function valueOf(field: string, text: string): string {
  const terms = readerTerms();
  switch (field) {
    case "name":
      return text;
    case "cost":
    case "set":
      return t(text);
    case "type":
    case "base.type":
    case "radiant.type":
      return pick(terms?.types, text);
    case "rarity":
    case "printedRarity":
      return text === NONE ? t(text) : pick(terms?.rarities, text);
    case "tags":
      return listOf(text, (item) => pick(terms?.tags, item));
    case "base.keywords":
    case "radiant.keywords":
      return listOf(text, readerKeyword);
    default:
      return VALUE_WORDS.has(text) ? t(text) : text;
  }
}

/** One change as a line of plain words, for the compact list: "Lines of code: not recorded → 15". */
export function changeSummary(change: FieldChange): string {
  if (change.kind === "text") return t("{label} changed", { label: t(change.label) });
  return `${t(change.label)}${isZhCn() ? "：" : ": "}${valueOf(change.field, change.before)} → ${valueOf(change.field, change.after)}`;
}

function ChangeLine({ change }: { change: FieldChange }): ReactElement {
  return (
    <li className="patch-change" data-testid={patchTestid.change} data-field={change.field} data-kind={change.kind}>
      <span className="patch-change__label">{t(change.label)}</span>
      {change.kind === "text" ? (
        <TextChangeView change={change} />
      ) : (
        <ValueFlow before={valueOf(change.field, change.before)} after={valueOf(change.field, change.after)} />
      )}
    </li>
  );
}

/** Every change of one card in one patch; a card the patch added or removed says so. */
export function ChangeList({ delta }: { delta: CardDelta }): ReactElement {
  if (delta.kind === "added") return <p className="patch-change-note">{t("Added in this patch.")}</p>;
  if (delta.kind === "removed") return <p className="patch-change-note">{t("Removed in this patch.")}</p>;
  return (
    <ul className="patch-changes">
      {delta.changes.map((change) => (
        <ChangeLine key={change.field} change={change} />
      ))}
    </ul>
  );
}
