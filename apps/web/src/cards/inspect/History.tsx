// The detail view's History section (SPEC §10.10, R375): when the card was created and each change
// since, newest first, from the patch list and the catalog snapshots (cards/patches.ts).
//
// Collapsed by default, and labelled with the card's version count, which `changes.json` gives
// without a snapshot. Opening it is what loads the snapshots (a dynamic import, so the main bundle
// never carries them), and the entries follow: each version's number, date and title, a
// "Reconstructed" badge on a version named only later, and what changed. A field reads
// "old → new"; a text reads as both versions whole, the words the older one drops struck through
// and the words the newer one adds underlined (radiantDiff.ts's `wordDiff`), each a shape as well as
// a colour. A Radiant text stored before v0.1.0-r3 is SPEC §8's shorthand and is labelled so, and
// the version that replaced one with the whole text says "written out in full" rather than
// diffing the two. The creation entry comes last: "Created in v0.1.0 · 2026-09-18". Every entry
// can draw the card as that version left it, both faces, from the snapshot's definition.
//
// Presentation only (CLAUDE.md rule 7): it reads public, printed card data and nothing else.

import { useEffect, useId, useState, type ReactElement } from "react";

import { cardHistory, type CardChange, type HistoryEntry, type HistoryField, type Snapshots } from "@jackioh/cards/history";
import type { CardDef } from "@jackioh/shared";

import { CardFace } from "../CardFace.tsx";
import { faceModel } from "../model.ts";
import { PATCHES, RECONSTRUCTED_NOTE, loadSnapshots, versionCount } from "../patches.ts";
import { wordDiff, type TextRange } from "../radiantDiff.ts";
import { CardDefsProvider, RefsInteractive } from "../refContext.tsx";
import {
  INSPECT_HISTORY,
  INSPECT_HISTORY_BADGE,
  INSPECT_HISTORY_ENTRY,
  INSPECT_HISTORY_FACES,
  INSPECT_HISTORY_SHOW,
  INSPECT_HISTORY_TOGGLE,
} from "./testids.ts";

/** How each named field is labelled; a field the list has no name for prints its key. */
const FIELD_LABELS: Readonly<Record<HistoryField, string>> = {
  name: "Name",
  cost: "Cost",
  type: "Type",
  rarity: "Rarity",
  tags: "Tags",
  stats: "Stats",
  keywords: "Keywords",
  radiantStats: "Radiant stats",
  radiantKeywords: "Radiant keywords",
  refs: "Cards named",
};

/** A Radiant text as SPEC §8 stored it before v0.1.0-r3. */
const SHORTHAND = "shorthand";

function fieldLabel(field: string): string {
  return (FIELD_LABELS as Readonly<Record<string, string>>)[field] ?? field;
}

export function versionsLabel(count: number): string {
  return `${String(count)} ${count === 1 ? "version" : "versions"}`;
}

/** A text with some stretches wrapped: struck through where removed, underlined where added. */
function DiffText({ text, marks, as }: { text: string; marks: readonly TextRange[]; as: "del" | "ins" }): ReactElement {
  if (text === "") return <em className="inspect-history-empty">none</em>;
  const Mark = as;
  const parts: (string | ReactElement)[] = [];
  let at = 0;
  for (const range of marks) {
    if (range.start > at) parts.push(text.slice(at, range.start));
    parts.push(
      <Mark key={range.start} className={`inspect-history-${as}`}>
        {text.slice(range.start, range.end)}
      </Mark>,
    );
    at = range.end;
  }
  if (at < text.length) parts.push(text.slice(at));
  return <>{parts}</>;
}

