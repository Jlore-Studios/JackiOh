// R2249000: a Starship's rules box in play: no text (model.ts's `textInPlay`), only the Starship term
// and the Starship Pieces fused into it, oldest first, each a card reference (CardRef.tsx). The list
// is the view's, public on both seats, and nothing is decided (CLAUDE.md rule 7). R3333002: each name
// has its module's icon beside it (`starshipModuleIconUri`), the one its hull plate and art draw.
// R5050000: a unit holding gear lists it the same way below its own text (`EquippedGear`), the list the face
// model's (`FaceModel.equipped`).

import type { ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { starshipModuleOf } from "./art/starshipModules.ts";
import { starshipModuleIconUri } from "./art/svg.ts";
import { CardRef } from "./CardRef.tsx";
import type { EquippedFace } from "../wire/generated/EquippedFace.ts";
import type { GlossaryEntry } from "../wire/generated/GlossaryEntry.ts";
import type { StarshipFace } from "../wire/generated/StarshipFace.ts";
import type { StarshipPieceRef } from "../wire/generated/StarshipPieceRef.ts";
import { useDefResolver } from "./refContext.tsx";
import { TermTip } from "./TermTip.tsx";

export const STARSHIP_PIECES_TESTID = "starship-pieces";
export const EQUIPPED_GEAR_TESTID = "equipped-gear";

/** A list's head: its word, a term tip where the glossary has the term. */
function Head({ word, term }: { word: string; term: GlossaryEntry | null }): ReactElement {
  return <span className="cf-starship-head">{term === null ? word : <TermTip entry={term}>{word}</TermTip>}</span>;
}

/** The pieces, each its module's icon and a card reference on the face it reads on. */
function Pieces({ list }: { list: readonly StarshipPieceRef[] }): ReactElement {
  const resolve = useDefResolver();
  return (
    <span className="cf-starship-list" role="list">
      {list.map((piece, index) => {
        const def = resolve?.(piece.defId);
        const name = def?.name ?? piece.defId;
        return (
          <span key={`${piece.defId}-${String(index)}`} className="cf-starship-piece" role="listitem" data-piece={piece.defId}>
            <i
              className="cf-starship-icon"
              aria-hidden="true"
              data-module={starshipModuleOf(piece.defId).kind}
              style={{ backgroundImage: `url("${starshipModuleIconUri(piece.defId, piece.radiant)}")` }}
            />
            {def === undefined ? (
              name
            ) : (
              <CardRef def={def} radiant={piece.radiant}>
                {name}
              </CardRef>
            )}
          </span>
        );
      })}
    </span>
  );
}

export function StarshipPieces({ starship }: { starship: StarshipFace }): ReactElement {
  const list = starship.list;
  useLocale();
  return (
    <span className="cf-starship" data-testid={STARSHIP_PIECES_TESTID} data-pieces={list.length}>
      <Head word={t("Starship")} term={starship.term} />
      {list.length === 0 ? <span className="cf-starship-empty">{t("No pieces yet")}</span> : <Pieces list={list} />}
    </span>
  );
}

/** R5050000: the Miners and Weapons a unit holds, oldest first, under the Equip term. */
export function EquippedGear({ equipped }: { equipped: EquippedFace }): ReactElement {
  useLocale();
  return (
    <span className="cf-starship cf-equipped" data-testid={EQUIPPED_GEAR_TESTID} data-pieces={equipped.list.length} data-host={equipped.host}>
      <Head word={t("Equipped")} term={equipped.term} />
      <Pieces list={equipped.list} />
    </span>
  );
}
