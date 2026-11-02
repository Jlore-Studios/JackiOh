// R2249000: a Starship's rules box in play: no text (model.ts's `textInPlay`), only the Starship term
// and the Starship Pieces fused into it, oldest first, each a card reference (CardRef.tsx). The list
// is the view's, public on both seats, and nothing is decided (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { CardRef } from "./CardRef.tsx";
import type { StarshipPieceRef } from "./model.ts";
import { useDefResolver } from "./refContext.tsx";
import { TermTip } from "./TermTip.tsx";

export const STARSHIP_PIECES_TESTID = "starship-pieces";

export function StarshipPieces({ list }: { list: readonly StarshipPieceRef[] }): ReactElement {
  useLocale();
  const resolve = useDefResolver();
  return (
    <span className="cf-starship" data-testid={STARSHIP_PIECES_TESTID} data-pieces={list.length}>
      <span className="cf-starship-head">
        <TermTip term="Starship">{t("Starship")}</TermTip>
      </span>
      {list.length === 0 ? (
        <span className="cf-starship-empty">{t("No pieces yet")}</span>
      ) : (
        <span className="cf-starship-list" role="list">
          {list.map((piece, index) => {
            const def = resolve?.(piece.defId);
            const name = def?.name ?? piece.defId;
            return (
              <span key={`${piece.defId}-${String(index)}`} className="cf-starship-piece" role="listitem" data-piece={piece.defId}>
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
      )}
    </span>
  );
}
