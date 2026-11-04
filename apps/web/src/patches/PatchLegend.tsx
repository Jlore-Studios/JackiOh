// R3012000: the legend on the side of the Patch notes page: a navigation bar of the release lines and,
// under each, its majors and patches, each patch's micro patches listed under it. A link jumps to its
// entry and opens it (PatchNotes.tsx does the opening); the entry in view is marked `aria-current`.
// On a phone the legend collapses into a drawer (`usePatchLegendCollapsed`). It reads only what the
// page hands it and decides nothing (CLAUDE.md rule 7).

import { useId, useState, type MouseEvent, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import type { Tier } from "./releases.ts";
import { patchTestid } from "./testids.ts";
import { usePatchLegendCollapsed } from "./usePatchLegendCollapsed.ts";

/** A major or a patch in the legend, with the versions of the micro patches it holds, oldest first. */
export type LegendEntry = { readonly version: string; readonly tier: Tier; readonly micros: readonly string[] };

/** A release line in the legend: `anchor` is its heading's id, `name` its name as `RELEASES` writes it. */
export type LegendLine = { readonly id: string; readonly anchor: string; readonly name: string; readonly entries: readonly LegendEntry[] };

type Props = {
  readonly lines: readonly LegendLine[];
  /** The version of the entry in view, or null. */
  readonly current: string | null;
  readonly onJump: (anchor: string) => void;
};

/** The address a link carries, so a copied or middle-clicked link opens the same entry (PatchNotes.tsx reads it back). */
export const legendHref = (anchor: string): string => `#${encodeURIComponent(anchor)}`;

function microCount(count: number): string {
  return count === 1 ? t("1 micro patch") : t("{n} micro patches", { n: count });
}

export function PatchLegend({ lines, current, onJump }: Props): ReactElement {
  const collapsed = usePatchLegendCollapsed();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());
  const bodyId = useId();

  const jump = (event: MouseEvent<HTMLAnchorElement>, anchor: string): void => {
    // A new tab, a new window or a download keeps the link's own address.
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    onJump(anchor);
    setDrawerOpen(false);
  };
  const toggleMicros = (version: string): void => {
    setExpanded((was) => {
      const next = new Set(was);
      if (!next.delete(version)) next.add(version);
      return next;
    });
  };
  const link = (version: string, tier: Tier): ReactElement => (
    <a
      className="link-button patch-legend__link"
      href={legendHref(version)}
      data-testid={patchTestid.legendLink}
      data-version={version}
      data-anchor={version}
      data-tier={tier}
      aria-current={current === version ? "location" : undefined}
      onClick={(event) => {
        jump(event, version);
      }}
    >
      {version}
    </a>
  );

  return (
    <nav className="patch-legend" aria-label={t("Patch notes legend")} data-testid={patchTestid.legend} data-collapsed={collapsed ? "true" : "false"}>
      {collapsed ? (
        <button
          type="button"
          className="link-button patch-legend__toggle"
          aria-expanded={drawerOpen}
          aria-controls={bodyId}
          data-testid={patchTestid.legendToggle}
          onClick={() => {
            setDrawerOpen((was) => !was);
          }}
        >
          {t("Jump to a patch")}
        </button>
      ) : null}
      <div id={bodyId} className="patch-legend__body" hidden={collapsed && !drawerOpen}>
        <ol className="patch-legend__lines">
          {lines.map((line) => (
            <li key={line.id} className="patch-legend__line">
              <a
                className="link-button patch-legend__link patch-legend__link--line"
                href={legendHref(line.anchor)}
                data-testid={patchTestid.legendLine}
                data-set={line.id}
                data-anchor={line.anchor}
                onClick={(event) => {
                  jump(event, line.anchor);
                }}
              >
                {t(line.name)}
              </a>
              <ol className="patch-legend__entries">
                {line.entries.map((entry) => {
                  const microsId = `${bodyId}-micros-${entry.version}`;
                  const shown = expanded.has(entry.version) || (current !== null && entry.micros.includes(current));
                  return (
                    <li key={entry.version} className="patch-legend__entry" data-tier={entry.tier}>
                      {link(entry.version, entry.tier)}
                      {entry.micros.length === 0 ? null : (
                        <>
                          <button
                            type="button"
                            className="link-button patch-legend__micros-toggle"
                            aria-expanded={shown}
                            aria-controls={microsId}
                            data-testid={patchTestid.legendMicros}
                            data-version={entry.version}
                            onClick={() => {
                              toggleMicros(entry.version);
                            }}
                          >
                            {microCount(entry.micros.length)}
                          </button>
                          <ol id={microsId} className="patch-legend__micros" hidden={!shown}>
                            {entry.micros.map((micro) => (
                              <li key={micro} className="patch-legend__micro">
                                {link(micro, "micro")}
                              </li>
                            ))}
                          </ol>
                        </>
                      )}
                    </li>
                  );
                })}
              </ol>
            </li>
          ))}
        </ol>
      </div>
    </nav>
  );
}
