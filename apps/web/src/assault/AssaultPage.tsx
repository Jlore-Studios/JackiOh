// `/assault`: the screen selected solely by the Assault state. Part D supplies the fight screen;
// this part loads it by the frozen named export without making its absent source a build dependency.

import {
  Component,
  Suspense,
  lazy,
  useCallback,
  useEffect,
  useRef,
  useState,
  type ComponentType,
  type ErrorInfo,
  type ReactElement,
  type ReactNode,
} from "react";

import { t } from "../i18n/i18n.ts";
import { getDecks, type DecksResponse } from "../net/api.ts";
import { readFailureOf, useAccount, type Account } from "../net/gate.ts";
import type { SavedDecks } from "../practice/PracticeSetup.tsx";
import { Briefing } from "./day/Briefing.tsx";
import { DayScreen } from "./day/DayScreen.tsx";
import {
  AbandonedScreen,
  AfterLevelScreen,
  LostScreen,
  RepelledScreen,
} from "./day/ResultScreens.tsx";
import { useAssault } from "./store.ts";
import { assaultTestid } from "./testids.ts";
import type { AssaultAction, AssaultDay, AssaultState } from "./types.ts";
import "./assault.css";

type FightProps = { state: AssaultState; day: AssaultDay; onAction(action: AssaultAction): void };

const fightPath = "./fight/FightScreen.tsx";
const FightScreen = lazy(async (): Promise<{ default: ComponentType<FightProps> }> => {
  const loaded = await import(/* @vite-ignore */ fightPath) as { AssaultFightScreen: ComponentType<FightProps> };
  return { default: loaded.AssaultFightScreen };
});

class FightBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  override state = { failed: false };

  static getDerivedStateFromError(): { failed: boolean } {
    return { failed: true };
  }

  override componentDidCatch(_error: Error, _info: ErrorInfo): void {
    // The reconcile supplies part D. A failed chunk is reported in place and leaves the attempt kept.
  }

  override render(): ReactNode {
    if (this.state.failed) return <p role="alert">{t("The fight screen did not load. Reload to try again.")}</p>;
    return this.props.children;
  }
}

function useSavedDecks(account: Account, loadDecks: (token: string) => Promise<DecksResponse> = getDecks): SavedDecks {
  const [loaded, setLoaded] = useState<{ token: string; saved: SavedDecks } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const load = useRef(loadDecks);
  load.current = loadDecks;
  const token = account.kind === "ready" && account.me.profile.status === "active" ? account.token : null;

  useEffect(() => {
    if (token === null) return;
    let canceled = false;
    load.current(token).then(
      (answer) => {
        if (canceled) return;
        setLoaded({
          token,
          saved: answer.decks.length === 0
            ? { kind: "none" }
            : {
                kind: "ready",
                decks: answer.decks.map((deck) => ({
                  name: deck.name,
                  cards: [...deck.cards],
                  portrait: deck.portrait ?? null,
                })),
              },
        });
      },
      (cause: unknown) => {
        if (canceled) return;
        const { retryAfterMs } = readFailureOf(cause);
        setLoaded({
          token,
          saved: retryAfterMs === undefined ? { kind: "unavailable" } : { kind: "unavailable", retryAfterMs },
        });
      },
    );
    return () => { canceled = true; };
  }, [token, attempt]);

  const retry = useCallback((): void => {
    setLoaded(null);
    setAttempt((before) => before + 1);
  }, []);

  switch (account.kind) {
    case "loading":
      return { kind: "checking" };
    case "anonymous":
      return { kind: "anonymous" };
    case "error":
      return account.retry === undefined
        ? { kind: "unavailable" }
        : { kind: "unavailable", retry: account.retry, retryAfterMs: account.retryAfterMs };
    case "ready":
      if (token === null) return { kind: "inactive" };
      if (loaded === null || loaded.token !== token) return { kind: "checking" };
      return loaded.saved.kind === "unavailable" ? { ...loaded.saved, retry } : loaded.saved;
  }
}

export default function AssaultPage(): ReactElement {
  const assault = useAssault();
  const saved = useSavedDecks(useAccount());
  const [afterLevel, setAfterLevel] = useState<number | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const { state } = assault;

  let screen: ReactElement;
  if (state === null) {
    screen = <DayScreen assault={assault} saved={saved} />;
  } else if (state.phase === "fight") {
    screen = (
      <FightBoundary key={`${String(state.setup.attempt)}-${String(state.level)}`}>
        <Suspense fallback={<p>{t("Loading the fight…")}</p>}>
          <FightScreen
            state={state}
            day={assault.day}
            onAction={(action) => {
              const result = assault.act(action);
              setRefusal(result);
              if (result === null && action.kind === "fightOver" && action.won && state.level < assault.constants.levels) {
                setAfterLevel(state.level);
              }
            }}
          />
        </Suspense>
      </FightBoundary>
    );
  } else if (state.phase === "lost") {
    screen = (
      <LostScreen
        record={assault.record}
        onTryAgain={() => {
          const startLevel = assault.record.checkpoints.at(-1) ?? 1;
          setRefusal(assault.start({
            date: state.setup.date,
            deck: state.setup.deck,
            source: state.setup.source,
            startLevel,
          }));
        }}
        onBack={assault.dismiss}
      />
    );
  } else if (state.phase === "repelled") {
    screen = <RepelledScreen record={assault.record} turns={state.turns} onBack={assault.dismiss} />;
  } else if (state.phase === "abandoned") {
    screen = <AbandonedScreen record={assault.record} onBack={assault.dismiss} />;
  } else if (afterLevel !== null) {
    screen = <AfterLevelScreen beaten={afterLevel} next={state.level} onContinue={() => { setAfterLevel(null); }} />;
  } else {
    const level = assault.day.levels.find((candidate) => candidate.level === state.level);
    screen = level === undefined ? (
      <p role="alert">{t("This Assault level could not be found. Reload to try again.")}</p>
    ) : (
      <Briefing
        content={assault.content}
        level={level}
        onFight={() => { setRefusal(assault.act({ kind: "fight" })); }}
        onAbandon={() => { setRefusal(assault.act({ kind: "abandon" })); }}
      />
    );
  }

  return (
    <div className="assault" data-testid={assaultTestid.page}>
      {screen}
      {refusal === null ? null : <p className="notice" role="alert">{refusal}</p>}
    </div>
  );
}
