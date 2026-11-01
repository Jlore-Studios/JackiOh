// Entry point and the route table. The client is a `PlayerView` renderer (CLAUDE.md rule 7, SPEC
// §10.8): this file picks a route, applies §9.4's gate once, and gets out of the way.
//
// Routing is still a pathname switch rather than a router dependency, but the pathname now comes
// from `usePathname()` (`net/navigate.ts`) instead of being read once at boot, because a redirect
// has to re-render: `cy.location("pathname")` in specs 09 and 10 reads the real URL, so the gate
// moves the URL with `history.replaceState` and this component follows it.
//
// THE GATE IS UX, NOT SECURITY. SPEC §9.4: "A pending account can log in, verify its email and see
// the code screen, and nothing else: no collection, loadout, queue or match." The server is what
// enforces that — every endpoint but `/api/auth/me` and `/api/codes/*` is declared `auth: "active"`
// and answers 403 `account_pending`, and `wsServer.ts` runs the same `assertActive` on the upgrade.
// What `Gated` does is read `GET /api/auth/me`, the endpoint §9.4 provides precisely so "the client
// knows to show the code screen", and send the browser to the screen that will actually work. It
// lives in exactly one place so there is one thing to change when §9.4 grows a status. The same
// read says whether an active account still owes the username prompt (R1435), which the gate then
// shows in place of the screen until the player picks a name or skips (`auth/UsernamePrompt.tsx`),
// and whether the account owes the age check or agreement to the terms in force (R757000, R757001),
// which comes first (`auth/ConsentPrompt.tsx`).
//
// NO DEAD ENDS (docs/polish/5-sign-in.md, B40). Every panel this file draws itself — the gate's
// error, the banned account, the 404, a screen whose code failed to load, and a check that is
// taking too long — offers a way out: home, sign out, and try again where trying again can help.
// The landing page is `routes/landing.tsx`, imported statically because it is the first screen most
// visits see. The panels sit in the same tavern frame as the sign-in screens (`auth/tavern.css`),
// since they are the waypoints between the landing and those screens.
//
// ONE GATE PER SCREEN. Every gated route renders `<Gated>` at the same place in the tree, so React
// would reuse one instance (and the account it read) across in-app moves: a redemption on
// `/invite` went on to `/decks` still holding the "pending" account it read before, and bounced
// straight back. Each route's gate is keyed by its path, so a move re-reads `/api/auth/me`. It opens
// at once on the page's last answer only for an active account that owes nothing, and reads behind
// it (R2630000); a pending account, a redirect or a prompt always waits for the read.
//
// ONE SOCKET PER SESSION (R194). The gate renews a session shortly before its token expires and
// hands the new token down. The match screen is keyed by the provider session the token belongs to,
// so a renewal of the same session keeps its socket (the new token waits for the next reconnect),
// and only a different session, or account, opens a new one.
//
// EMAILED LINKS ON ANY PATH (R193). Before the route switch runs, a link is scrubbed from the
// address bar, on `/login` too (whose screen is a lazy chunk that may be slow or fail to load), and
// one that landed anywhere else (Supabase's Site URL fallback) is handed to `/login`.
//
// A ROOM LINK (R767). Before the route switch runs, a room link on /play is taken out of the address
// bar and kept for this tab (net/roomLink.ts), so the gate's way to /login cannot lose it; the lobby
// uses it once.
//
// THE QUEUE FOLLOWS THE PLAYER (R765). A player queued on `/play` may go anywhere in the client while
// they wait. The route table's own `useQueueFollow` (`net/liveGame.ts`) watches the account on every
// screen but `/play`, which watches for itself, and when the queue pairs them it says so over the
// screen (`QueueFound`) and takes them to the game.
//
// THE ENGINE LOADS ALONGSIDE (#778). The engine's module is 2.31 MB gzipped, and the landing, sign-in,
// the reset and code screens and the legal pages never call it. The page paints once the language is
// in; EngineGate holds every other screen until the module is in, with a retry if it never arrives.
// A gated screen that calls it (`<Gated engine>`) asks for it, and for its own code, as its gate
// mounts, beside the account read (R2630001).
//
// SCREEN CODE ON INTENT (#2116, R2116000). The screens are chunks of one map
// (`routes/chunks.tsx`), which the prefetch shares: intent toward a control starts its screen's
// code, and a game screen's engine, before the click, and a screen whose code is already in skips
// the loading panel.
//
// FRIENDS FIND THE PLAYER ANYWHERE (#936, B47, B57). While the tab holds a session, the shell sends the
// presence heartbeat (`together/presence.ts`) and shows the challenge toast (`together/ChallengeToast.tsx`)
// over every screen, whichever is open: both are this file's, as the queue's follow is, so neither
// waits on a screen's gate nor restarts with it.
//
// A COMMAND MENU ON EVERY MENU (R2118000). Ctrl+K (Cmd+K) opens the command menu
// (`routes/CommandMenu.tsx`, `routes/commandEntries.ts`) on every menu screen: the common actions,
// every menu page and the Almanac shelf's cards by name, each a `navigate` move. It is never
// mounted on a board route, nor over a board sharing a menu's path (`game/boardShown.ts`), and it
// is keyed by the path, so a move closes it. Presentation only (CLAUDE.md rule 7).

