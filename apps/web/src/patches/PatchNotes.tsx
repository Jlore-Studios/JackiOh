// Patch notes (brief B4.2 item 5; R388, R507): opened snapshots load lazily, large additions limit
// their faces. Patches sit in their release line, the newest open (R724000), each with its player
// note, never its record's source or notes (R724001). A major or a patch is an entry with its art
// (R3012001), the micro patches that follow it nested inside it, and a legend on the side jumps to
// any entry, opening its line and its cards (R3012000); an address hash does the same. Public patch
// data only (CLAUDE.md rule 7).

import { createContext, useCallback, useContext, useEffect, useId, useMemo, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { CardDef, CardDefs } from "@jackioh/shared";

import { listSeparator, readerName } from "../cards/chinese.ts";
import { EmptyState } from "../empty/EmptyState.tsx";
import { CardDetail } from "../cards/inspect/CardDetail.tsx";
import { OVERLAY_ROOT_PROPS, useModalOverlay } from "../cards/inspect/store.ts";
import { CardDefsProvider } from "../cards/refContext.tsx";
import { formatDay, t } from "../i18n/i18n.ts";
import { Skeleton, useShownAfterDelay } from "../skeleton/Skeleton.tsx";
import { ChangeList, changeSummary } from "./ChangeList.tsx";
import { PATCHES_OPEN_ON_LOAD, PATCH_LEGEND_BAND_MARGIN, RELEASES_OPEN_ON_LOAD } from "./constants.ts";
import { useLoaded, usePatchSource, type Loaded } from "./context.tsx";
import { faceShown, type CardDelta } from "./diff.ts";
import { cardCount, countsLine, currentDef, nameMatches, newestFirst, patchCards, versionBefore, type PatchCards } from "./history.ts";
import { PatchFace } from "./PatchFace.tsx";
import { PatchLegend, legendHref, type LegendLine } from "./PatchLegend.tsx";
import { releaseArtPath } from "./releaseArt.ts";
import { NOTE_SECTIONS, RELEASES, nestMicros, playerTitle, releaseGroups, technicalLines, tiers, type PatchNest, type Release } from "./releases.ts";
import type { Patch, PlayerNote } from "./source.ts";
import { patchTestid } from "./testids.ts";

import "./patches.css";

type OpenCard = (id: string) => void;

const NO_DEFS: CardDefs = {};

function patchCount(count: number): string {
  return count === 1 ? t("1 patch") : t("{n} patches", { n: count });
}

/** A card's name that opens its history; `link-button` keeps the wood-button rule off it (patches.css draws the gold link). */
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
      {readerName(def)}
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
        <h5 className="patch-card__name">
          <NameButton def={delta.def} onOpen={onOpen} />
        </h5>
        <ChangeList delta={delta} />
      </div>
    </li>
  );
}

function Section({ title, count, children }: { title: string; count: number; children: ReactElement }): ReactElement {
  const headingId = useId();
  return (
    <section className="patch-section" aria-labelledby={headingId}>
      <h4 className="patch-section__title" id={headingId}>
        {title} <span className="patch-count">({count})</span>
      </h4>
      {children}
    </section>
  );
}

