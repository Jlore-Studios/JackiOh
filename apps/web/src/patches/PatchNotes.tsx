// The Patch notes page's body (brief B4.2 item 5, R388, R507): every patch, newest first, with its
// version, date, title, source and notes, and the cards it touched.
//
// - The newest PATCHES_OPEN_ON_LOAD patches show their cards on arrival; every other patch shows
//   its counts and a "Show the cards" control, and loads its snapshot and the one before it only
//   when opened, so the page never downloads a snapshot nobody asked to see.
// - A patch's cards, filtered by name: the changed cards whose faces print the change as faces from
//   that patch's snapshot, each with what changed marked (ChangeList); the changed cards whose every
//   change lies in data a face does not print (lines of code, the cards a text names) as names with
//   a line saying what changed; the added cards as names grouped by set, each set's tokens apart, so
//   a patch that adds hundreds of cards is a list to scan and not hundreds of faces; and the removed
//   cards as names.
// - Every name and face opens the card as it stands now in the collection's detail view, with its
//   History section open.
//
// It reads the patch source and nothing else, and decides nothing (CLAUDE.md rule 7).

import { useId, useState, type ReactElement } from "react";

import type { CardDef, CardDefs } from "@jackioh/shared";

import { isGlitch } from "../cards/glitch.ts";
import { CardDetail } from "../cards/inspect/CardDetail.tsx";
import { CardDefsProvider } from "../cards/refContext.tsx";
import { ChangeList, changeSummary } from "./ChangeList.tsx";
import { PATCHES_OPEN_ON_LOAD } from "./constants.ts";
import { useLoaded, usePatchSource, type Loaded } from "./context.tsx";
import { faceShown, type CardDelta } from "./diff.ts";
import { currentDef, nameMatches, newestFirst, patchCards, versionBefore, type PatchCards } from "./history.ts";
import { PatchFace } from "./PatchFace.tsx";
import type { Patch } from "./source.ts";
import { patchTestid } from "./testids.ts";

import "./patches.css";

type OpenCard = (id: string) => void;

const NO_DEFS: CardDefs = {};

/** "1 card", "206 cards". */
function cards(count: number): string {
  return `${String(count)} ${count === 1 ? "card" : "cards"}`;
}

/** "206 added · 111 changed", from the patch's own record; nothing for a kind it has none of. */
export function countsLine(patch: Patch): string {
  // R674: Glitch is not counted, as it is not listed (history.ts `patchCards`).
  const count = (kind: Patch["changes"][number]["kind"]): number =>
    patch.changes.filter((change) => change.kind === kind && !isGlitch(change.id)).length;
  const parts = [
    [count("added"), "added"],
    [count("changed"), "changed"],
    [count("removed"), "removed"],
  ] as const;
  return parts
    .filter(([n]) => n > 0)
    .map(([n, word]) => `${cards(n)} ${word}`)
    .join(" · ");
}

/**
 * A card's name as a control that opens its history. `link-button` keeps the tavern's wood-button
 * rule off it; patches.css draws it as the tavern's gold prose link.
 */
function NameButton({ def, onOpen }: { def: CardDef; onOpen: OpenCard }): ReactElement {
  return (
    <button
      type="button"
      className="link-button patch-name"
      data-testid={patchTestid.openCard}
      data-card={def.id}
      onClick={() => {
        onOpen(def.id);
      }}
    >
      {def.name}
    </button>
  );
}

function ChangedCard({ delta, onOpen }: { delta: Extract<CardDelta, { kind: "changed" }>; onOpen: OpenCard }): ReactElement {
  return (
    <li className="patch-card" data-testid={patchTestid.changedCard} data-card={delta.id}>
      {/* The face opens the card too, for a mouse or a finger; the keyboard has the name. */}
      <div
        className="patch-card__face"
        onClick={() => {
          onOpen(delta.id);
        }}
      >
        <PatchFace def={delta.def} face={faceShown(delta)} />
      </div>
      <div className="patch-card__body">
        <h4 className="patch-card__name">
          <NameButton def={delta.def} onOpen={onOpen} />
        </h4>
        <ChangeList delta={delta} />
      </div>
    </li>
  );
}

function Section({ title, count, children }: { title: string; count: number; children: ReactElement }): ReactElement {
  const headingId = useId();
  return (
    <section className="patch-section" aria-labelledby={headingId}>
      <h3 className="patch-section__title" id={headingId}>
        {title} <span className="patch-count">({count})</span>
      </h3>
      {children}
    </section>
  );
}

