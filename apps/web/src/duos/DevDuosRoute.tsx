// `/dev/duos?seed=42&a1=first20&a2=first20&b1=first20&b2=first20` (#936 B65): a four-seat hotseat
// Duos game over the WASM duos bindings, with a seat switcher, outside production only.
//
// As `/dev/hotseat` is for a duel, this route runs the engine in the browser and renders, for
// whichever seat holds the device, the engine's own `duos_view_for` and `duos_legal_actions` through
// the same `DuosBoard` the networked route renders (CLAUDE.md rule 7: it computes no view and no
// legality of its own). The state is opaque (`DuosState`); every action goes through `duos_reduce`,
// which refuses an illegal one in `error`. The dev decks are `/dev/hotseat`'s (`game/decks.ts`),
// one per seat, and both teams play by DUOS_HUMAN_TEAM's rules.
//
// Pings travel nowhere: a seat's ping is kept here and shown to its own team's seats as the server
// would relay it (B39), so a hand-over to the teammate shows it.
//
// `window.__jackiohDuos` (development builds only) holds the state, the log and a dispatch per seat,
// for a spec or a person at the console.

import { useCallback, useEffect, useMemo, useState, type ReactElement } from "react";

import type { CardDefs } from "@jackioh/shared";
import { PING_SHOW_MS } from "@jackioh/server-config";

import type { DuosAction } from "../wire/generated/DuosAction.ts";
import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { DEFAULT_DECK_ID, DECK_IDS, resolveDeck } from "../game/decks.ts";
import { useLocale } from "../i18n/i18n.ts";
import type { PingKind, PingTarget, ReceivedPing } from "../net/duos.ts";
import { navigate, paths } from "../net/navigate.ts";
import { BackLink } from "../routes/nav.tsx";
import {
  duosBegin,
  duosCreate,
  duosLegalActions,
  duosReduce,
  duosViewFor,
  loadWasm,
  registeredCatalog,
  type DuosState,
} from "../wasm/index.ts";
import { DUOS_PINGS_KEPT } from "./config.ts";
import { DuosChromeContext, type DuosChrome } from "./context.ts";
import { DuosBoard } from "./DuosBoard.tsx";
import { DUOS_SEATS, seatLabel, teamOf } from "./seats.ts";
import { duosTestid } from "./testids.ts";
import "./duos.css";

const DEV_ONLY = import.meta.env.MODE !== "production";

/** `/dev/hotseat`'s default seed (BUILD's example URL), for the same reason. */
export const DEFAULT_DUOS_SEED = "42";

export type DevDuosParams = { seed: string } & Record<DuosSeat, string>;

/** The URL's seed and one dev deck id per seat, each defaulting as `/dev/hotseat`'s do. */
export function readDuosParams(search: string): DevDuosParams {
  const params = new URLSearchParams(search);
  const value = (key: string, fallback: string): string => {
    const given = params.get(key);
    return given === null || given === "" ? fallback : given;
  };
  return {
    seed: value("seed", DEFAULT_DUOS_SEED),
    a1: value("a1", DEFAULT_DECK_ID),
    a2: value("a2", DEFAULT_DECK_ID),
    b1: value("b1", DEFAULT_DECK_ID),
    b2: value("b2", DEFAULT_DECK_ID),
  };
}

type Ready = { kind: "ready"; state: DuosState; defs: CardDefs | null; seed: string };

type Status = { kind: "loading" } | { kind: "failed"; message: string } | Ready;

function catalogOrNull(): CardDefs | null {
  try {
    return registeredCatalog();
  } catch {
    return null;
  }
}

/** The four decks, then the game, begun: all four mulligans open. Every refusal is the engine's words. */
function start(params: DevDuosParams): Status {
  const defs = catalogOrNull();
  const decks: Partial<Record<DuosSeat, string[]>> = {};
  for (const seat of DUOS_SEATS) {
    const resolved = resolveDeck(params[seat], defs ?? {});
    if ("error" in resolved) return { kind: "failed", message: `${seatLabel(seat)}: ${resolved.error}` };
    decks[seat] = resolved.deck;
  }
  const { a1, a2, b1, b2 } = decks;
  if (a1 === undefined || a2 === undefined || b1 === undefined || b2 === undefined) {
    return { kind: "failed", message: "a seat has no deck" };
  }
  try {
    const created = duosCreate({ seed: params.seed, decks: { a1, a2, b1, b2 } });
    const begun = duosBegin(created);
    if (begun.error !== undefined) return { kind: "failed", message: begun.error };
    return { kind: "ready", state: begun.state, defs, seed: params.seed };
  } catch (cause) {
    return { kind: "failed", message: cause instanceof Error ? cause.message : String(cause) };
  }
}

/** Whether `legal` holds anything a seat owes: more than the concede every seat may always offer. */
function owes(legal: readonly DuosActionBody[]): boolean {
  return legal.some((body) => body.kind !== "concede");
}

type DevDuosHandle = {
  readonly state: DuosState;
  readonly seed: string;
  readonly seat: DuosSeat;
  readonly log: readonly DuosAction[];
  setSeat: (seat: DuosSeat) => void;
  dispatch: (seat: DuosSeat, body: DuosActionBody) => string | null;
  view: (seat: DuosSeat) => ReturnType<typeof duosViewFor>;
  legal: (seat: DuosSeat) => DuosActionBody[];
};