function CardsView({ cards: all, query, onOpen }: { cards: PatchCards; query: string; onOpen: OpenCard }): ReactElement {
  const keep = (def: CardDef): boolean => nameMatches(def.name, query) || nameMatches(readerName(def), query);
  const changed = all.changed.filter((delta) => keep(delta.def));
  const quiet = all.dataOnly.filter((delta) => keep(delta.def));
  const added = all.added.map((group) => ({ ...group, cards: group.cards.filter(keep) })).filter((group) => group.cards.length > 0);
  const removed = all.removed.filter(keep);
  const addedCount = added.reduce((sum, group) => sum + group.cards.length, 0);
  const nothing = changed.length + quiet.length + addedCount + removed.length === 0;

  if (nothing) {
    return (
      <EmptyState
        text={query.trim() === "" ? t("This patch changed no card.") : t("No card in this patch matches “{query}”.", { query: query.trim() })}
        testId={patchTestid.noMatch}
      />
    );
  }
  return (
    <>
      {changed.length === 0 ? null : (
        <Section title={t("Changed")} count={changed.length}>
          <ul className="patch-card-list">
            {changed.map((delta) => (
              <ChangedCard key={delta.id} delta={delta} onOpen={onOpen} />
            ))}
          </ul>
        </Section>
      )}
      {quiet.length === 0 ? null : (
        <Section title={t("Changed only in details the card doesn't print")} count={quiet.length}>
          <ul className="patch-quiet">
            {quiet.map((delta) => (
              <li key={delta.id} className="patch-quiet__card" data-testid={patchTestid.dataOnly} data-card={delta.id}>
                <NameButton def={delta.def} onOpen={onOpen} />
                <span className="patch-quiet__what">{delta.changes.map(changeSummary).join(listSeparator())}</span>
              </li>
            ))}
          </ul>
        </Section>
      )}
      {added.length === 0 ? null : (
        <Section title={t("Added")} count={addedCount}>
          <div className="patch-groups">
            {added.map((group) => (
              <div key={group.label} className="patch-group" data-testid={patchTestid.addedGroup} data-group={group.label}>
                <h5 className="patch-group__title">
                  {group.token ? t("{set} tokens", { set: t(group.set) }) : t(group.set)} <span className="patch-count">({group.cards.length})</span>
                </h5>
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
        <Section title={t("Removed")} count={removed.length}>
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
    return <Skeleton variant="grid" label={t("Loading this patch's cards…")} testId={patchTestid.cardsLoading} />;
  }
  if (read.status === "error") {
    return (
      <div className="patch-status">
        <p role="alert" data-testid={patchTestid.cardsError}>
          {t("This patch's cards didn't load.")}
        </p>
        <button type="button" className="patch-retry" data-testid={patchTestid.cardsRetry} onClick={read.retry}>
          {t("Try again")}
        </button>
      </div>
    );
  }
  return (
    <div className="patch-cards" data-testid={patchTestid.cards}>
      <label className="patch-filter">
        <span className="patch-filter__label">{t("Find a card in {version}", { version: patch.version })}</span>
        <input
          type="search"
          data-testid={patchTestid.filter}
          value={query}
          placeholder={t("Card name")}
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

/** R724001: a patch's note under the page's fixed headings, one level lower under a micro patch's own title (R3012000). */
function PlayerNoteView({ note, nested = false }: { note: PlayerNote | undefined; nested?: boolean }): ReactElement | null {
  const sections = note === undefined ? [] : NOTE_SECTIONS.filter(({ key }) => note[key].length > 0);
  if (note === undefined || sections.length === 0) return null;
  const Heading = nested ? "h5" : "h4";
  return (
    <div className="patch-note" data-testid={patchTestid.note}>
      {sections.map(({ key, heading }) => (
        <section key={key} className="patch-note__section" data-testid={patchTestid.noteSection} data-section={key}>
          <Heading className="patch-note__heading">{t(heading)}</Heading>
          <ul className="patch-note__lines">
            {note[key].map((line, at) => (
              <li key={at}>{line}</li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

/** R1369000: the lines players read of the technical log of a patch, the X they were logged on. */
function TechnicalLog({ lines, nested = false }: { lines: readonly string[]; nested?: boolean }): ReactElement | null {
  if (lines.length === 0) return null;
  const Heading = nested ? "h5" : "h4";
  return (
    <div className="patch-note">
      <section className="patch-note__section" data-testid={patchTestid.technicalLog}>
        <Heading className="patch-note__heading">{t("Technical changes")}</Heading>
        <ul className="patch-note__lines">
          {lines.map((line, at) => (
            <li key={at}>{line}</li>
          ))}
        </ul>
      </section>
    </div>
  );
}

const NO_LINES: readonly string[] = [];

/** What the page's entries share: the history, the technical log, which snapshots are open and how to open a card. */
type Page = {
  readonly patches: readonly Patch[];
  readonly technical: ReadonlyMap<string, readonly string[]>;
  /** R507, R3012000: the versions whose cards are open; a snapshot loads when its version joins. */
  readonly openCards: ReadonlySet<string>;
  readonly toggleCards: (version: string) => void;
  readonly onOpenCard: OpenCard;
};

const PageContext = createContext<Page | null>(null);

function usePage(): Page {
  const page = useContext(PageContext);
  if (page === null) throw new Error("patch notes: an entry is drawn outside its page");
  return page;
}

/**
 * #2681, R3012001: the artwork of a major or a patch (releaseArt.ts), decorative: the title above it
 * names the patch. A still picture in both cases, so nothing here moves.
 */
function ReleaseArtwork({ patch, major }: { patch: Patch; major: boolean }): ReactElement {
  return (
    <div className="patch__art">
      <img
        className="patch__art-img"
        src={releaseArtPath(patch.version)}
        alt=""
        loading="lazy"
        decoding="async"
        draggable={false}
        data-testid={patchTestid.releaseArt}
        data-version={patch.version}
        data-major={major ? "true" : "false"}
      />
    </div>
  );
}

/** R507: a patch's counts line, its Show / Hide the cards control and its lazily loaded panel. */
function PatchCardsToggle({ patch }: { patch: Patch }): ReactElement {
  const { patches, openCards, toggleCards, onOpenCard } = usePage();
  const panelId = useId();
  const open = openCards.has(patch.version);
  if (patch.changes.length === 0) return <p className="patch__counts">{t("No card changed.")}</p>;
  return (
    <>
      <p className="patch__counts">{countsLine(patch)}</p>
      <button
        type="button"
        className="patch__toggle"
        aria-expanded={open}
        aria-controls={panelId}
        data-testid={patchTestid.toggle}
        onClick={() => {
          toggleCards(patch.version);
        }}
      >
        {open ? t("Hide the cards") : t("Show the {cards}", { cards: cardCount(patch.changes.length) })}
      </button>
      <div id={panelId} className="patch__panel" hidden={!open}>
        {open ? <PatchCardsPanel patch={patch} patches={patches} onOpen={onOpenCard} /> : null}
      </div>
    </>
  );
}

/** R3012000: a micro patch inside the entry of the patch it follows: its version, date and notes, with no art. */
function MicroEntry({ patch }: { patch: Patch }): ReactElement {
  const { technical } = usePage();
  const titleId = useId();
  return (
    <article
      className="patch-micro"
      id={patch.version}
      tabIndex={-1}
      data-entry={patch.version}
      data-testid={patchTestid.micro}
      data-version={patch.version}
      aria-labelledby={titleId}
    >
      <header className="patch-micro__head">
        <h4 className="patch-micro__title" id={titleId}>
          <span className="patch-version">{patch.version}</span> <span className="patch__name">{playerTitle(patch)}</span>
        </h4>
        <time className="patch-date" dateTime={patch.date}>
          {formatDay(patch.date)}
        </time>
      </header>
      <PlayerNoteView note={patch.player} nested />
      <TechnicalLog lines={technical.get(patch.version) ?? NO_LINES} nested />
      <PatchCardsToggle patch={patch} />
    </article>
  );
}

/**
 * R3012000, R3012001: a major's or a patch's entry, with its art and the micro patches that follow it
 * inside it, oldest first. A micro patch with no major or patch before it in its line is an entry of
 * its own, with no art.
 */
function PatchEntry({ nest }: { nest: PatchNest }): ReactElement {
  const { technical } = usePage();
  const { patch, tier, micros } = nest;
  const titleId = useId();
  return (
    <article
      className="patch"
      id={patch.version}
      tabIndex={-1}
      data-entry={patch.version}
      data-testid={patchTestid.patch}
      data-version={patch.version}
      data-tier={tier}
      aria-labelledby={titleId}
    >
      <header className="patch__head">
        <h3 className="patch__title" id={titleId}>
          <span className="patch-version">{patch.version}</span> <span className="patch__name">{playerTitle(patch)}</span>
        </h3>
        <p className="patch__meta">
          <time className="patch-date" dateTime={patch.date}>
            {formatDay(patch.date)}
          </time>
        </p>
      </header>
      {tier === "micro" ? null : <ReleaseArtwork patch={patch} major={tier === "major"} />}
      <PlayerNoteView note={patch.player} />
      <TechnicalLog lines={technical.get(patch.version) ?? NO_LINES} />
      <PatchCardsToggle patch={patch} />
      {micros.length === 0 ? null : (
        <ol className="patch-micros" aria-label={t("Micro patches")}>
          {micros.map((micro) => (
            <li key={micro.version}>
              <MicroEntry patch={micro} />
            </li>
          ))}
        </ol>
      )}
    </article>
  );
}

/**
 * #2052: the detail's place while a card opened from a patch loads, or after it failed. A dialog as the
 * detail is (`useModalOverlay`), so Escape, the scrim and Back close it too; its wait stays text (R2108000).
 */
function OpeningCard({ read, onClose }: { read: Exclude<Loaded<CardDef | null>, { status: "ready" }>; onClose: () => void }): ReactElement {
  const closeButton = useRef<HTMLButtonElement>(null);
  const modal = useModalOverlay(onClose, closeButton);
  return createPortal(
    <div className="inspect-layer inspect-layer--detail" {...OVERLAY_ROOT_PROPS}>
      <div className="inspect-scrim" aria-hidden="true" {...modal.dismissProps} />
      <div className="inspect-detail" data-testid={patchTestid.opening} role="dialog" aria-modal="true" aria-label={t("Card")}>
        {read.status === "loading" ? (
          <p className="patch-status" role="status" data-testid={patchTestid.openingLoading}>
            {t("Loading this card…")}
          </p>
        ) : (
          <div className="patch-status">
            <p role="alert" data-testid={patchTestid.openingError}>
              {t("This card didn't load.")}
            </p>
            <button type="button" className="patch-retry" data-testid={patchTestid.openingRetry} onClick={read.retry}>
              {t("Try again")}
            </button>
          </div>
        )}
        <div className="inspect-actions">
          <button ref={closeButton} type="button" className="inspect-close" data-testid={patchTestid.openingClose} {...modal.dismissProps}>
            {t("Close")}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}

function OpenedCard({ id, patches, onClose }: { id: string; patches: readonly Patch[]; onClose: () => void }): ReactElement | null {
  const source = usePatchSource();
  const read = useLoaded<CardDef | null>(`open:${id}`, async () => currentDef(id, patches, await source.index(), (version) => source.snapshot(version)));
  // #2052, R2108000: a card that answers within SKELETON_DELAY_MS opens with no wait shown.
  const waited = useShownAfterDelay();
  if (read.status === "ready") return read.value === null ? null : <CardDetail def={read.value} onClose={onClose} historyOpen />;
  if (read.status === "loading" && !waited) return null;
  return <OpeningCard read={read} onClose={onClose} />;
}

/** R3012000: the id of a release line's heading, which the legend and an address hash jump to; a version's own is the version. */
const lineAnchor = (releaseId: string): string => `line-${releaseId}`;

/** R724000: one release line as a bubble. Its open state is the page's, so a jump can open it (R3012000). */
function ReleaseBubble({
  release,
  group,
  nests,
  open,
  onToggle,
}: {
  release: Release;
  /** Every patch of the line, micros included, newest first: its span and count. */
  group: readonly Patch[];
  nests: readonly PatchNest[];
  open: boolean;
  onToggle: () => void;
}): ReactElement {
  const headingId = useId();
  const panelId = useId();
  const newest = group[0]?.version ?? "";
  const oldest = group.at(-1)?.version ?? "";
  return (
    <section
      className="patch-set"
      id={lineAnchor(release.id)}
      tabIndex={-1}
      data-testid={patchTestid.set}
      data-set={release.id}
      aria-labelledby={headingId}
    >
      <h2 className="patch-set__heading" id={headingId}>
        <button
          type="button"
          className="link-button patch-set__toggle"
          data-testid={patchTestid.setToggle}
          aria-expanded={open}
          aria-controls={panelId}
          onClick={onToggle}
        >
          <span className="patch-set__name">{t(release.name)}</span>
          <span className="patch-set__span">{oldest === newest ? newest : `${oldest} – ${newest}`}</span>
          <span className="patch-set__count">{patchCount(group.length)}</span>
        </button>
      </h2>
      <div id={panelId} className="patch-set__panel" hidden={!open}>
        {open ? (
          <>
            {release.summary === "" ? null : <p className="patch-set__summary">{release.summary}</p>}
            {release.technical.length === 0 ? null : (
              <section className="patch-tech" data-testid={patchTestid.technical}>
                <h3 className="patch-tech__title">{t("Technical changes")}</h3>
                <ul className="patch-tech__lines">
                  {release.technical.map((line, at) => (
                    <li key={at}>{line}</li>
                  ))}
                </ul>
              </section>
            )}
            <ol className="patch-list">
              {nests.map((nest) => (
                <li key={nest.patch.version}>
                  <PatchEntry nest={nest} />
                </li>
              ))}
            </ol>
          </>
        ) : null}
      </div>
    </section>
  );
}

/** What an address or a legend link can name: a release line or a version, the line it sits in, and whether it has cards to open. */
type Anchor = { readonly line: string; readonly version: boolean; readonly cards: boolean };

/** R3012000: the entry the address's hash names, when this page has one; a hash that is no entry (or no valid escape) names none. */
function hashedAnchor(anchors: ReadonlyMap<string, Anchor>): string | null {
  if (typeof window === "undefined") return null;
  try {
    const anchor = decodeURIComponent(window.location.hash.slice(1));
    return anchors.has(anchor) ? anchor : null;
  } catch {
    return null;
  }
}

function PatchList({ patches, releases }: { patches: readonly Patch[]; releases: readonly Release[] }): ReactElement {
  const source = usePatchSource();
  const [openId, setOpenId] = useState<string | null>(null);
  const newest = patches[patches.length - 1];
  const defs: Loaded<CardDefs | null> = useLoaded(`defs:${newest?.version ?? ""}`, () =>
    newest === undefined ? Promise.resolve(null) : source.snapshot(newest.version),
  );
  // Until the technical log loads, or if it fails, the page reads without its lines.
  const log = useLoaded("technical", () => source.technical());
  const technical = technicalLines(log.status === "ready" ? log.value : []);

  // R3012000: the release lines, each patch's micros nested under it, and what the legend and the address can name.
  const { groups, nests, anchors, legend } = useMemo(() => {
    const tierList = tiers(patches);
    const tierOf = new Map(patches.map((patch, at) => [patch.version, tierList[at] ?? "patch"] as const));
    const lines = releaseGroups(patches, releases);
    const nested = lines.map((line) => nestMicros(line.patches, tierOf));
    const named = new Map<string, Anchor>();
    const entries: LegendLine[] = lines.map((line, at) => {
      named.set(lineAnchor(line.release.id), { line: line.release.id, version: false, cards: false });
      for (const patch of line.patches) named.set(patch.version, { line: line.release.id, version: true, cards: patch.changes.length > 0 });
      return {
        id: line.release.id,
        anchor: lineAnchor(line.release.id),
        name: line.release.name,
        entries: (nested[at] ?? []).map((nest) => ({ version: nest.patch.version, tier: nest.tier, micros: nest.micros.map((micro) => micro.version) })),
      };
    });
    return { groups: lines, nests: nested, anchors: named, legend: entries };
  }, [patches, releases]);

  // R724000, R507: the newest line and the newest patch's cards open on arrival; a hash in the address opens its entry too.
  const [openLines, setOpenLines] = useState<ReadonlySet<string>>(() => {
    const open = new Set(groups.slice(0, RELEASES_OPEN_ON_LOAD).map((group) => group.release.id));
    const hashed = hashedAnchor(anchors);
    if (hashed !== null) open.add(anchors.get(hashed)?.line ?? "");
    return open;
  });
  const [openCards, setOpenCards] = useState<ReadonlySet<string>>(() => {
    const open = new Set(newestFirst(patches).slice(0, PATCHES_OPEN_ON_LOAD).map((patch) => patch.version));
    const hashed = hashedAnchor(anchors);
    if (hashed !== null && anchors.get(hashed)?.cards === true) open.add(hashed);
    return open;
  });
  const [jump, setJump] = useState<{ readonly anchor: string } | null>(() => {
    const hashed = hashedAnchor(anchors);
    return hashed === null ? null : { anchor: hashed };
  });
  const [current, setCurrent] = useState<string | null>(() => {
    const hashed = hashedAnchor(anchors);
    return hashed !== null && anchors.get(hashed)?.version === true ? hashed : null;
  });

  /** Opens the entry or line an anchor names, then asks the page to scroll to it. */
  const reveal = useCallback(
    (anchor: string): boolean => {
      const target = anchors.get(anchor);
      if (target === undefined) return false;
      setOpenLines((was) => (was.has(target.line) ? was : new Set(was).add(target.line)));
      if (target.cards) setOpenCards((was) => (was.has(anchor) ? was : new Set(was).add(anchor)));
      if (target.version) setCurrent(anchor);
      setJump({ anchor });
      return true;
    },
    [anchors],
  );
  const onJump = (anchor: string): void => {
    if (reveal(anchor)) window.history.replaceState(null, "", legendHref(anchor));
  };

  useEffect(() => {
    const onHash = (): void => {
      const hashed = hashedAnchor(anchors);
      if (hashed !== null) reveal(hashed);
    };
    window.addEventListener("hashchange", onHash);
    return () => {
      window.removeEventListener("hashchange", onHash);
    };
  }, [anchors, reveal]);

  // The jump itself, once the entry's line is open in the document: scrolls at once (nothing here moves) and focuses it.
  useEffect(() => {
    if (jump === null) return;
    const target = document.getElementById(jump.anchor);
    if (target !== null) {
      if (typeof target.scrollIntoView === "function") target.scrollIntoView({ block: "start" });
      target.focus({ preventScroll: true });
    }
    setJump(null);
  }, [jump]);

  // R3012000: the entry in view is the last one, in document order (so a micro before its patch), that touches the top band of the viewport.
  const openKey = [...openLines].join("|");
  useEffect(() => {
    if (typeof IntersectionObserver === "undefined") return;
    const entries = Array.from(document.querySelectorAll<HTMLElement>("[data-entry]"));
    if (entries.length === 0) return;
    const touching = new Set<string>();
    const observer = new IntersectionObserver(
      (records) => {
        for (const record of records) {
          const version = (record.target as HTMLElement).dataset.entry;
          if (version === undefined) continue;
          if (record.isIntersecting) touching.add(version);
          else touching.delete(version);
        }
        const inView = entries.map((entry) => entry.dataset.entry ?? "").filter((version) => touching.has(version));
        const last = inView.at(-1);
        if (last !== undefined) setCurrent(last);
      },
      { rootMargin: PATCH_LEGEND_BAND_MARGIN },
    );
    for (const entry of entries) observer.observe(entry);
    return () => {
      observer.disconnect();
    };
  }, [openKey, groups]);

  const page: Page = {
    patches,
    technical,
    openCards,
    toggleCards: (version) => {
      setOpenCards((was) => {
        const next = new Set(was);
        if (!next.delete(version)) next.add(version);
        return next;
      });
    },
    onOpenCard: setOpenId,
  };
  return (
    <CardDefsProvider defs={defs.status === "ready" ? (defs.value ?? NO_DEFS) : NO_DEFS}>
      <PageContext.Provider value={page}>
        <div className="patch-notes__layout">
          <PatchLegend lines={legend} current={current} onJump={onJump} />
          <div className="patch-notes__main">
            <ol className="patch-sets">
              {groups.map(({ release, patches: group }, at) => (
                <li key={release.id}>
                  <ReleaseBubble
                    release={release}
                    group={group}
                    nests={nests[at] ?? []}
                    open={openLines.has(release.id)}
                    onToggle={() => {
                      setOpenLines((was) => {
                        const next = new Set(was);
                        if (!next.delete(release.id)) next.add(release.id);
                        return next;
                      });
                    }}
                  />
                </li>
              ))}
            </ol>
          </div>
        </div>
      </PageContext.Provider>
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

export function PatchNotes({ releases = RELEASES }: { releases?: readonly Release[] }): ReactElement {
  const source = usePatchSource();
  const read = useLoaded("patches", () => source.patches());

  if (read.status === "loading") {
    return <Skeleton variant="rows" label={t("Loading the patch notes…")} testId={patchTestid.loading} />;
  }
  if (read.status === "error") {
    return (
      <div className="patch-status">
        <p role="alert" data-testid={patchTestid.error}>
          {t("The patch notes didn't load. Check your connection and try again.")}
        </p>
        <button type="button" className="patch-retry" data-testid={patchTestid.retry} onClick={read.retry}>
          {t("Try again")}
        </button>
      </div>
    );
  }
  if (read.value.length === 0) {
    return <EmptyState text={t("There are no patch notes yet.")} testId={patchTestid.empty} />;
  }
  return <PatchList patches={read.value} releases={releases} />;
}