/** A patch's cards, filtered by `query`. */
function CardsView({ cards: all, query, onOpen }: { cards: PatchCards; query: string; onOpen: OpenCard }): ReactElement {
  const keep = (def: CardDef): boolean => nameMatches(def.name, query);
  const changed = all.changed.filter((delta) => keep(delta.def));
  const quiet = all.dataOnly.filter((delta) => keep(delta.def));
  const added = all.added.map((group) => ({ ...group, cards: group.cards.filter(keep) })).filter((group) => group.cards.length > 0);
  const removed = all.removed.filter(keep);
  const addedCount = added.reduce((sum, group) => sum + group.cards.length, 0);
  const nothing = changed.length + quiet.length + addedCount + removed.length === 0;

  if (nothing) {
    return (
      <p className="patch-status" data-testid={patchTestid.noMatch}>
        {query.trim() === "" ? "This patch changed no card." : `No card in this patch matches “${query.trim()}”.`}
      </p>
    );
  }
  return (
    <>
      {changed.length === 0 ? null : (
        <Section title="Changed" count={changed.length}>
          <ul className="patch-card-list">
            {changed.map((delta) => (
              <ChangedCard key={delta.id} delta={delta} onOpen={onOpen} />
            ))}
          </ul>
        </Section>
      )}
      {quiet.length === 0 ? null : (
        <Section title="Changed only in details the card doesn’t print" count={quiet.length}>
          <ul className="patch-quiet">
            {quiet.map((delta) => (
              <li key={delta.id} className="patch-quiet__card" data-testid={patchTestid.dataOnly} data-card={delta.id}>
                <NameButton def={delta.def} onOpen={onOpen} />
                <span className="patch-quiet__what">{delta.changes.map(changeSummary).join("; ")}</span>
              </li>
            ))}
          </ul>
        </Section>
      )}
      {added.length === 0 ? null : (
        <Section title="Added" count={addedCount}>
          <div className="patch-groups">
            {added.map((group) => (
              <div key={group.label} className="patch-group" data-testid={patchTestid.addedGroup} data-group={group.label}>
                <h4 className="patch-group__title">
                  {group.label} <span className="patch-count">({group.cards.length})</span>
                </h4>
                <ul className="patch-names">
                  {group.cards.map((def) => (
                    <li key={def.id}>
                      <NameButton def={def} onOpen={onOpen} />
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </Section>
      )}
      {removed.length === 0 ? null : (
        <Section title="Removed" count={removed.length}>
          <ul className="patch-names">
            {removed.map((def) => (
              <li key={def.id} data-testid={patchTestid.removed} data-card={def.id}>
                <NameButton def={def} onOpen={onOpen} />
              </li>
            ))}
          </ul>
        </Section>
      )}
    </>
  );
}

/** A patch's cards: its snapshot and the one before it, loaded on first show, then the filter and the lists. */
function PatchCardsPanel({ patch, patches, onOpen }: { patch: Patch; patches: readonly Patch[]; onOpen: OpenCard }): ReactElement {
  const source = usePatchSource();
  const [query, setQuery] = useState("");
  const read = useLoaded<PatchCards>(`cards:${patch.version}`, async () => {
    const beforeVersion = versionBefore(patches, patch.version);
    const [after, before] = await Promise.all([
      source.snapshot(patch.version),
      beforeVersion === null ? Promise.resolve(null) : source.snapshot(beforeVersion),
    ]);
    if (after === null) throw new Error(`No snapshot of ${patch.version}`);
    return patchCards(patch, after, before);
  });

  if (read.status === "loading") {
    return (
      <p className="patch-status" role="status" data-testid={patchTestid.cardsLoading}>
        Loading this patch&rsquo;s cards…
      </p>
    );
  }
  if (read.status === "error") {
    return (
      <div className="patch-status">
        <p role="alert" data-testid={patchTestid.cardsError}>
          This patch&rsquo;s cards didn&rsquo;t load.
        </p>
        <button type="button" className="patch-retry" data-testid={patchTestid.cardsRetry} onClick={read.retry}>
          Try again
        </button>
      </div>
    );
  }
  return (
    <div className="patch-cards" data-testid={patchTestid.cards}>
      <label className="patch-filter">
        <span className="patch-filter__label">Find a card in {patch.version}</span>
        <input
          type="search"
          data-testid={patchTestid.filter}
          value={query}
          placeholder="Card name"
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
      </label>
      <CardsView cards={read.value} query={query} onOpen={onOpen} />
    </div>
  );
}

function PatchEntry({
  patch,
  patches,
  initiallyOpen,
  onOpen,
}: {
  patch: Patch;
  patches: readonly Patch[];
  initiallyOpen: boolean;
  onOpen: OpenCard;
}): ReactElement {
  const [open, setOpen] = useState(initiallyOpen);
  const titleId = useId();
  const panelId = useId();
  const counts = countsLine(patch);
  return (
    <article className="patch" data-testid={patchTestid.patch} data-version={patch.version} aria-labelledby={titleId}>
      <header className="patch__head">
        <h2 className="patch__title" id={titleId}>
          <span className="patch-version">{patch.version}</span> <span className="patch__name">{patch.title}</span>
        </h2>
        <p className="patch__meta">
          <time className="patch-date" dateTime={patch.date}>
            {patch.date}
          </time>
          {patch.source === "" ? null : <span className="patch__source"> · {patch.source}</span>}
        </p>
      </header>
      {patch.notes === "" ? null : <p className="patch__notes">{patch.notes}</p>}
      {patch.changes.length === 0 ? (
        <p className="patch__counts">No card changed.</p>
      ) : (
        <>
          <p className="patch__counts">{counts}</p>
          <button
            type="button"
            className="patch__toggle"
            aria-expanded={open}
            aria-controls={panelId}
            data-testid={patchTestid.toggle}
            onClick={() => {
              setOpen((was) => !was);
            }}
          >
            {open ? "Hide the cards" : `Show the ${cards(patch.changes.length)}`}
          </button>
          <div id={panelId} className="patch__panel" hidden={!open}>
            {open ? <PatchCardsPanel patch={patch} patches={patches} onOpen={onOpen} /> : null}
          </div>
        </>
      )}
    </article>
  );
}

/** The card a name opened, as it stands now, in the detail view with its history open. */
function OpenedCard({ id, patches, onClose }: { id: string; patches: readonly Patch[]; onClose: () => void }): ReactElement | null {
  const source = usePatchSource();
  const read = useLoaded<CardDef | null>(`open:${id}`, async () => currentDef(id, patches, await source.index(), (version) => source.snapshot(version)));
  if (read.status !== "ready" || read.value === null) return null;
  return <CardDetail def={read.value} onClose={onClose} historyOpen />;
}

/** The list, once patches.json has loaded: references resolve against the newest snapshot. */
function PatchList({ patches }: { patches: readonly Patch[] }): ReactElement {
  const source = usePatchSource();
  const [openId, setOpenId] = useState<string | null>(null);
  const newest = patches[patches.length - 1];
  const defs: Loaded<CardDefs | null> = useLoaded(`defs:${newest?.version ?? ""}`, () =>
    newest === undefined ? Promise.resolve(null) : source.snapshot(newest.version),
  );
  const ordered = newestFirst(patches);
  return (
    <CardDefsProvider defs={defs.status === "ready" ? (defs.value ?? NO_DEFS) : NO_DEFS}>
      <ol className="patch-list">
        {ordered.map((patch, at) => (
          <li key={patch.version}>
            <PatchEntry patch={patch} patches={patches} initiallyOpen={at < PATCHES_OPEN_ON_LOAD} onOpen={setOpenId} />
          </li>
        ))}
      </ol>
      {openId === null ? null : (
        <OpenedCard
          key={openId}
          id={openId}
          patches={patches}
          onClose={() => {
            setOpenId(null);
          }}
        />
      )}
    </CardDefsProvider>
  );
}

export function PatchNotes(): ReactElement {
  const source = usePatchSource();
  const read = useLoaded("patches", () => source.patches());

  if (read.status === "loading") {
    return (
      <p className="patch-status" role="status" data-testid={patchTestid.loading}>
        Loading the patch notes…
      </p>
    );
  }
  if (read.status === "error") {
    return (
      <div className="patch-status">
        <p role="alert" data-testid={patchTestid.error}>
          The patch notes didn&rsquo;t load. Check your connection and try again.
        </p>
        <button type="button" className="patch-retry" data-testid={patchTestid.retry} onClick={read.retry}>
          Try again
        </button>
      </div>
    );
  }
  if (read.value.length === 0) {
    return (
      <p className="patch-status" data-testid={patchTestid.empty}>
        There are no patch notes yet.
      </p>
    );
  }
  return <PatchList patches={read.value} />;
}