import {
  Component,
  StrictMode,
  Suspense,
  useEffect,
  useState,
  type ErrorInfo,
  type ReactElement,
  type ReactNode,
} from "react";
import { createRoot } from "react-dom/client";

import { GATE_SLOW_NOTICE_SECONDS } from "@jackioh/server-config";
import { AccountError } from "./auth/AccountError.tsx";
import { adoptAuthRedirect, sessionIdFromToken } from "./auth/redirect.ts";
import { dropStaleSignUpConsent } from "./auth/consent.ts";
import ConsentPrompt from "./auth/ConsentPrompt.tsx";
import { shellTestid } from "./auth/testids.ts";
import UsernamePrompt from "./auth/UsernamePrompt.tsx";
import { useCardPicksAccountSync } from "./cards/cardPicksSync.ts";
import { ViewerAccountCosmetics, ViewerFullArt } from "./cards/ownedFullArt.ts";
import { countLocaleVisit, initLocale, t, useLocale } from "./i18n/i18n.ts";
import { useAccount, type Account } from "./net/gate.ts";
import { canonicalUrlFor, documentTitleFor } from "./net/head.ts";
import { useQueueFollow } from "./net/liveGame.ts";
import { adoptCoopLink, adoptRoomLink, readRoomLink } from "./net/roomLink.ts";
import { useSettingsAccountSync } from "./settings/accountSync.ts";
import {
  currentPath,
  duosIdOf,
  loginPath,
  matchIdOf,
  navigate,
  paths,
  seriesIdOf,
  usePathname,
  watchIdOf,
} from "./net/navigate.ts";
import { readSession } from "./net/session.ts";
import type { MeResponse } from "./net/api.ts";
import { ChallengeToast } from "./together/ChallengeToast.tsx";
import { usePresenceHeartbeat } from "./together/presence.ts";
import { watchFriendOf } from "./together/watchLink.ts";
// Static, not lazy: the gate's own panels offer "Sign out", which must work synchronously from a
// screen that failed to load anything else, so account.tsx is in the entry chunk either way.
import AccountRoute, { signOut, signOutLabel, useSigningOut } from "./routes/account.tsx";
import CommandMenu from "./routes/CommandMenu.tsx";
import { QueueFound } from "./routes/GameBanner.tsx";
import { routeChunks, startScreen, watchRouteIntent } from "./routes/chunks.tsx";
import LandingRoute from "./routes/landing.tsx";
import { followInApp } from "./routes/nav.tsx";
import { readSettings } from "./settings/store.ts";
import { Skeleton, useShownAfterDelay, type SkeletonVariant } from "./skeleton/Skeleton.tsx";
import { loadWasm, wasmLoaded } from "./wasm/index.ts";

import "./index.css";
import "./auth/tavern.css";

