// #2646: a choice card, the face a short choice's option wears in the card picker (Prompt.tsx
// `ChoiceOption`), the way Hearthstone draws a Choose One. Prompt chrome, not a card: no cost gem,
// stats, rarity, foil or card id, and it never reaches a zone, the log or the collection; the option
// it draws is the engine's, answered by its key. Its picture is `CardArt` seeded by `choiceSeed`, its
// name the option's label, its text the option's detail. R831004: an option that names a set wears
// the set's mark as its picture. R3291000: an option that names a card's art (an Activate ability's,
// the card that prints it) wears that card's picture.

import type { ReactElement } from "react";

import { CardArt, RulesText, TermTips } from "../cards/index.ts";
import { setMarkOf } from "../cards/setMark.ts";
import { useCardInfo } from "./catalog.ts";

/** R3291000: the card whose picture a choice card wears, on the face it is on. */
export type ChoiceArt = { defId: string; radiant: boolean };

/**
 * The art seed of option `key`: the asking card's id and the option, or the prompt kind's when no card
 * asks. The `mode:` prefix is dropped, so a play's and an engine prompt's option draw the same picture.
 */
export function choiceSeed(source: string | undefined, kind: string, key: string): string {
  return `${source ?? kind}:${key.replace(/^mode:/, "")}`;
}

/** R3291000: a card's own picture, read as its face reads it: its name, tags and type from the catalog. */
function CardsArt(props: { art: ChoiceArt }): ReactElement {
  const info = useCardInfo(props.art.defId, props.art.radiant);
  return (
    <CardArt
      defId={props.art.defId}
      radiant={props.art.radiant}
      tags={info.tags}
      type={info.type}
      shape="window"
      name={info.name}
    />
  );
}

export function ChoiceCard(props: { seed: string; label: string; detail?: string; set?: string; art?: ChoiceArt }): ReactElement {
  const mark = props.set === undefined ? null : setMarkOf(props.set);
  return (
    <span className="cf-option choice-card" data-art-of={props.art?.defId}>
      <span className="choice-card-art">
        {mark !== null && mark.kind !== "unknown" ? (
          <img className="prompt-set-mark" src={mark.src} alt="" aria-hidden="true" title={mark.label} />
        ) : props.art !== undefined ? (
          <CardsArt art={props.art} />
        ) : (
          <CardArt defId={props.seed} radiant={false} tags={[]} type="Spell" shape="window" name={props.label} />
        )}
      </span>
      <span className="choice-card-name">{props.label}</span>
      {props.detail === undefined || props.detail === "" ? null : (
        <span className="choice-card-text">
          <TermTips>
            <RulesText words={props.detail} />
          </TermTips>
        </span>
      )}
    </span>
  );
}
