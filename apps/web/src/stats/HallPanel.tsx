// /stats' Experimental tab's Hall of Fame view (SPEC §9.11, R4681000, R4681001): the most busted cards
// JackiOh ever shipped, snapshotted as they were, with the version live today beside them. Four sections in
// task order, Most broken, Most nerfed, Most buffed and Most changed, each entry one row: its rank, the before
// face and the after face small side by side (`PatchFace`, each from its own catalog as the drill-down draws
// them) with the arrow between, the versions and their dates, Adjusted and tier before and after, and the
// patch's player-facing balance line. Pressing an entry opens X2's drill-down on the before version.
// Every figure is the server's (CLAUDE.md rule 7).

import { useEffect, useState, type ReactElement, type ReactNode } from "react";

import { canWatchArt, whenNear } from "../cards/art/near.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { PublicHallEntry, PublicHallOfFameResponse, PublicPlayVersion } from "../net/api.ts";
import { RefsInteractive } from "../cards/refContext.tsx";
import { ADJUSTED_RANGE_DIGITS } from "./config.ts";
import { RangeText, TierFitText } from "./RangeText.tsx";
import { statsTestid } from "./testids.ts";
import { TierChip, adjustedText } from "./TierChip.tsx";
import { cardTierFit } from "./tierFit.ts";

/** R4681001: the four sections, in the order the issue lists them. */
type HallSectionKey = "broken" | "nerfed" | "buffed" | "changed";
const SECTIONS: readonly HallSectionKey[] = ["broken", "nerfed", "buffed", "changed"];

/** R4681001: each section's heading, a literal `t()` call so R711000's scan covers it. */
function sectionTitle(key: HallSectionKey): string {
  switch (key) {
    case "broken":
      return t("Most broken");
    case "nerfed":
      return t("Most nerfed");
    case "buffed":
      return t("Most buffed");
    case "changed":
      return t("Most changed");
  }
}

/** R4681001: one side's Adjusted and tier, or "not enough games" below R654's floor. */
function SideFigures({
  version,
  band,
  showRanges,
}: {
  version: PublicPlayVersion;
  band: Pick<PublicHallOfFameResponse, "bandLow" | "bandHigh">;
  showRanges: boolean;
}): ReactElement {
  if (!version.hasEnoughGames) {
    return <span className="stats-not-enough">{t("not enough games")}</span>;
  }
  return (
    <span className="hall-figures-side">
      <TierChip tier={version.tier} testid={statsTestid.tierCell} />{" "}
      <span data-testid={statsTestid.adjustedCell}>
        {adjustedText(version.adjustedWinRate)}
        {showRanges && version.adjustedWinRate !== null && (
          <RangeText low={version.adjustedLow} high={version.adjustedHigh} digits={ADJUSTED_RANGE_DIGITS} testid={statsTestid.adjustedRange} />
        )}
      </span>
      {showRanges && <TierFitText fit={cardTierFit(version, band)} />}
    </span>
  );
}

/** R4681001: the versions and their dates, "v0.5.8 (2026-09-18) → v0.6.10 (2026-11-25)", one version alone where before equals after. */
function VersionsLine({ entry }: { entry: PublicHallEntry }): ReactElement {
  const dated = (version: PublicPlayVersion, date: string | null): string =>
    date ? `${version.first} (${date})` : version.first;
  return (
    <span className="hall-versions">
      {entry.before.id === entry.after.id
        ? dated(entry.before, entry.beforeDate)
        : `${dated(entry.before, entry.beforeDate)} → ${dated(entry.after, entry.afterDate)}`}
    </span>
  );
}