const HotseatRoute = routeChunks.hotseat?.Screen ?? null;
const DevDuosRoute = routeChunks.duosDev?.Screen ?? null;
const DuosRoute = routeChunks.duos.Screen;
const ReplaysRoute = routeChunks.replays.Screen;
const LoginRoute = routeChunks.login.Screen;
const ResetPasswordRoute = routeChunks.resetPassword.Screen;
const InviteRoute = routeChunks.invite.Screen;
const DecksRoute = routeChunks.decks.Screen;
const PlayRoute = routeChunks.play.Screen;
const MatchRoute = routeChunks.match.Screen;
const SeriesRoute = routeChunks.series.Screen;
const PracticeRoute = routeChunks.practice.Screen;
const PrivacyRoute = routeChunks.privacy.Screen;
const TermsRoute = routeChunks.terms.Screen;
const PurchasesRoute = routeChunks.purchases.Screen;
const AccessibilityRoute = routeChunks.accessibility.Screen;
const AiRoute = routeChunks.ai.Screen;
const CopyrightRoute = routeChunks.copyright.Screen;
const PatchNotesRoute = routeChunks.patchNotes.Screen;
const AlmanacRoute = routeChunks.almanac.Screen;
const LeaderboardRoute = routeChunks.leaderboard.Screen;
const CollectionRoute = routeChunks.collection.Screen;
const ShopRoute = routeChunks.shop.Screen;
const StatsRoute = routeChunks.stats.Screen;
const CreatorRoute = routeChunks.creator.Screen;
const TogetherRoute = routeChunks.together.Screen;
const WatchRoute = routeChunks.watch.Screen;

/** This file's chrome testids, kept in `auth/testids.ts` for Cypress and re-exported for existing imports. */
export { shellTestid };

/** The tab title and canonical link live in `net/head.ts`, so the build can name pages as the app does. */
export { SITE_NAME, canonicalUrlFor, documentTitleFor } from "./net/head.ts";

/**
 * The frame every panel this file draws sits in: the wordmark on the tavern board. Exported for
 * the screens that wait the same way (the match screen before its first view).
 */
export function ShellPanel({
  testId,
  shown = true,
  children,
}: {
  testId?: string;
  shown?: boolean;
  children: ReactNode;
}): ReactElement {
  return (
    <div className="app-shell tavern shell-panel" data-testid={testId} data-shown={shown ? "true" : "false"}>
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        {children}
      </section>
    </div>
  );
}

/** "Home": back to the landing page, in place on a plain click and a real link otherwise. */
function HomeLink({ testId, label }: { testId: string; label?: string }): ReactElement {
  return (
    <a
      className="button-secondary"
      href={paths.landing}
      data-testid={testId}
      onClick={followInApp(paths.landing)}
    >
      {label ?? t("Home")}
    </a>
  );
}

/** "Sign out": `routes/account.tsx` clears the device, revokes the session and reloads `/`. */
function SignOutButton(): ReactElement {
  const leaving = useSigningOut();
  return (
    <button
      type="button"
      data-testid={shellTestid.signOut}
      disabled={leaving}
      aria-busy={leaving}
      onClick={() => {
        signOut();
      }}
    >
      {signOutLabel(leaving)}
    </button>
  );
}

/** A path the client serves nothing at. Said in a player's words; the path is shown, never followed. */
function NotFound({ path }: { path: string }): ReactElement {
  // Split around the path, so the sentence is one dictionary entry.
  const [nothingBefore = "", nothingAfter = ""] = t("There is nothing at {path}. The link may be wrong or out of date.").split(
    "{path}",
  );
  return (
    <ShellPanel testId={shellTestid.notFound}>
      <p className="notice">{t("That page doesn't exist.")}</p>
      <p className="auth-hint">
        {nothingBefore}
        <code>{path}</code>
        {nothingAfter}
      </p>
      <div className="row">
        <HomeLink testId={shellTestid.notFoundHome} label={t("Back to the start")} />
      </div>
    </ShellPanel>
  );
}

export type LoadingProps = {
  what: string;
  slow?: boolean;
  /** The panel's own testid, for a screen that waits in it (the match screen's `match-connecting`). */
  testId?: string;
  /** More to say under the sentence: the match screen's connection line. */
  children?: ReactNode;
  /** #2115: draw a skeleton of this shape, the sentence its hidden status line. */
  skeleton?: SkeletonVariant;
};

/**
 * A holding panel. With `slow`, a wait that runs past `GATE_SLOW_NOTICE_SECONDS` says why it may
 * be slow (Render's free tier sleeps and takes about a minute to wake) and offers the way out, so a
 * server or network that never answers is not a trap.
 */