function Hotseat({ ready }: { ready: Ready }): ReactElement {
  useLocale();
  const [state, setState] = useState<DuosState>(ready.state);
  const [seat, setSeat] = useState<DuosSeat>("a1");
  const [log, setLog] = useState<readonly DuosAction[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [pings, setPings] = useState<readonly ReceivedPing[]>([]);
  const lookup = useMemo(() => (ready.defs === null ? null : lookupFromDefs(ready.defs)), [ready.defs]);

  const view = useMemo(() => duosViewFor(state, seat), [state, seat]);
  const legal = useMemo(() => duosLegalActions(state, seat), [state, seat]);
  const owing = useMemo(
    () => new Set(DUOS_SEATS.filter((each) => owes(duosLegalActions(state, each)))),
    [state],
  );

  /**
   * One seat's action, nonced as `/dev/hotseat` nonces (`n<count>` over accepted actions), so the
   * same seed and log fold to the same game. Answers the engine's refusal, or null.
   */
  const act = useCallback(
    (actor: DuosSeat, body: DuosActionBody): string | null => {
      const action: DuosAction = { seat: actor, nonce: `n${String(log.length)}`, body };
      const result = duosReduce(state, action);
      if (result.error !== undefined) {
        setError(result.error);
        return result.error;
      }
      setError(null);
      setState(result.state);
      setLog([...log, action]);
      return null;
    },
    [state, log],
  );

  const switchTo = useCallback((next: DuosSeat) => {
    setError(null);
    setSeat(next);
  }, []);

  const ping = useCallback(
    (kind: PingKind, target: PingTarget) => {
      const at = Date.now();
      setPings((kept) => [...kept.filter((old) => at - old.at < PING_SHOW_MS), { from: seat, ping: kind, target, at }].slice(-DUOS_PINGS_KEPT));
    },
    [seat],
  );
  // B39: a ping reaches its sender's team only.
  const teamPings = useMemo(() => pings.filter((each) => teamOf(each.from) === teamOf(seat)), [pings, seat]);

  useEffect(() => {
    if (!DEV_ONLY) return undefined;
    const handle: DevDuosHandle = {
      state,
      seed: ready.seed,
      seat,
      log,
      setSeat: switchTo,
      dispatch: act,
      view: (each) => duosViewFor(state, each),
      legal: (each) => duosLegalActions(state, each),
    };
    const host = window as unknown as { __jackiohDuos?: DevDuosHandle };
    host.__jackiohDuos = handle;
    return () => {
      if (host.__jackiohDuos === handle) delete host.__jackiohDuos;
    };
  }, [state, seat, log, ready.seed, switchTo, act]);

  const chrome: DuosChrome = useMemo(
    () => ({
      back: <BackLink showSettings={false} />,
      status: (
        <>
          seed <code>{ready.seed}</code> · seat <code>{seatLabel(seat)}</code>
        </>
      ),
      onLeave: () => {
        navigate(paths.landing);
      },
    }),
    [ready.seed, seat],
  );

  return (
    <div className="app-shell app-shell--wide">
      <header className="duos-dev-bar">
        <span>
          Duos hotseat · round <code>{view.round}</code>
        </span>
        {DUOS_SEATS.map((each) => (
          <button
            key={each}
            type="button"
            data-testid={duosTestid.devSeat(each)}
            data-owes={owing.has(each) ? "true" : "false"}
            aria-pressed={each === seat}
            onClick={() => {
              switchTo(each);
            }}
          >
            {seatLabel(each)}
            {owing.has(each) ? " •" : ""}
          </button>
        ))}
      </header>
      <CatalogContext.Provider value={lookup}>
        <DuosChromeContext.Provider value={chrome}>
          <DuosBoard
            key={seat}
            view={view}
            legal={legal}
            onAction={(body) => {
              act(seat, body);
            }}
            pings={teamPings}
            onPing={ping}
            error={error}
          />
        </DuosChromeContext.Provider>
      </CatalogContext.Provider>
    </div>
  );
}

export function DevDuosRoute(): ReactElement {
  const params = useMemo(() => readDuosParams(window.location.search), []);
  const [status, setStatus] = useState<Status>({ kind: "loading" });

  useEffect(() => {
    let cancelled = false;
    loadWasm().then(
      () => {
        if (!cancelled) setStatus(start(params));
      },
      (cause: unknown) => {
        if (!cancelled) setStatus({ kind: "failed", message: cause instanceof Error ? cause.message : String(cause) });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [params]);

  if (status.kind === "loading") {
    return (
      <div className="app-shell">
        <h1>JackiOh — Duos hotseat</h1>
        <p className="notice">Loading the engine…</p>
      </div>
    );
  }
  if (status.kind === "failed") {
    return (
      <div className="app-shell">
        <h1>JackiOh — Duos hotseat</h1>
        <p className="notice">
          No game for <code>?seed={params.seed}&amp;a1={params.a1}&amp;a2={params.a2}&amp;b1={params.b1}&amp;b2={params.b2}</code>:{" "}
          {status.message}
        </p>
        <p className="notice">
          Dev decks: <code>{DECK_IDS.join(", ")}</code>.
        </p>
      </div>
    );
  }
  return <Hotseat ready={status} />;
}

export default DevDuosRoute;
