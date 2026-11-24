// R4677001: /stats' patch search, the dialog the patch bar's Find a patch button opens. It lists the patches
// `GET /api/stats/cards` sends, which are the caller's alone (R4677000), newest first and grouped by line, and a
// text box filters them by version, micros included. Picking one is the page's (`onPick`): it decides nothing
// (CLAUDE.md rule 7). For anyone but a dev account it ends with one line saying older patches are theirs.

import { useRef, useState, type ReactElement } from "react";

import { Dialog } from "../game/Dialog.tsx";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import { isCollecting, patchLines, patchName, type PatchWindow } from "./patches.ts";
import { statsTestid } from "./testids.ts";

/** R4677000, R4677001: the line that closes the search, and the refused address's, for anyone but a dev account. */
export function olderPatchesLine(): string {
  return t("Older patches are for dev accounts.");
}

/** The magnifier the patch bar's Find a patch button wears. */
export function FindPatchIcon(): ReactElement {
  return (
    <svg className="stats-patch-search-icon" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5 14 14" />
    </svg>
  );
}

export function PatchSearch({
  patches,
  shown,
  olderPatchesForDevs,
  onPick,
  onClose,
}: {
  /** Every patch the caller may read, newest first. */
  patches: readonly PatchWindow[];
  /** The index in `patches` of the patch the page shows, -1 for none. */
  shown: number;
  /** R4677000: the caller is no dev account. */
  olderPatchesForDevs: boolean;
  onPick: (patch: PatchWindow) => void;
  onClose: () => void;
}): ReactElement {
  useLocale();
  const [query, setQuery] = useState("");
  const field = useRef<HTMLInputElement>(null);
  const lines = patchLines(patches, query);

  return (
    <Dialog
      role="dialog"
      testid={statsTestid.patchSearchDialog}
      className="stats-patch-search-dialog"
      title={t("Find a patch")}
      onClose={onClose}
      actions={[]}
      focus={field}
    >
      <input
        ref={field}
        type="search"
        className="stats-search-input stats-patch-search-input"
        data-testid={statsTestid.patchSearchInput}
        aria-label={t("Patch version")}
        placeholder={t("Version, as 0.5.8")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <div className="stats-patch-search-lines">
        {lines.map(({ line, patches: held }) => (
          <section key={line} className="stats-patch-search-line" data-testid={statsTestid.patchSearchLine} aria-label={line}>
            <h3 className="stats-patch-search-line-name">{line}</h3>
            <ul className="stats-patch-search-options">
              {held.map((patch) => {
                const index = patches.indexOf(patch);
                return (
                  <li key={patchName(patch)}>
                    <button
                      type="button"
                      className="stats-patch-search-option"
                      data-testid={statsTestid.patchSearchOption}
                      aria-current={index === shown ? "true" : undefined}
                      onClick={() => onPick(patch)}
                    >
                      <span className="stats-patch-tab-name">{patchName(patch)}</span>
                      <span className="stats-patch-tab-games">
                        {isCollecting(patches, index)
                          ? t("Collecting data")
                          : t("{games} games", { games: formatNumber(patch.games) })}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
        {lines.length === 0 && (
          <p className="stats-patch-search-none" data-testid={statsTestid.patchSearchNone}>
            {t("No patch holds version {version}.", { version: query.trim() })}
          </p>
        )}
      </div>
      {olderPatchesForDevs && (
        <p className="stats-patch-search-devs" data-testid={statsTestid.patchSearchDevs}>
          {olderPatchesLine()}
        </p>
      )}
    </Dialog>
  );
}