export function Loading({ what, slow = false, testId, children, skeleton }: LoadingProps): ReactElement {
  const [late, setLate] = useState(false);
  useEffect(() => {
    if (!slow) return;
    const timer = window.setTimeout(() => {
      setLate(true);
    }, GATE_SLOW_NOTICE_SECONDS * 1000);
    return () => {
      window.clearTimeout(timer);
    };
  }, [slow]);
  // #2108, R2108000: a wait drawn as a skeleton shows once it outlasts SKELETON_DELAY_MS, and a
  // sentence that waits on nothing shows at once.
  const waited = useShownAfterDelay();

  return (
    <ShellPanel {...(testId === undefined ? {} : { testId })} shown={skeleton === undefined || waited}>
      {skeleton === undefined ? (
        <p className="notice shell-panel__loading" data-testid={shellTestid.loading} role="status">
          {t(what)}
        </p>
      ) : (
        <Skeleton variant={skeleton} label={t(what)} testId={shellTestid.loading} />
      )}
      {children}
      {late ? (
        <>
          <p className="notice" data-testid={shellTestid.slow}>
            {t("This is taking a while. The server may be waking up, which can take up to a minute.")}
          </p>
          <div className="row">
            <HomeLink testId={shellTestid.home} />
            {readSession() === null ? null : <SignOutButton />}
          </div>
        </>
      ) : null}
    </ShellPanel>
  );
}

/** The gate's error panel: the sentence, and the three ways on. A stated wait holds the retry. */
function GateError({ account }: { account: Extract<Account, { kind: "error" }> }): ReactElement {
  return (
    <ShellPanel>
      <AccountError
        message={account.message}
        retryAfterMs={account.retryAfterMs}
        onRetry={() => {
          // Re-reads /api/auth/me in place, without a page load.
          account.retry?.();
        }}
        testId={shellTestid.error}
        retryTestId={shellTestid.retry}
      >
        <HomeLink testId={shellTestid.home} />
        <SignOutButton />
      </AccountError>
    </ShellPanel>
  );
}

/**
 * A screen whose code could not be loaded: a lazy chunk fails on a flaky network, and after every
 * deploy for a tab left open (the old hashed file is gone). Without this the whole root would unmount
 * to a blank page. A reload fetches the current build; the home link is a real page load too.
 */
type LoadFailedState = { failed: boolean };

export class RouteErrorBoundary extends Component<{ children: ReactNode }, LoadFailedState> {
  override state: LoadFailedState = { failed: false };

  static getDerivedStateFromError(): LoadFailedState {
    return { failed: true };
  }

  override componentDidCatch(error: unknown, info: ErrorInfo): void {
    // Kept for whoever opens the console; the player gets the panel below.
    console.error("A screen failed to load", error, info.componentStack);
  }

  override render(): ReactNode {
    if (!this.state.failed) return this.props.children;
    return (
      <ShellPanel testId={shellTestid.loadFailed}>
        <p className="notice" role="alert">
          {t("This screen didn't load. Check your connection, then reload the page.")}
        </p>
        <div className="row">
          <button
            type="button"
            className="button-primary"
            data-testid={shellTestid.reload}
            onClick={() => {
              window.location.reload();
            }}
          >
            {t("Reload")}
          </button>
          <a className="button-secondary" href={paths.landing} data-testid={shellTestid.loadFailedHome}>
            {t("Home")}
          </a>
        </div>
      </ShellPanel>
    );
  }
}

// The engine's module (#778)

/** Where the engine's WebAssembly module stands, for a screen that calls it. */
type EngineLoad = "loading" | "ready" | "failed";

/**
 * Holds a screen that calls the engine until its module is in, in the panel a lazy screen waits in.
 * A module that did not arrive gets a panel whose retry asks `loadWasm()` again in place: a failed
 * download is not cached (`wasm/index.ts`), so the next call fetches it anew.
 */
