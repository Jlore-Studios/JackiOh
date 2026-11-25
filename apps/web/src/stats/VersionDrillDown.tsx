// R4680001: the drill-down on one play version (R4675000), which a Versions row opens and `?card=<id>&version=<first>`
// names: its base and Radiant faces as its own catalog printed them (`PatchFace`, the patch notes' face), never
// today's; its figures (R4675001), read against the play versions' band; and the versions before and after it, each
// with its face and the change between the two (`ChangeList`), so the change reads at a glance. Old printings make no
// card reference interactive (R279). A version's definition is its catalog's at its last version (R4675001), loaded
// only when asked (R4680000); every figure is the server's (CLAUDE.md rule 7).

import { useEffect, type ReactElement } from "react";
import type { CardDef, CardDefs } from "@jackioh/shared";

import { RefsInteractive } from "../cards/refContext.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { PublicPlayVersion, PublicStatsVersionsResponse } from "../net/api.ts";
import { ChangeList } from "../patches/ChangeList.tsx";
import { diffCard } from "../patches/diff.ts";
import { PatchFace } from "../patches/PatchFace.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { ADJUSTED_RANGE_DIGITS } from "./config.ts";
import { RangeText, TierFitText } from "./RangeText.tsx";
import { patchesText, spanText } from "./RunningPanel.tsx";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText } from "./TierChip.tsx";
import { cardTierFit } from "./tierFit.ts";

import "../patches/patches.css";

/** R4680001: a version's definition and the catalog it is in, which names the cards it refers to; null where none loaded. */
export type VersionRead = { readonly def: CardDef; readonly names: CardDefs } | null;

/** A play version's span as its rows print it. */
function spanOf(version: PublicPlayVersion): string {
  return spanText(version.first, version.live ? null : version.last);
}

/** Its definition's loading, its failure (with a retry), or what `draw` makes of it. */
function Loaded({
  read,
  onRetry,
  draw,
}: {
  read: VersionRead | undefined;
  onRetry: () => void;
  draw: (shown: NonNullable<VersionRead>) => ReactElement;
}): ReactElement {
  if (read === undefined) return <Skeleton variant="rows" label={t("Loading this version's card…")} />;
  if (read === null) {
    return (
      <div className="stats-error" data-testid={statsTestid.versionFailed}>
        <p>{t("This version's card didn't load.")}</p>
        <button type="button" className="stats-retry-btn" onClick={onRetry}>
          {t("Retry")}
        </button>
      </div>
    );
  }
  return draw(read);
}

/** R4680001: the version before or after the open one: its span, which opens it, its face and the change between them. */
function Neighbour({
  side,
  neighbour,
  shown,
  read,
  need,
  onOpen,
}: {
  side: "before" | "after";
  neighbour: PublicPlayVersion | undefined;
  shown: VersionRead | undefined;
  read: (version: PublicPlayVersion) => VersionRead | undefined;
  need: (version: PublicPlayVersion) => void;
  onOpen: (version: PublicPlayVersion) => void;
}): ReactElement {
  const testid = side === "before" ? statsTestid.versionBefore : statsTestid.versionAfter;
  const heading = side === "before" ? t("The version before") : t("The version after");
  if (neighbour === undefined) {
    return (
      <section className="stats-drilldown-section" data-testid={testid}>
        <h3>{heading}</h3>
        <p className="stats-empty-sub">{side === "before" ? t("None: this is its first version.") : t("None: this is its live version.")}</p>
      </section>
    );
  }
  const theirs = read(neighbour);
  return (
    <section className="stats-drilldown-section stats-version-neighbour" data-testid={testid} data-version-id={neighbour.id}>
      <h3>{heading}</h3>
      <button
        type="button"
        className="stats-drilldown-action button-secondary"
        data-testid={statsTestid.versionOpen}
        onClick={() => onOpen(neighbour)}
        aria-label={t("Open {name}, {span}", { name: neighbour.name, span: spanOf(neighbour) })}
      >
        {spanOf(neighbour)}
      </button>
      <Loaded
        read={theirs}
        onRetry={() => need(neighbour)}
        draw={(them) => {
          const delta =
            shown === undefined || shown === null
              ? undefined
              : side === "before"
                ? diffCard(them.def, shown.def, { before: them.names, after: shown.names })
                : diffCard(shown.def, them.def, { before: shown.names, after: them.names });
          return (
            <div className="stats-version-change">
              <figure className="patch-history__face">
                <PatchFace def={them.def} face="base" />
              </figure>
              {delta === null ? (
                <p className="stats-sample-hint">{t("Its card did not change; how it plays did.")}</p>
              ) : delta === undefined ? null : (
                <div>
                  <p className="stats-sample-hint">
                    {side === "before" ? t("What changed from it to this version") : t("What changed from this version to it")}
                  </p>
                  <ChangeList delta={delta} />
                </div>
              )}
            </div>
          );
        }}
      />
    </section>
  );
}

