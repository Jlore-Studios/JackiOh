// The owned-cards panel (#1307, SPEC §14): a completion count per shipped set and, behind each
// set's toggle, its cards. A card the account does not own is a locked silhouette that still
// opens the inspect view, so a player sees what they are missing. The server's `GET
// /api/collection` says which are owned (CLAUDE.md rule 7); this counts the rows and draws them.

import { useId, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, SetName } from "@jackioh/shared";

import { CardDefsProvider, CardDetail, CardFace, closeInspect, faceModel } from "../cards/index.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { CollectionResponse } from "../net/api.ts";
import { COLLECTED_DEFS, collectionTestid, setCompletions } from "./collection.ts";

type Inspected = { def: CardDef; owned: boolean };

export default function OwnedCards({
  collection,
  defs = COLLECTED_DEFS,
}: {
  collection: CollectionResponse;
  defs?: readonly CardDef[];
}): ReactElement {
  const [openSet, setOpenSet] = useState<SetName | null>(null);
  const [inspected, setInspected] = useState<Inspected | null>(null);
  const idBase = useId();

  return (
    <CardDefsProvider defs={CATALOG}>
      {setCompletions(collection, defs).map(({ set, owned, total, cards }) => {
        const open = openSet === set;
        const gridId = `${idBase}-${set}`;
        return (
          <div key={set} className="collection-set" data-testid={collectionTestid.set(set)}>
            <button
              type="button"
              className="collection-set__toggle"
              data-testid={collectionTestid.setToggle(set)}
              aria-expanded={open}
              aria-controls={gridId}
              onClick={() => {
                setOpenSet(open ? null : set);
              }}
            >
              <span>{t(set)}</span>
              <span data-testid={collectionTestid.completion(set)}>
                {t("{progress} of {target}", { progress: formatNumber(owned), target: formatNumber(total) })}
              </span>
            </button>
            {open ? (
              <ul id={gridId} className="collection-grid">
                {cards.map(({ def, owned: held }) => (
                  <li key={def.id}>
                    <button
                      type="button"
                      className="collection-card"
                      data-testid={collectionTestid.card(def.id)}
                      data-owned={String(held)}
                      aria-label={held ? def.name : t("{name}, not owned yet", { name: def.name })}
                      onClick={() => {
                        closeInspect();
                        setInspected({ def, owned: held });
                      }}
                    >
                      <span className="collection-card__face">
                        <CardFace face={faceModel({ defId: def.id, def, radiant: false })} layout="full" lazyArt />
                      </span>
                      {held ? null : (
                        <span className="collection-card__lock" aria-hidden="true">
                          {t("Locked")}
                        </span>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        );
      })}
      {inspected === null ? null : (
        <CardDetail
          def={inspected.def}
          onClose={() => {
            setInspected(null);
          }}
          meta={<span>{inspected.owned ? t("Owned") : t("Not owned yet")}</span>}
        />
      )}
    </CardDefsProvider>
  );
}