function EngineGate({ children }: { children: ReactNode }): ReactElement {
  const [load, setLoad] = useState<EngineLoad>(() => (wasmLoaded() ? "ready" : "loading"));
  useEffect(() => {
    if (load !== "loading") return;
    let live = true;
    void loadWasm().then(
      () => {
        if (live) setLoad("ready");
      },
      (cause: unknown) => {
        if (!live) return;
        console.error("the engine's WebAssembly module did not load", cause);
        setLoad("failed");
      },
    );
    return () => {
      live = false;
    };
  }, [load]);

  if (load === "loading") return <Loading what="Loading…" slow skeleton="panel" testId={shellTestid.engineLoading} />;
  if (load === "failed") {
    return (
      <ShellPanel testId={shellTestid.engineFailed}>
        <p className="notice" role="alert">
          {t("The game didn't load. Check your connection, then try again.")}
        </p>
        <div className="row">
          <button
            type="button"
            className="button-primary"
            data-testid={shellTestid.retry}
            onClick={() => {
              setLoad("loading");
            }}
          >
            {t("Try again")}
          </button>
          <HomeLink testId={shellTestid.home} />
          {readSession() === null ? null : <SignOutButton />}
        </div>
      </ShellPanel>
    );
  }
  return <>{children}</>;
}

// The gate (SPEC §9.4)

export type ReadyAccount = { token: string; me: MeResponse };

export type GatedProps = {
  /** `/invite`, the one screen a pending account may see, opens to both statuses; all else is `active`-only. */
  allowPending?: boolean;
  /**
   * R757001: `/account` opens whether or not the account owes the consent screen, so a player who
   * will not agree can still delete their account or sign out there.
   */
  consentExempt?: boolean;
  /**
   * R2630001: the screen calls the engine (its children open with `EngineGate`), so the gate asks for
   * the engine's module as it mounts, beside `/api/auth/me`, along with the screen's own code.
   */
  engine?: boolean;
  children: (account: ReadyAccount) => ReactElement;
};

/** Where a given account belongs, or null when it may stay where it is. */
export function redirectFor(account: Account, allowPending: boolean): string | null {
  if (account.kind === "anonymous") {
    // A renewal the provider refused (R194) ended the session, and the sign-in screen says so.
    // The reason is the only thing ever put in that query (`loginPath`), never a destination.
    return account.reason === "expired" ? loginPath({ reason: "expired" }) : paths.login;
  }
  if (account.kind === "ready" && account.me.profile.status === "pending" && !allowPending) {
    return paths.invite;
  }
  return null;
}

/**
 * R1435: the profiles this page has let past the username prompt: one a gated screen was shown to
 * while active, and one that answered the prompt here. Page-wide, since each route mounts a gate of
 * its own, whose state would forget and raise it over the next screen (the match a queue just
 * paired). A pending account's `/invite` screen does not count, so the prompt still follows it.
 */
const usernamePromptPassed = new Set<string>();

/**
 * R757000, R757001: the profiles that answered the consent screen on this page. The answer counts
 * at once, as the username prompt's does, whatever the re-read after it finds.
 */
const consentAnswered = new Set<string>();

/** Test seam: a new page, which has let no profile past either prompt yet. */
export function resetUsernamePromptForTests(): void {
  usernamePromptPassed.clear();
  consentAnswered.clear();
}