function ChangeLine({ change }: { change: CardChange }): ReactElement {
  if (change.kind === "field") {
    return (
      <li className="inspect-history-change" data-field={change.field}>
        <span className="inspect-history-field">{fieldLabel(change.field)}</span>{" "}
        <span className="inspect-history-before">{change.before}</span>
        <span className="inspect-history-arrow"> → </span>
        <span className="inspect-history-after">{change.after}</span>
      </li>
    );
  }
  const field = change.face === "base" ? "text" : "radiantText";
  const label = change.face === "base" ? "Text" : "Radiant text";
  // A shorthand replaced by the whole text is a new way of writing it, not a list of edits.
  const writtenOut = change.beforeShorthand && !change.afterShorthand;
  const diff = writtenOut ? { removed: [], added: [] } : wordDiff(change.before, change.after);
  // R277's comparison sets case, separators and line breaks aside, so a change of only those marks
  // no word; the line says so instead of showing two texts that seem the same.
  const layoutOnly = !writtenOut && diff.removed.length === 0 && diff.added.length === 0;
  return (
    <li
      className="inspect-history-change inspect-history-change--text"
      data-field={field}
      data-written-out={writtenOut ? "true" : undefined}
      data-layout-only={layoutOnly ? "true" : undefined}
    >
      <span className="inspect-history-field">
        {label}
        {writtenOut ? ", written out in full" : null}
        {layoutOnly ? ", capitals, punctuation or line breaks only" : null}
      </span>
      <span className="inspect-history-before">
        <DiffText text={change.before} marks={diff.removed} as="del" />
        {change.beforeShorthand ? <span className="inspect-history-shorthand"> ({SHORTHAND})</span> : null}
      </span>
      <span className="inspect-history-arrow" aria-hidden="true">
        ↓
      </span>
      <span className="inspect-history-after">
        <span className="inspect-history-sr">became </span>
        <DiffText text={change.after} marks={diff.added} as="ins" />
        {change.afterShorthand ? <span className="inspect-history-shorthand"> ({SHORTHAND})</span> : null}
      </span>
    </li>
  );
}

/** Both faces of the card as one version left it, drawn from that version's own catalog. */
function VersionFaces({ entry, defs }: { entry: HistoryEntry; defs: Snapshots[string] }): ReactElement {
  const def: CardDef = entry.def;
  return (
    <CardDefsProvider defs={defs}>
      <RefsInteractive enabled={false}>
        <div className="inspect-history-faces" data-testid={INSPECT_HISTORY_FACES} data-version={entry.patch.version}>
          <figure className="inspect-history-face">
            <div className="inspect-face inspect-face--history">
              <CardFace face={faceModel({ defId: def.id, def, radiant: false })} layout="full" />
            </div>
            <figcaption className="inspect-detail-caption">Base</figcaption>
          </figure>
          <figure className="inspect-history-face inspect-detail-face--radiant">
            <div className="inspect-face inspect-face--history">
              <CardFace face={faceModel({ defId: def.id, def, radiant: true })} layout="full" />
            </div>
            <figcaption className="inspect-detail-caption">{entry.radiantShorthand ? `Radiant (${SHORTHAND})` : "Radiant"}</figcaption>
          </figure>
        </div>
      </RefsInteractive>
    </CardDefsProvider>
  );
}

