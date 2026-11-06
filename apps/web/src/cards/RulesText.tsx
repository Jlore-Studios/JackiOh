// Rules text as a face prints it (docs/polish/6-cards.md, B10; SPEC §10.10): the one component that
// draws the spans the face model gives (jackioh-shared `face`, #3106), so every surface prints them the same
// way. It renders what the spans say and decides nothing:
// - glossary terms in bold (`.cf-term`), each showing its rule where the surface allows (TermTip.tsx, R2098000);
// - on a Radiant face, what its base face lacks (a `mark`, R277): gold, bold and underlined, so the mark does
//   not rest on color alone;
// - the names the card's `refs` link (R279) as references; a term inside a name is not bolded, since the name
//   is the card, not the keyword ("Rush Token");
// - in play, what its formula comes to now (a `value`, R280, R372), each after its own line's label;
// - in play, the numbers the view moved off their printed values (a `number`, R386), the glyph drawn by the
//   stylesheet so the text stays the number.
// It renders text nodes and spans (and portals), so it can sit inside a <button>, where references stay marks
// and never become controls.

import { Fragment, type ReactElement, type ReactNode } from "react";

import type { CardDef, PreviewValue } from "@jackioh/shared";

import { useLocale } from "../i18n/i18n.ts";
import type { Span } from "../wire/generated/Span.ts";
import { richWords, type RichText } from "./model.ts";
import { CardRef } from "./CardRef.tsx";
import { TermTip } from "./TermTip.tsx";
import { useEngineReady } from "../wasm/useEngineReady.ts";
import { useDefResolver, type DefResolver } from "./refContext.tsx";

type RulesTextProps =
  | {
      /** The face's text as the face model gives it: final words in the reader's language (jackioh-shared). */
      rich: RichText;
    }
  | {
      /** Words from the engine or a card, which the face model puts its terms in bold in (`richWords`). */
      words: string;
    };

/**
 * R280, R372: what a value prints as: the word the view names it by (#93's grade letter, "C"), else
 * its number. The client never turns one into the other.
 */
export function printedValue(entry: PreviewValue): string {
  return entry.display !== undefined && entry.display !== "" ? entry.display : String(entry.value);
}

/**
 * One span as markup. The face model decided every one of them (jackioh-shared `face/spans.rs`): which
 * words are bold terms, which a Radiant face adds, which a card names, which number moved and where the
 * formula's values stand. A reference only becomes a control where the surface can find the card it names.
 */
function draw(spans: readonly Span[], resolve: DefResolver | null, next: () => number): ReactNode[] {
  return spans.map((span): ReactNode => {
    switch (span.kind) {
      case "text":
        return <Fragment key={next()}>{span.text}</Fragment>;
      case "keyword":
        return (
          <TermTip key={next()} entry={span.entry}>
            {draw(span.children, resolve, next)}
          </TermTip>
        );
      case "number":
        return (
          <span key={next()} className="cf-tuned" data-way={span.way} data-printed={span.printed} data-key={span.key} title={span.title}>
            {span.text}
          </span>
        );
      case "card": {
        const def: CardDef | undefined = resolve?.(span.defId);
        const children = draw(span.children, resolve, next);
        return def === undefined ? (
          <Fragment key={next()}>{children}</Fragment>
        ) : (
          <CardRef key={next()} def={def} radiant={span.radiant}>
            {children}
          </CardRef>
        );
      }
      case "mark":
        return (
          <span key={next()} className="cf-mark" data-mark="radiant">
            {draw(span.children, resolve, next)}
          </span>
        );
      case "value":
        return (
          <span key={next()} className="cf-value" data-value={span.value} data-label={span.label} title={span.title}>
            {span.text}
          </span>
        );
    }
  });
}

/** The ids of the cards `spans` names, in order. */
function namedCards(spans: readonly Span[]): string[] {
  return spans.flatMap((span): string[] => {
    switch (span.kind) {
      case "card":
        return [span.defId, ...namedCards(span.children)];
      case "keyword":
      case "mark":
        return namedCards(span.children);
      default:
        return [];
    }
  });
}

export function RulesText(props: RulesTextProps): ReactElement {
  useLocale();
  const resolve = useDefResolver();
  let key = 0;
  const next = (): number => (key += 1);
  // Words that are not a face's are put in bold by the module; a screen that has not got it yet prints them plain.
  const ready = useEngineReady();
  const rich: RichText = "rich" in props ? props.rich : ready ? richWords(props.words) : { full: props.words, spans: [{ kind: "text", text: props.words }] };
  // A name the surface cannot find the card of is plain text, and a term inside it is bold again.
  const linked = rich.unlinked === undefined || (resolve !== null && namedCards(rich.spans).every((id) => resolve(id) !== undefined));
  return <>{draw(linked ? rich.spans : (rich.unlinked ?? rich.spans), resolve, next)}</>;
}
