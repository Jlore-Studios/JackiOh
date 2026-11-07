// R2249000: a Starship's rules box in play: no text (model.ts's `textInPlay`), only the Starship term
// and the Starship Pieces fused into it, oldest first, each a card reference (CardRef.tsx). The list
// is the view's, public on both seats, and nothing is decided (CLAUDE.md rule 7). R3333002: each name
// has its module's icon beside it (`starshipModuleIconUri`), the one its hull plate and art draw.

import type { ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { starshipModuleOf } from "./art/starshipModules.ts";
import { starshipModuleIconUri } from "./art/svg.ts";
import { CardRef } from "./CardRef.tsx";
import type { StarshipFace } from "../wire/generated/StarshipFace.ts";
import { useDefResolver } from "./refContext.tsx";
import { TermTip } from "./TermTip.tsx";

export const STARSHIP_PIECES_TESTID = "starship-pieces";

export function StarshipPieces({ starship }: { starship: StarshipFace }): ReactElement {
  const list = starship.list;
  useLocale();
  const resolve = useDefResolver();
  return (
    <span className="cf-starship" data-testid={STARSHIP_PIECES_TESTID} data-pieces={list.length}>
      <span className="cf-starship-head">
        {starship.term === null ? t("Starship") : <TermTip entry={starship.term}>{t("Starship")}</TermTip>}
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
      )}
    </span>
  );
}
