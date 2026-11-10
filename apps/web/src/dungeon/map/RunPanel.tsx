// The dungeon run's Your run panel (#3501): the HUD's button, and the Dialog it opens with the run's
// seed (copyable, since seeds are shared, §15.2), its deck, its relics, its blessings and its curses.
// Presentation only (CLAUDE.md rule 7): it reads the run and its content and sends nothing to either.
// `run.css`, which `RunScreen` loads, supplies the card and passive classes it reuses.

import { useRef, useState, type ReactElement } from "react";

import type { RunContent, RunState } from "@jackioh/run";

import { Dialog } from "../../game/Dialog.tsx";
import { t } from "../../i18n/i18n.ts";
import { attempt } from "../../net/errors.ts";
import { CurseChoice, PassiveChoice, RunCardFace } from "./RunDisplay.tsx";
import { dungeonMapTestid } from "./testids.ts";
import "./run-panel.css";

/** The seed, which a person shares so a friend can play the same run, and the button that copies it. */
function SeedRow({ seed }: { seed: string }): ReactElement {
  const [copied, setCopied] = useState(false);
  const copy = (): void => {
    attempt(() => navigator.clipboard.writeText(seed)).then(
      () => {
        setCopied(true);
      },
      // A refused or missing clipboard stays silent: the seed stays on screen to be read out.
      () => undefined,
    );
  };
  return (
    <section className="dungeon-run-panel__section">
      <h3 className="dungeon-run-panel__heading">{t("Seed")}</h3>
      <div className="dungeon-run-panel__seed">
        <code className="dungeon-run-panel__seed-code" data-testid={dungeonMapTestid.panelSeed}>
          {seed}
        </code>
        <button
          type="button"
          className="button-secondary"
          data-testid={dungeonMapTestid.panelSeedCopy}
          onClick={copy}
        >
          {copied ? t("Copied") : t("Copy seed")}
        </button>
      </div>
    </section>
  );
}

type PassiveListProps = {
  title: string;
  empty: string;
  testid: string;
  /** Each entry's `<li>`, drawn by the caller: the relics carry a level, the blessings and curses do not. */
  children: ReactElement[];
};

/** A heading over a list of passives, or over the line saying there are none. */
function PassiveList({ title, empty, testid, children }: PassiveListProps): ReactElement {
  return (
    <section className="dungeon-run-panel__section">
      <h3 className="dungeon-run-panel__heading">{title}</h3>
      <div data-testid={testid}>
        {children.length === 0 ? (
          <p className="dungeon-run-note">{empty}</p>
        ) : (
          <ul className="dungeon-run-choices">{children}</ul>
        )}
      </div>
    </section>
  );
}

/**
 * The HUD's Your run button, and the panel it opens: the seed first, so a long deck cannot push it out of
 * view on a phone, then the deck, the relics, the blessings and the curses.
 */
export function RunPanel({ run, content }: { run: RunState; content: RunContent }): ReactElement {
  const [open, setOpen] = useState(false);
  const opener = useRef<HTMLButtonElement>(null);
  // Safari does not focus a button a click lands on, so the focus goes back to it by hand.
  const close = (): void => {
    setOpen(false);
    opener.current?.focus({ preventScroll: true });
  };
  return (
    <>
      <button
        type="button"
        ref={opener}
        className="button-secondary dungeon-run-hud__open"
        data-testid={dungeonMapTestid.yourRun}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => {
          setOpen(true);
        }}
      >
        {t("Your run")}
      </button>
      {open ? (
        <Dialog
          role="dialog"
          testid={dungeonMapTestid.panel}
          closeTestid={dungeonMapTestid.panelClose}
          className="dungeon-run-panel"
          title={t("Your run")}
          onClose={close}
          actions={[]}
        >
          <SeedRow seed={run.seed} />
          <section className="dungeon-run-panel__section">
            <h3 className="dungeon-run-panel__heading">
              {t("Deck")}: {run.deck.length}
            </h3>
            {run.deck.length === 0 ? (
              <p className="dungeon-run-note">{t("No cards yet.")}</p>
            ) : (
              <ul className="dungeon-run-panel__deck" data-testid={dungeonMapTestid.panelDeck}>
                {run.deck.map((card, index) => (
                  <li key={`${String(index)}-${card.id}`}>
                    <RunCardFace
                      id={card.id}
                      radiant={card.radiant}
                      buffs={card.buffs}
                      content={content}
                      testid={dungeonMapTestid.panelCard(index)}
                    />
                  </li>
                ))}
              </ul>
            )}
          </section>
          <PassiveList title={t("Relics")} empty={t("No relics")} testid={dungeonMapTestid.panelArtifacts}>
            {run.artifacts.map((artifact, index) => (
              <li
                className="dungeon-run-passive-row"
                data-testid={dungeonMapTestid.panelArtifact(index)}
                key={`${String(index)}-${artifact.id}`}
              >
                <PassiveChoice id={artifact.id} level={artifact.level} content={content} />
              </li>
            ))}
          </PassiveList>
          <PassiveList title={t("Blessings")} empty={t("No blessings")} testid={dungeonMapTestid.panelBlessings}>
            {run.blessings.map((id, index) => (
              <li
                className="dungeon-run-passive-row"
                data-testid={dungeonMapTestid.panelBlessing(index)}
                key={`${String(index)}-${id}`}
              >
                <PassiveChoice id={id} content={content} />
              </li>
            ))}
          </PassiveList>
          <PassiveList title={t("Curses")} empty={t("No curses")} testid={dungeonMapTestid.panelCurses}>
            {run.curses.map((id, index) => (
              <li
                className="dungeon-run-passive-row"
                data-testid={dungeonMapTestid.panelCurse(index)}
                key={`${String(index)}-${id}`}
              >
                <CurseChoice id={id} content={content} />
              </li>
            ))}
          </PassiveList>
        </Dialog>
      ) : null}
    </>
  );
}