function Entry({ entry, snapshots }: { entry: HistoryEntry; snapshots: Snapshots }): ReactElement {
  const [shown, setShown] = useState(false);
  const { version, date, title, reconstructed } = entry.patch;
  // A removal draws the card as it stood before it went.
  const at = entry.kind === "removed" ? PATCHES[PATCHES.findIndex((patch) => patch.version === version) - 1]?.version : version;
  const defs = at === undefined ? undefined : snapshots[at];
  const head = entry.kind === "created" ? `Created in ${version}` : entry.kind === "removed" ? `Removed in ${version}` : version;
  return (
    <li className="inspect-history-entry" data-testid={INSPECT_HISTORY_ENTRY} data-version={version} data-kind={entry.kind}>
      <p className="inspect-history-head">
        <span className="inspect-history-version">{head}</span>
        {" · "}
        <time dateTime={date}>{date}</time>
        {reconstructed ? (
          <span className="inspect-history-badge" data-testid={INSPECT_HISTORY_BADGE} title={RECONSTRUCTED_NOTE}>
            Reconstructed
          </span>
        ) : null}
      </p>
      <p className="inspect-history-title">{title}</p>
      {entry.changes.length === 0 ? null : (
        <ul className="inspect-history-changes">
          {entry.changes.map((change, index) => (
            <ChangeLine key={index} change={change} />
          ))}
        </ul>
      )}
      {defs === undefined ? null : (
        <>
          <button
            type="button"
            className="link-button inspect-history-show"
            data-testid={INSPECT_HISTORY_SHOW}
            aria-expanded={shown}
            onClick={() => {
              setShown((open) => !open);
            }}
          >
            {shown ? `Hide the card in ${version}` : `Show the card in ${version}`}
          </button>
          {shown ? <VersionFaces entry={entry} defs={defs} /> : null}
        </>
      )}
    </li>
  );
}

type Loaded = { status: "loading" } | { status: "ready"; snapshots: Snapshots } | { status: "failed" };

/** The snapshots, loaded the first time `open` is true; `retry` loads again after a failure. */
function useSnapshots(open: boolean): { loaded: Loaded | null; retry: () => void } {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [attempt, setAttempt] = useState(0);
  const wanted = open || loaded !== null;
  useEffect(() => {
    if (!wanted) return;
    let live = true;
    setLoaded((current) => (current?.status === "ready" ? current : { status: "loading" }));
    loadSnapshots().then(
      (snapshots) => {
        if (live) setLoaded({ status: "ready", snapshots });
      },
      () => {
        if (live) setLoaded({ status: "failed" });
      },
    );
    return () => {
      live = false;
    };
    // `wanted` only ever turns true, so this runs on the first open and on each retry.
  }, [wanted, attempt]);
  return {
    loaded,
    retry: () => {
      setAttempt((n) => n + 1);
    },
  };
}

export function CardHistory({ def }: { def: CardDef }): ReactElement | null {
  const [open, setOpen] = useState(false);
  const { loaded, retry } = useSnapshots(open);
  const bodyId = useId();
  const count = versionCount(def.id);
  // A definition no version holds (a match-made one) has no history to show.
  if (count === 0) return null;

  let body: ReactElement | null = null;
  if (open && (loaded === null || loaded.status === "loading")) {
    body = (
      <p className="inspect-history-status" role="status">
        Loading the card&rsquo;s history…
      </p>
    );
  } else if (open && loaded?.status === "failed") {
    body = (
      <p className="inspect-history-status" role="alert">
        The history didn&rsquo;t load.{" "}
        <button type="button" className="link-button" onClick={retry}>
          Try again
        </button>
      </p>
    );
  } else if (open && loaded?.status === "ready") {
    const entries = cardHistory(def.id, PATCHES, loaded.snapshots).reverse();
    body = (
      <>
        <ol className="inspect-history-list">
          {entries.map((entry) => (
            <Entry key={`${entry.patch.version}:${entry.kind}`} entry={entry} snapshots={loaded.snapshots} />
          ))}
        </ol>
        {entries.some((entry) => entry.patch.reconstructed) ? <p className="inspect-history-note">{RECONSTRUCTED_NOTE}</p> : null}
      </>
    );
  }

  return (
    <section className="inspect-history" data-testid={INSPECT_HISTORY} data-open={open ? "true" : "false"}>
      <button
        type="button"
        className="inspect-history-toggle"
        data-testid={INSPECT_HISTORY_TOGGLE}
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={() => {
          setOpen((current) => !current);
        }}
      >
        <span className="inspect-history-label">History</span>{" "}
        <span className="inspect-history-count">({versionsLabel(count)})</span>
      </button>
      <div id={bodyId} className="inspect-history-body" hidden={!open}>
        {body}
      </div>
    </section>
  );
}
