// #2646: a choice card, the face a short choice's option wears in the card picker (Prompt.tsx
// `ChoiceOption`), the way Hearthstone draws a Choose One. Prompt chrome, not a card: no cost gem,
// stats, rarity, foil or card id, and it never reaches a zone, the log or the collection; the option
// it draws is the engine's, answered by its key. Its picture is `CardArt` seeded by `choiceSeed`, its
// name the option's label, its text the option's detail. R831004: an option that names a set wears
// the set's mark as its picture.

import type { ReactElement } from "react";

import { CardArt, RulesText, TermTips } from "../cards/index.ts";
import { setMarkOf } from "../cards/setMark.ts";

/**
 * The art seed of option `key`: the asking card's id and the option, or the prompt kind's when no card
 * asks. The `mode:` prefix is dropped, so a play's and an engine prompt's option draw the same picture.
 */
export function choiceSeed(source: string | undefined, kind: string, key: string): string {
  return `${source ?? kind}:${key.replace(/^mode:/, "")}`;
}

export function ChoiceCard(props: { seed: string; label: string; detail?: string; set?: string }): ReactElement {
  const mark = props.set === undefined ? null : setMarkOf(props.set);
  return (
    <span className="cf-option choice-card">
      <span className="choice-card-art">
        {mark !== null && mark.kind !== "unknown" ? (
          <img className="prompt-set-mark" src={mark.src} alt="" aria-hidden="true" title={mark.label} />
        ) : (
          <CardArt defId={props.seed} radiant={false} tags={[]} type="Spell" shape="window" name={props.label} />
        )}
      </span>
      <span className="choice-card-name">{props.label}</span>
      {props.detail === undefined || props.detail === "" ? null : (
        <span className="choice-card-text">
          <TermTips>
            <RulesText text={props.detail} />
          </TermTips>
        </span>
      )}
    </span>
  );
}