export function VersionDrillDown({
  version,
  before,
  after,
  band,
  title,
  read,
  need,
  onOpen,
  onClose,
}: {
  version: PublicPlayVersion;
  /** The card's play versions either side of it, none at either end. */
  before: PublicPlayVersion | undefined;
  after: PublicPlayVersion | undefined;
  /** R3184001: the band its tier fit reads: the play versions' own (R4675001). */
  band: Pick<PublicStatsVersionsResponse, "bandLow" | "bandHigh">;
  title: string;
  /** A version's definition: undefined until its catalog loads (R4680000). */
  read: (version: PublicPlayVersion) => VersionRead | undefined;
  /** Asks for a version's catalog, again after a failure. */
  need: (version: PublicPlayVersion) => void;
  onOpen: (version: PublicPlayVersion) => void;
  onClose: () => void;
}): ReactElement {
  // R4680000: only the three catalogs this drill-down draws.
  useEffect(() => {
    for (const shown of [version, before, after]) if (shown !== undefined) need(shown);
  }, [version, before, after, need]);

  const shown = read(version);
  const adjusted = version.hasEnoughGames ? version.adjustedWinRate : null;
  return (
    <Dialog
      role="dialog"
      testid={statsTestid.drillDownModal}
      className="stats-drilldown-modal"
      data={{ "data-radiant": "true", "data-version-id": version.id }}
      closeTestid={statsTestid.drillDownClose}
      closeLabel={t("Close card details")}
      title={title}
      onClose={onClose}
      actions={[]}
    >
      <div className="stats-drilldown-body" data-testid={statsTestid.versionDrillDown}>
        <div className="stats-drilldown-cards">
          <Loaded
            read={shown}
            onRetry={() => need(version)}
            draw={({ def }) => (
              <RefsInteractive enabled={false}>
                <div className="stats-drilldown-faces" data-testid={statsTestid.versionFaces}>
                  <figure className="stats-drilldown-face">
                    <div className="stats-drilldown-card-preview">
                      <PatchFace def={def} face="base" />
                    </div>
                    <figcaption className="stats-drilldown-caption">{t("Normal")}</figcaption>
                  </figure>
                  <figure className="stats-drilldown-face">
                    <div className="stats-drilldown-card-preview">
                      <PatchFace def={def} face="radiant" />
                    </div>
                    <figcaption className="stats-drilldown-caption">{t("Radiant")}</figcaption>
                  </figure>
                </div>
              </RefsInteractive>
            )}
          />
        </div>
        <div className="stats-drilldown-details">
          <dl className="stats-grid-figures stats-version-figures" data-testid={statsTestid.versionFigures}>
            <dt>{t("Tier")}</dt>
            <dd>
              <TierChip tier={version.hasEnoughGames ? version.tier : null} testid={statsTestid.tierCell} />
              <TierFitText fit={cardTierFit(version, band)} />
            </dd>
            <dt>{t("Adjusted")}</dt>
            <dd data-testid={statsTestid.adjustedCell}>
              {adjustedText(adjusted)}
              {adjusted !== null && (
                <RangeText low={version.adjustedLow} high={version.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
              )}
            </dd>
            <dt>{t("Games")}</dt>
            <dd>{formatNumber(version.games)}</dd>
            <dt>{t("Since")}</dt>
            <dd data-testid={statsTestid.versionsSpan}>
              {spanOf(version)}
              {version.patches !== undefined && version.patches > 0 && <span className="stats-sample-hint"> {patchesText(version.patches)}</span>}
            </dd>
          </dl>
          <RefsInteractive enabled={false}>
            <Neighbour side="before" neighbour={before} shown={shown} read={read} need={need} onOpen={onOpen} />
            <Neighbour side="after" neighbour={after} shown={shown} read={read} need={need} onOpen={onOpen} />
          </RefsInteractive>
        </div>
      </div>
    </Dialog>
  );
}