export function Gated({ allowPending = false, consentExempt = false, engine = false, children }: GatedProps): ReactElement {
  const account = useAccount();
  // R2630001: the screen's code, and the engine when it calls it, start beside the account read
  // (whose effect `useAccount` has already queued), not once it answers; with no session the gate
  // sends the player to sign in, so nothing starts. Keyed by path, a gate mounts once per screen.
  useEffect(() => {
    if (readSession() !== null) startScreen(currentPath(), engine);
  }, [engine]);
  // R757001: this gate has shown its screen to an active account that owed nothing. A later read
  // that finds the terms owed (they changed while the player was here) leaves the screen up, and
  // the next screen asks. Per gate, since each screen mounts its own (ONE GATE PER SCREEN).
  const [screenShown] = useState(() => new Set<string>());
  // R634: an active account's game settings are kept level with this device's, on every gated screen.
  useSettingsAccountSync(account);
  // R1834000: and its picks of each card, which live on the account, the device's being their cache.
  useCardPicksAccountSync(account);
  const target = redirectFor(account, allowPending);
  // R1435: renders the gate again once the prompt is answered here, since the page-wide set above
  // renders nothing by changing.
  const [, setAnswered] = useState(0);

  // In an effect, never during render: `navigate` dispatches an event that re-renders every
  // `usePathname` subscriber, which would be an update during this render.
  useEffect(() => {
    if (target === null) return;
    navigate(target, { replace: true });
  }, [target]);

  if (account.kind === "loading") return <Loading what="Checking your account…" slow skeleton="panel" />;
  if (account.kind === "anonymous") return <Loading what="Sign in to continue." />;
  if (account.kind === "error") return <GateError account={account} />;

  const status = account.me.profile.status;
  if (status === "pending" && !allowPending) return <Loading what="Redeem an invite code first." />;
  if (status === "banned") {
    // Not in SPEC, and no R-row: §9.4 names a screen for two of its three statuses. This is a terminal
    // message, not a decision: the server already refuses a banned account at every door (R145).
    return (
      <ShellPanel>
        <p className="notice" data-testid={shellTestid.error} role="alert">
          {t("This account is banned.")}
        </p>
        <div className="row">
          <HomeLink testId={shellTestid.home} />
          <SignOutButton />
        </div>
      </ShellPanel>
    );
  }
  // R1435: an active account owes the username prompt until it picks or skips, and meets it before
  // any gated screen; a server that sends no `username` owes none. Never while the account is in a
  // match or a Conquest series: a reload mid-game opens the board on a new page, and the prompt waits
  // for the next sign-in.
  const profileId = account.me.profile.id;
  const playing = account.me.currentMatchId !== null || (account.me.currentSeriesId ?? null) !== null;
  // R757000, R757001: an account owing the age check or agreement to the terms in force meets the
  // consent screen first, a pending one before the code screen, whose redemption the server refuses
  // until it is answered.
  const consentOwed = account.me.ageCheckOwed === true || account.me.termsOwed === true;
  if (
    consentOwed &&
    !consentExempt &&
    !playing &&
    !consentAnswered.has(profileId) &&
    !(status === "active" && screenShown.has(profileId))
  ) {
    return (
      <ConsentPrompt
        token={account.token}
        me={account.me}
        onAnswered={() => {
          consentAnswered.add(profileId);
          setAnswered((count) => count + 1);
        }}
      />
    );
  }
  if (
    status === "active" &&
    account.me.username?.promptOwed === true &&
    !playing &&
    !usernamePromptPassed.has(profileId)
  ) {
    return (
      <UsernamePrompt
        token={account.token}
        name={account.me.username.name}
        onAnswered={() => {
          usernamePromptPassed.add(profileId);
          setAnswered((count) => count + 1);
        }}
      />
    );
  }
  // Remembered from the render itself, so the screen's first render already counts. Adding a member
  // the set may hold already is the same however often React renders this.
  if (status === "active") {
    usernamePromptPassed.add(profileId);
    screenShown.add(profileId);
  }

  // R1313000: the screen's own copies print the full arts the account owns, once ownership decides;
  // a pending account owns none and reads nothing. R1833000: and its pickers and the viewer's side
  // offer and wear the account cosmetics it owns.
  const ownershipToken = status === "active" ? account.token : null;
  return (
    <ViewerFullArt token={ownershipToken}>
      <ViewerAccountCosmetics token={ownershipToken}>{children({ token: account.token, me: account.me })}</ViewerAccountCosmetics>
    </ViewerFullArt>
  );
}

// The tab title and the canonical link

/**
 * Every route is served from one index.html, so a static canonical link would point every route at
 * `/`. The one `<link rel="canonical">` is kept here instead, following the path, and removed on a
 * page that does not exist.
 */
function useDocumentHead(path: string): void {
  // The tab title is in the reader's language and follows a switch (#711).
  const locale = useLocale();
  useEffect(() => {
    document.title = documentTitleFor(path);
    const href = canonicalUrlFor(path);
    let link = document.head.querySelector<HTMLLinkElement>('link[rel="canonical"]');
    if (href === null) {
      link?.remove();
      return;
    }
    if (link === null) {
      link = document.createElement("link");
      link.rel = "canonical";
      document.head.append(link);
    }
    link.href = href;
  }, [path, locale]);
}

// The route table

/**
 * The provider session a token belongs to (its `session_id` claim), else the token itself (the
 * end-to-end fixtures name none, and never renew). See ONE SOCKET PER SESSION above.
 */
function sessionKeyOf(token: string): string {
  return sessionIdFromToken(token) ?? token;
}

