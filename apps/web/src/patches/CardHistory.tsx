// Card history (brief B4.2 item 5; R388, R507) is public, read-only data (CLAUDE.md rule 7; §5.1).
// Old printings do not make card references interactive (R279).

import { useId, useState, type ReactElement } from "react";

import { RefsInteractive } from "../cards/refContext.tsx";
import { ChangeList } from "./ChangeList.tsx";
import { useLoaded, usePatchSource } from "./context.tsx";
import { cardHistory, unchangedSince, versionsForCard, type HistoryEntry } from "./history.ts";
import { PatchFace } from "./PatchFace.tsx";
import { patchTestid } from "./testids.ts";

import "./patches.css";

type HistoryRead = { kind: "no-patches" } | { kind: "entries"; entries: HistoryEntry[] };

function EntryView({ entry }: { entry: HistoryEntry }): ReactElement {
  const { patch, delta, def } = entry;
  return (
    <li className="patch-history__entry" data-testid={patchTestid.historyEntry} data-version={patch.version} data-kind={delta.kind}>
      <h4 className="patch-history__head">
        <span className="patch-version">{patch.version}</span>
        <time className="patch-date" dateTime={patch.date}>
          {patch.date}
        </time>
        <span className="patch-history__patch-title">{patch.title}</span>
      </h4>
      <div className="patch-history__faces">
        <figure className="patch-history__face">
          <PatchFace def={def} face="base" />
          <figcaption>Base</figcaption>
        </figure>
        <figure className="patch-history__face patch-history__face--radiant">
          <PatchFace def={def} face="radiant" />
          <figcaption>Radiant</figcaption>
        </figure>
      </div>
      <ChangeList delta={delta} />
    </li>
  );
}

export function CardHistoryBody({ cardId }: { cardId: string }): ReactElement {
  const source = usePatchSource();
  const read = useLoaded<HistoryRead>(`history:${cardId}`, async () => {
    const patches = await source.patches();
    if (patches.length === 0) return { kind: "no-patches" };
    const index = await source.index();
    const versions = versionsForCard(cardId, patches, index);
    const snapshots = await Promise.all(versions.map((version) => source.snapshot(version)));
    const byVersion = new Map(versions.map((version, at) => [version, snapshots[at] ?? null]));
    return { kind: "entries", entries: cardHistory(cardId, patches, index, byVersion) };
  });

  if (read.status === "loading") {
    return (
      <p className="patch-status" role="status" data-testid={patchTestid.historyLoading}>
        Loading this card&rsquo;s history…
      </p>
    );
  }
  if (read.status === "error") {
    return (
      <div className="patch-status">
        <p role="alert" data-testid={patchTestid.historyError}>
          This card&rsquo;s history didn&rsquo;t load.
        </p>
        <button type="button" className="patch-retry" data-testid={patchTestid.historyRetry} onClick={read.retry}>
          Try again
        </button>
      </div>
    );
  }
  if (read.value.kind === "no-patches") {
    return (
      <p className="patch-status" data-testid={patchTestid.historyEmpty}>
        There are no patch notes yet.
      </p>
    );
  }
  const entries = read.value.entries;
  if (entries.length === 0) {
    return (
      <p className="patch-status" data-testid={patchTestid.historyEmpty}>
        No patch has recorded this card.
      </p>
    );
  }
  const since = unchangedSince(entries);
  if (since !== null) {
    return (
      <p className="patch-status" data-testid={patchTestid.historyUnchanged}>
        Unchanged since {since}.
      </p>
    );
  }
  return (
    <RefsInteractive enabled={false}>
      <ol className="patch-history__list">
        {entries.map((entry) => (
          <EntryView key={entry.patch.version} entry={entry} />
        ))}
      </ol>
    </RefsInteractive>
  );
}

export type CardHistoryProps = {
  cardId: string;
  initiallyOpen?: boolean;
};

export function CardHistory({ cardId, initiallyOpen = false }: CardHistoryProps): ReactElement {
  const [open, setOpen] = useState(initiallyOpen);
  const bodyId = useId();
  return (
    <section className="patch-history" data-testid={patchTestid.history} data-card={cardId}>
      <h3 className="patch-history__title">
        <button
          type="button"
          className="patch-history__toggle"
          aria-expanded={open}
          aria-controls={bodyId}
          data-testid={patchTestid.historyToggle}
          onClick={() => {
            setOpen((was) => !was);
          }}
        >
          History
        </button>
      </h3>
      <div id={bodyId} className="patch-history__body" hidden={!open}>
        {open ? <CardHistoryBody cardId={cardId} /> : null}
      </div>
    </section>
  );
}
