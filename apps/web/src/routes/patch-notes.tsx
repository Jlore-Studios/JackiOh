// `/patch-notes`: every card patch, newest first (SPEC §10.10, R375). Public, like `/privacy`, and
// linked from the site footer, which names the current version.
//
// Each patch shows its version, date, title and notes, the issues, pull requests and commits it came
// from on GitHub, a "Reconstructed" badge on a version that was named only later, and the cards it
// created and changed, each of which opens that card's detail view, History included. The lists come
// from `changes.json` (cards/patches.ts), so the page loads no snapshot until a History is opened.
//
// Presentation only (CLAUDE.md rule 7): the page reads the public catalog and the patch list.

import { useState, type ReactElement } from "react";

import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";

import { CardDefsProvider, CardDetail } from "../cards/index.ts";
import {
  CURRENT_VERSION,
  PATCHES,
  PATCH_CARDS,
  RECONSTRUCTED_NOTE,
  commitUrl,
  sourceLabel,
  sourceUrl,
} from "../cards/patches.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./patch-notes.css";

export const patchNotesTestid = {
  screen: "patch-notes-screen",
  current: "patch-notes-current",
  patch: "patch-notes-patch",
  badge: "patch-notes-badge",
  source: "patch-notes-source",
  commit: "patch-notes-commit",
  card: "patch-notes-card",
} as const;

/** JSON widens the unions to strings; the catalog is proved against SPEC §8 in packages/cards. */
const CATALOG = catalogJson as unknown as CardDefs;

/** How many characters of a commit id the page prints, as git's short form does. */
const SHORT_COMMIT = 7;

function CardList({
  label,
  ids,
  onOpen,
}: {
  label: string;
  ids: readonly string[];
  onOpen: (id: string) => void;
}): ReactElement | null {
  if (ids.length === 0) return null;
  return (
    <div className="patch-notes__cards">
      <h4>
        {label} ({ids.length})
      </h4>
      <ul>
        {ids.map((id) => {
          const def = CATALOG[id];
          return (
            <li key={id}>
              {def === undefined ? (
                <span>{id}</span>
              ) : (
                <button
                  type="button"
                  className="patch-notes__card"
                  data-testid={patchNotesTestid.card}
                  data-card={id}
                  onClick={() => {
                    onOpen(id);
                  }}
                >
                  {def.token ? def.name : `#${def.index} ${def.name}`}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export default function PatchNotesRoute(): ReactElement {
  const [detail, setDetail] = useState<string | null>(null);
  const detailDef = detail === null ? undefined : CATALOG[detail];
  const newestFirst = [...PATCHES].reverse();

  return (
    <div className="app-shell tavern patch-notes-screen" data-testid={patchNotesTestid.screen}>
      <BackLink />

      <article className="panel panel--auth patch-notes">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>Patch notes</h2>
        <p className="patch-notes__current" data-testid={patchNotesTestid.current}>
          The current version is {CURRENT_VERSION}.
        </p>
        <p className="patch-notes__intro">
          Every change to the cards, newest first. Open a card to see its whole history. {RECONSTRUCTED_NOTE}
        </p>

        {newestFirst.map((patch) => {
          const cards = PATCH_CARDS[patch.version] ?? { created: [], changed: [], removed: [] };
          const headingId = `patch-${patch.version}`;
          return (
            <section
              key={patch.version}
              className="patch-notes__patch"
              aria-labelledby={headingId}
              data-testid={patchNotesTestid.patch}
              data-version={patch.version}
            >
              <h3 id={headingId}>
                {patch.version} · {patch.title}
                {patch.reconstructed ? (
                  <span className="patch-notes__badge" data-testid={patchNotesTestid.badge} title={RECONSTRUCTED_NOTE}>
                    Reconstructed
                  </span>
                ) : null}
              </h3>
              <p className="patch-notes__date">
                <time dateTime={patch.date}>{patch.date}</time>
              </p>
              {patch.notes === "" ? null : <p>{patch.notes}</p>}
              <p className="patch-notes__links">
                {patch.sources.map((source) => (
                  <a
                    key={`${source.kind}-${String(source.number)}`}
                    href={sourceUrl(source)}
                    data-testid={patchNotesTestid.source}
                    rel="noopener noreferrer"
                  >
                    {sourceLabel(source)}
                  </a>
                ))}
                {patch.commits.map((commit) => (
                  <a key={commit} href={commitUrl(commit)} data-testid={patchNotesTestid.commit} rel="noopener noreferrer">
                    <code>{commit.slice(0, SHORT_COMMIT)}</code>
                  </a>
                ))}
              </p>
              <CardList label="New cards" ids={cards.created} onOpen={setDetail} />
              <CardList label="Changed cards" ids={cards.changed} onOpen={setDetail} />
              <CardList label="Removed cards" ids={cards.removed} onOpen={setDetail} />
            </section>
          );
        })}
      </article>

      {detailDef === undefined ? null : (
        <CardDefsProvider defs={CATALOG}>
          <CardDetail
            def={detailDef}
            onClose={() => {
              setDetail(null);
            }}
          />
        </CardDefsProvider>
      )}
    </div>
  );
}
