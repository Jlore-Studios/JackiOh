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

import type { TextRange } from "../cards/radiantDiff.ts";
import type { CardDelta, FieldChange, TextChange } from "./diff.ts";
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
        <span className="patch-text__now patch-text__none">No text</span>
      ) : (
        <span className="patch-text__now">
          <MarkedText text={change.after} ranges={change.added} kind="added" />
        </span>
      )}
      {change.removed.length === 0 ? null : (
        <span className="patch-text__was">
          <span className="patch-text__tag">Was</span>{" "}
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
      <span className="patch-sr"> becomes </span>
      <span className="patch-values__now">{after}</span>
    </span>
  );
}

/** One change as a line of plain words, for the compact list: "Lines of code: not recorded → 15". */
export function changeSummary(change: FieldChange): string {
  if (change.kind === "text") return `${change.label} changed`;
  return `${change.label}: ${change.before} → ${change.after}`;
}

function ChangeLine({ change }: { change: FieldChange }): ReactElement {
  return (
    <li className="patch-change" data-testid={patchTestid.change} data-field={change.field} data-kind={change.kind}>
      <span className="patch-change__label">{change.label}</span>
      {change.kind === "text" ? <TextChangeView change={change} /> : <ValueFlow before={change.before} after={change.after} />}
    </li>
  );
}

/** Every change of one card in one patch; a card the patch added or removed says so. */
export function ChangeList({ delta }: { delta: CardDelta }): ReactElement {
  if (delta.kind === "added") return <p className="patch-change-note">Added in this patch.</p>;
  if (delta.kind === "removed") return <p className="patch-change-note">Removed in this patch.</p>;
  return (
    <ul className="patch-changes">
      {delta.changes.map((change) => (
        <ChangeLine key={change.field} change={change} />
      ))}
    </ul>
  );
}