export function HallPanel({
  hall,
  showRanges,
  band,
  faceOf,
  needOf,
  onNeed,
  onOpen,
}: {
  hall: PublicHallOfFameResponse;
  showRanges: boolean;
  /** R3184001: the band a tier fit reads: the play versions' own (R4675001). */
  band: Pick<PublicHallOfFameResponse, "bandLow" | "bandHigh">;
  /** An entry side's face from its own catalog, or what stands in for it until that catalog loads. */
  faceOf: (version: PublicPlayVersion) => ReactNode;
  /** R4680000: the shipped version whose catalog a face still needs, none for a live one or a loaded one. */
  needOf: (version: PublicPlayVersion) => string | undefined;
  /** A face near the screen whose version's catalog has not loaded asks for it. */
  onNeed: (version: string) => void;
  /** Pressing an entry opens X2's drill-down on its before version. */
  onOpen: (id: string, opener: HTMLElement) => void;
}): ReactElement {
  // R4680000: each face whose version's catalog has not loaded asks for it once it nears the screen (at
  // once where nothing can watch the screen), so only the entries on screen load theirs.
  const [root, setRoot] = useState<HTMLElement | null>(null);
  const order = SECTIONS.map((key) => hall[key].map((entry) => entry.before.id).join(",")).join("\n");
  useEffect(() => {
    if (root === null) return;
    const stops = [...root.querySelectorAll<HTMLElement>("[data-needs]")].map((element) => {
      const version = element.dataset.needs ?? "";
      if (canWatchArt()) return whenNear(element, () => onNeed(version));
      onNeed(version);
      return () => undefined;
    });
    return () => {
      for (const stop of stops) stop();
    };
  }, [root, order, onNeed]);

  return (
    <div ref={setRoot}>
      {SECTIONS.map((key) => {
        const entries = hall[key];
        if (entries.length === 0) return null;
        return (
          <section key={key} data-testid={statsTestid.hallSection} aria-label={sectionTitle(key)}>
            <h3 className="hall-title">{sectionTitle(key)}</h3>
            <ol className="hall-list">
              {entries.map((entry, index) => {
                const single = entry.before.id === entry.after.id;
                const beforeNeeds = needOf(entry.before);
                const afterNeeds = single ? undefined : needOf(entry.after);
                return (
                  <li key={entry.before.id}>
                    <button
                      type="button"
                      className="hall-entry"
                      data-testid={statsTestid.hallEntry}
                      onClick={(e) => onOpen(entry.before.id, e.currentTarget)}
                      aria-label={`#${index + 1} ${entry.before.name}, ${entry.before.first} → ${entry.after.first}`}
                    >
                      <span className="hall-rank" aria-hidden="true">
                        {formatNumber(index + 1)}
                      </span>
                      <span className="hall-faces">
                        <span
                          className="hall-face"
                          data-testid={statsTestid.hallBefore}
                          data-needs={beforeNeeds}
                        >
                          <RefsInteractive enabled={false}>{faceOf(entry.before)}</RefsInteractive>
                        </span>
                        {!single && (
                          <>
                            <span className="hall-arrow" aria-hidden="true">
                              →
                            </span>
                            <span
                              className="hall-face"
                              data-testid={statsTestid.hallAfter}
                              data-needs={afterNeeds}
                            >
                              <RefsInteractive enabled={false}>{faceOf(entry.after)}</RefsInteractive>
                            </span>
                          </>
                        )}
                      </span>
                      <span className="hall-details">
                        <VersionsLine entry={entry} />
                        <span className="hall-figures">
                          <SideFigures version={entry.before} band={band} showRanges={showRanges} />
                          {!single && (
                            <>
                              <span className="hall-arrow" aria-hidden="true">
                                →
                              </span>
                              <SideFigures version={entry.after} band={band} showRanges={showRanges} />
                            </>
                          )}
                        </span>
                        {entry.changesBetween !== null && (
                          <span className="hall-changes">
                            {entry.changesBetween === 1
                              ? t("1 change")
                              : t("{n} changes", { n: formatNumber(entry.changesBetween) })}
                          </span>
                        )}
                        {entry.balance !== null && (
                          <span className="hall-balance" data-testid={statsTestid.hallBalance}>
                            {entry.balance}
                          </span>
                        )}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ol>
          </section>
        );
      })}
    </div>
  );
}