export function App(): ReactElement {
  // The whole tree redraws in the new language when the reader switches (#711): the screens' `t()`
  // calls run again, and nothing below remounts, so a match in progress keeps its socket.
  useLocale();
  // Before the first read of the path, on any path (`/login`'s screen is a lazy chunk that may fail to
  // load): an emailed link is scrubbed, and one that landed elsewhere (Supabase's Site URL fallback)
  // moved to `/login` (R193).
  useState(() => adoptAuthRedirect(paths.login));
  // R767: a room link leaves the address bar and waits in this tab before the gate can send a signed-out visitor to sign in.
  useState(adoptRoomLink);
  // R2094000: a co-op link does the same for /together.
  useState(adoptCoopLink);
  const path = usePathname();
  useDocumentHead(path);
  // R765: a pairing finds the player on any screen; see THE QUEUE FOLLOWS THE PLAYER above.
  const found = useQueueFollow(path);
  // #936: see FRIENDS FIND THE PLAYER ANYWHERE above. Read at every render, so a sign-in in this page
  // (which moves to another path) starts both.
  const signedIn = readSession() !== null;
  usePresenceHeartbeat(signedIn);
  // R2116000: see SCREEN CODE ON INTENT above.
  useEffect(() => watchRouteIntent(document), []);

  // #778: every screen but the engine-free ones (the signed-out Play hub among them, R1578000) waits in EngineGate, inside the account gate so its redirects do not wait.
  const route = ((): ReactElement => {
    if (path === paths.landing) return <LandingRoute />;
    if (path === paths.login) return <LoginRoute />;
    if (path === paths.resetPassword) return <ResetPasswordRoute />;
    // `key={path}` on every gate: see ONE GATE PER SCREEN above.
    // The gate's own read of the account is handed down, so the code screen does not read it twice.
    if (path === paths.invite) {
      return <Gated key={path} allowPending>{(account) => <InviteRoute account={account} />}</Gated>;
    }
    if (path === paths.decks) {
      return (
        <Gated key={path} engine>
          {() => (
            <EngineGate>
              <DecksRoute />
            </EngineGate>
          )}
        </Gated>
      );
    }
    if (path === paths.play) {
      // R1578000: the hub is every mode's front door, so a visitor with no account sees it too, its
      // online panel a sign-in. A room link waits through the sign-in (R767), and a stored session,
      // even a stale one, still meets the gate.
      if (!signedIn && readRoomLink() === null) return <PlayRoute token={null} />;
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <PlayRoute token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }
    // `allowPending`: a pending account still has an email, a status and a way to sign out,
    // and being unable to sign out of the screen that tells you to redeem a code is the trap
    // this route exists to remove.
    if (path === paths.account) {
      return (
        <Gated key={path} allowPending consentExempt engine>
          {(account) => (
            <EngineGate>
              <AccountRoute token={account.token} me={account.me} />
            </EngineGate>
          )}
        </Gated>
      );
    }

    if (path === paths.practice) {
      return (
        <EngineGate>
          <PracticeRoute />
        </EngineGate>
      );
    }
    // #920: the card creator keeps its cards on the account, so it is behind the gate like the decks.
    if (path === paths.creator) {
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <CreatorRoute token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }
    if (path === paths.leaderboard) {
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <LeaderboardRoute token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }
    if (path === paths.collection) {
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <CollectionRoute token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }
    // #1577: the shop reads only the server's projections, never the engine, so no EngineGate.
    if (path === paths.shop) {
      return (
        <Gated key={path}>
          {(account) => <ShopRoute token={account.token} />}
        </Gated>
      );
    }
    if (path === paths.privacy) return <PrivacyRoute />;
    if (path === paths.terms) return <TermsRoute />;
    // R1665000: the page is public, its disclaimer anyone's; with a session it reads the odds and the account's counters itself.
    if (path === paths.purchases) return <PurchasesRoute />;
    if (path === paths.accessibility) return <AccessibilityRoute />;
    if (path === paths.ai) return <AiRoute />;
    if (path === paths.copyright) return <CopyrightRoute />;
    // R388: the patch history is public, like the catalog it records.
    if (path === paths.patchNotes) {
      return (
        <EngineGate>
          <PatchNotesRoute />
        </EngineGate>
      );
    }
    // R630: the Card Almanac is public, like the catalog it shows.
    if (path === paths.almanac) {
      return (
        <EngineGate>
          <AlmanacRoute />
        </EngineGate>
      );
    }
    // R768000: the recent games: the device's practice games for anyone, the account's online ones signed in.
    if (path === paths.replays) {
      return <ReplaysRoute />;
    }
    // R654: public statistics page is public, like the almanac.
    if (path === paths.stats) {
      return (
        <EngineGate>
          <StatsRoute />
        </EngineGate>
      );
    }
    // #936 B54: Play together asks for an active account the way `/play` does.
    if (path === paths.together) {
      return (
        <Gated key={path} engine>
          {() => (
            <EngineGate>
              <TogetherRoute />
            </EngineGate>
          )}
        </Gated>
      );
    }

    const matchId = matchIdOf(path);
    if (matchId !== null) {
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <MatchRoute key={sessionKeyOf(account.token)} matchId={matchId} token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }

    // R338: the Conquest series screen, behind the same gate as the board it leads to.
    const seriesId = seriesIdOf(path);
    if (seriesId !== null) {
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <SeriesRoute seriesId={seriesId} token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }

    // #936 B50: a friend's live game, watched from their seat; the friend rides on `?friend=`.
    const watchId = watchIdOf(path);
    if (watchId !== null) {
      const friendId = watchFriendOf(window.location.search);
      return (
        <Gated key={path} engine>
          {() => (
            <EngineGate>
              <WatchRoute key={friendId} matchId={watchId} friendId={friendId} />
            </EngineGate>
          )}
        </Gated>
      );
    }

    // #936: a Duos co-op match, behind the same gate as a match; its socket carries the gate's token (R1115000).
    const duosId = duosIdOf(path);
    if (duosId !== null) {
      return (
        <Gated key={path} engine>
          {(account) => (
            <EngineGate>
              <DuosRoute key={sessionKeyOf(account.token)} matchId={duosId} token={account.token} />
            </EngineGate>
          )}
        </Gated>
      );
    }

    if (path === paths.hotseat) {
      if (HotseatRoute === null) return <NotFound path={path} />;
      return (
        <EngineGate>
          <HotseatRoute />
        </EngineGate>
      );
    }
    if (path === paths.duosDev) {
      if (DevDuosRoute === null) return <NotFound path={path} />;
      return (
        <EngineGate>
          <DevDuosRoute />
        </EngineGate>
      );
    }
    return <NotFound path={path} />;
  })();

  // Keyed by the path, so moving to another screen clears a failed load's panel.
  return (
    <>
      <RouteErrorBoundary key={path}>
        <Suspense fallback={<Loading what="Loading…" slow skeleton="panel" />}>{route}</Suspense>
      </RouteErrorBoundary>
      <QueueFound game={found} />
      {signedIn ? <ChallengeToast /> : null}
      {/* Keyed by the path, so a move closes it; its own key, as the boundary above holds `path`. */}
      <CommandMenu key={`command-menu:${path}`} path={path} />
    </>
  );
}

/**
 * `index.html` loads this module as its only script, so the mount is a module side effect. The
 * engine is WebAssembly (docs/v0.3.0/SURFACE.md §10.3); see THE ENGINE LOADS ALONGSIDE. Vitest
 * imports the module to reach `App`, and a second React root would duplicate every query and every
 * `/api/auth/me` round trip; `MODE` is `"test"` only under vitest (`routes/gate.test.tsx` asserts it).
 */
if (import.meta.env.MODE !== "test") {
  const host = document.getElementById("root");
  if (host === null) throw new Error("index.html is missing #root");

  // The settings store's first read puts `data-reduce-motion` on <html> (index.css), so the player's
  // "Reduce motion" holds from the first paint of every screen, not only once a board has read it.
  readSettings();
  // R757000: a sign-up's held birth answer past its time leaves this browser.
  dropStaleSignUpConsent();
  // The language's dictionary is in before the first paint, so a Chinese reader never sees English flash by (#711).
  // R1916000: one more boot on this device's language.
  countLocaleVisit();
  const localeReady = initLocale();
  // #778: see THE ENGINE LOADS ALONGSIDE above.
  void loadWasm().catch(() => undefined);
  void localeReady.then(() => {
    createRoot(host).render(
      <StrictMode>
        <App />
      </StrictMode>,
    );
  });
  // Polish 2: UI ticks and the audio unlock on every screen, off the first paint (audio/appAudio.ts).
  void import("./audio/appAudio.ts").then((audio) => audio.retainAppAudio());
}
