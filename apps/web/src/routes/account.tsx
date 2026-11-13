// `/account` — who you are signed in as, how you have done, and the way out. No spec drives this
// screen (§9.4 gives an account a status, §9.5 a results history, §9.12 a visible rank). The record
// and win rate are counted by `GET /api/profile`, so the rate cannot disagree with the counts. A
// pending account, which that route refuses (403 `account_pending`), is shown from the gate's
// `/api/auth/me` read, with the way on: an invite code. The username is R1435's and R1436's; the
// shop lives on `/shop` (#1580, R1580000); this screen links no other menu (R1646000).
// R3733000 (#3733): the screen is a profile header over a settings list, laid out like iPadOS Settings: six
// sections, each at its own `/account?section=` URL, the list beside the open one on a wide window and,
// on a phone, the list first and then one section (so Back returns to the list).
// R2778000: the rank, season record and badges show for Constructed, then All random, each its own
// `GET /api/ranked` read.

import {
  useEffect,
  useId,
  useRef,
  useState,
  useSyncExternalStore,
  type FormEvent,
  type ReactElement,
} from "react";

import { AUTH_SIGN_OUT_WAIT_SECONDS } from "@jackioh/server-config";
import {
  deleteAccount,
  getOwnRank,
  getProfile,
  getReplays,
  type MeResponse,
  type OnlineReplayListing,
  type OwnRankResponse,
  type OwnUsername,
  type ProfileResponse,
} from "../net/api.ts";
import { forgetPushOnSignOut } from "../daily/push.ts";
import { revokeSignedOutSession, sessionNearExpiry } from "../net/auth.ts";
import { announceAccountChange } from "../net/gate.ts";
import { PortraitArt } from "../emotes/PortraitArt.tsx";
import { formatDateTime, t } from "../i18n/i18n.ts";
import { NotificationSettings } from "../daily/NotificationSettings.tsx";
import { rankCaption } from "../rank/rank.ts";
import RankCrest from "../rank/RankCrest.tsx";
import StreakFlame from "../rank/StreakFlame.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { navigate, paths, useSearch } from "../net/navigate.ts";
import {
  clearSession,
  forgetLinkAfterSignIn,
  forgetPendingAddresses,
  linkAfterSignIn,
  readSession,
} from "../net/session.ts";
import { hasLinkResult } from "../auth/redirect.ts";
import ChangeEmail from "../settings/ChangeEmail.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import { Wordmark } from "./Wordmark.tsx";
import TwoStepSettings from "../auth/TwoStepSettings.tsx";
import LinkedAccounts from "../auth/LinkedAccounts.tsx";
import { useSecondsUntil } from "../auth/cooldown.ts";
import Username from "../auth/Username.tsx";
import UsernameField, { nextChangeSentence } from "../auth/UsernameField.tsx";
import { attempt, messageOf } from "../net/errors.ts";
import { Dialog } from "../game/Dialog.tsx";
import {
  ACCOUNT_HISTORY_ROWS,
  ACCOUNT_SECTIONS,
  PENDING_ACCOUNT_SECTIONS,
  accountSectionOf,
  accountSectionPath,
  type AccountSection,
} from "./accountSections.ts";
import { MODE_WORDS, RESULT_WORDS, opponentName, unavailableText } from "./replayWords.ts";

import "../auth/tavern.css";
import "./account.css";

export const accountTestid = {
  screen: "account-screen",
  email: "account-email",
  status: "account-status",
  /** R1435: the account's username, and when a change is next allowed during the cooldown. */
  username: "account-username",
  usernameCooldown: "account-username-cooldown",
  rank: "account-rank",
  /** R2255000: this season's ranked win–loss–draw record, under the rank line; absent at 0 games. */
  seasonRecord: "account-season-record",
  badges: "account-badges",
  /** R2778000: the All random ladder's rank line, season record and badges, under the Constructed ones above. */
  randomRank: "account-random-rank",
  randomSeasonRecord: "account-random-season-record",
  randomBadges: "account-random-badges",
  leaderboard: "account-leaderboard",
  /** R3733000: the list of sections, one item per section, and the open one's panel. */
  list: "account-list",
  listItem: (section: AccountSection): string => `account-section-${section}`,
  section: "account-section",
  /** R3733000: a phone's way from the open section back to the list. */
  sectionBack: "account-section-back",
  /** R3733000: the profile header, its portrait, Copy and "Member since". */
  profile: "account-profile",
  portrait: "account-portrait",
  nameCopy: "account-name-copy",
  memberSince: "account-member-since",
  /** R3733000: the Overview's own copies of the season record, the header having the first. */
  overviewSeasonRecord: "account-overview-season-record",
  overviewRandomSeasonRecord: "account-overview-random-season-record",
  historyRow: (matchId: string): string => `account-history-${matchId}`,
  historyEmpty: "account-history-empty",
  historyError: "account-history-error",
  historyRetry: "account-history-retry",
  historyAll: "account-history-all",
  /** R3733000: Privacy & data's note, its policy link and the watch rules (R2615000). */
  privacyNote: "account-privacy-note",
  privacyPolicy: "account-privacy-policy",
  watchRules: "account-watch-rules",
  record: "account-record",
  winRate: "account-win-rate",
  signOut: "account-sign-out",
  error: "account-error",
  /** #1568: a failed read offers Try again beside the alert. */
  retry: "account-retry",
  loading: "account-loading",
  redeem: "account-redeem",
  deleteStart: "account-delete",
  deleteInput: "account-delete-input",
  deleteConfirm: "account-delete-confirm",
  deleteCancel: "account-delete-cancel",
  deleteError: "account-delete-error",
  /** R3221000: the question "Delete my account" opens, an alertdialog on Dialog.tsx. */
  deleteDialog: "account-delete-dialog",
  /** R1300000: the account secret ID, hidden until "Reveal", and its copy button. */
  secretIdShow: "account-secret-id-show",
  secretId: "account-secret-id",
  secretIdCopy: "account-secret-id-copy",
} as const;

/** What a player types before an account is deleted. Case and spaces around it don't matter. */
export const DELETE_CONFIRM_WORD = "DELETE";

/** An account's status in a player's words (§9.4's three). */
export function statusWords(status: MeResponse["profile"]["status"]): string {
  switch (status) {
    case "pending":
      return t("Waiting for an invite code");
    case "active":
      return t("Active");
    case "banned":
      return t("Banned");
  }
}

/** 0.5 -> "50%". Null means nothing has been played, which is not the same as 0%. */
export function formatWinRate(winRate: number | null): string {
  return winRate === null ? "—" : `${String(Math.round(winRate * 100))}%`;
}

/**
 * Clears the token, revokes it at the provider (R194: a copied refresh token must not keep renewing
 * sessions) and does a REAL navigation: `useAccount` reads the session once on mount, so a pushState
 * would leave mounted gates holding the old account. A live session's revoke goes with `keepalive`,
 * not awaited; an expired one must be renewed before the provider revokes it, so sign-out waits for
 * both, at most `AUTH_SIGN_OUT_WAIT_SECONDS`, then loads the landing page whatever happened.
 */
export function signOut(): void {
  // One at a time: during an expired session's renewal the device's copy is already gone, so a
  // second press would load the page at once and cut off the revocation. It joins the first instead.
  if (signingOut) return;
  const session = readSession();
  // #3727: the server forgets this device's daily push subscription while the token still works.
  if (session !== null) forgetPushOnSignOut(session.accessToken);
  clearSession();
  // Whatever sign-up or reset this browser was waiting on ends with it (R193): an address left
  // armed here would accept the next person's link, or one planted by someone else.
  forgetPendingAddresses();
  // R3147002: and so does a provider waiting to be linked to the account signed in next.
  forgetLinkAfterSignIn();
  let left = false;
  const leave = (): void => {
    if (left || typeof window === "undefined") return;
    left = true;
    window.location.assign(paths.landing);
  };
  if (session === null) {
    leave();
    return;
  }
  const revoking = revokeSignedOutSession(session);
  if (!sessionNearExpiry(session, Date.now())) {
    leave();
    return;
  }
  setSigningOut(true);
  const timer = setTimeout(() => {
    leave();
    setSigningOut(false);
  }, AUTH_SIGN_OUT_WAIT_SECONDS * 1000);
  void revoking.finally(() => {
    clearTimeout(timer);
    leave();
    // The page is leaving; a press in the moment before it goes finds no session and just leaves.
    setSigningOut(false);
  });
}

/** True from a sign-out that has to wait (an expired session) until the page leaves. */
let signingOut = false;
const signingOutListeners = new Set<() => void>();

function setSigningOut(value: boolean): void {
  signingOut = value;
  for (const listener of signingOutListeners) listener();
}

/** Test seam: a sign-out whose page load a test stubbed never ends on its own. */
export function resetSigningOutForTests(): void {
  setSigningOut(false);
}

/** Whether a sign-out is under way: Sign out buttons show "Signing out…", disabled, till the page leaves. */
export function useSigningOut(): boolean {
  return useSyncExternalStore(
    (listener) => {
      signingOutListeners.add(listener);
      return () => {
        signingOutListeners.delete(listener);
      };
    },
    () => signingOut,
    () => false,
  );
}

export function signOutLabel(busy: boolean): string {
  return busy ? t("Signing out…") : t("Sign out");
}

export type AccountRouteProps = {
  token: string;
  /** The gate's read of the account. A pending one is shown from it, since it has no record. */
  me?: MeResponse;
};

/** R2778000: one ladder's rank line and, at 1 game or more, its season record (R2255000), with the test ids it is drawn under. */
function LadderLine({
  rank,
  testids,
}: {
  rank: OwnRankResponse;
  testids: { rank: string; seasonRecord: string };
}): ReactElement {
  return (
    <>
      <p data-testid={testids.rank}>
        <Username name={rank.username} /> · <RankCrest rank={rank.rank} size="large" />{" "}
        {t("{rank} · Season {season}", { rank: rankCaption(rank.rank), season: rank.season })}
        <StreakFlame streak={rank.streak} size="large" />
      </p>
      {/* R2255000: the season's ranked record, labeled apart from the all-time Record; none at 0 games. */}
      {rank.record.games > 0 ? (
        <p data-testid={testids.seasonRecord}>
          {t("This season (ranked): {wins}–{losses}–{draws}", {
            wins: rank.record.wins,
            losses: rank.record.losses,
            draws: rank.record.draws,
          })}
        </p>
      ) : null}
    </>
  );
}

/** R607: one ladder's season best badges, under the test id it is drawn with. */
function LadderBadges({ rank, testid }: { rank: OwnRankResponse; testid: string }): ReactElement {
  return (
    <p data-testid={testid}>
      {rank.badges.length === 0 ? (
        t("No season best yet — finish placements to earn one.")
      ) : (
        <>
          {t("Season best:")}
          {rank.badges.map((badge, index) => (
            <span key={badge.seasonId}>
              {index === 0 ? " " : t(", ")}
              <RankCrest badge={badge} size="large" />{" "}
              {t("{rank} · Season {season}", { rank: rankCaption(badge), season: badge.seasonId })}
            </span>
          ))}
        </>
      )}
    </p>
  );
}

/** The Match history's read: the newest games, or that the read failed (which never hides the rest). */
type History = { rows: OnlineReplayListing[] } | "failed";

/** A section's title in the list and over its panel. */
function sectionTitle(section: AccountSection): string {
  switch (section) {
    case "overview":
      return t("Overview");
    case "history":
      return t("Match history");
    case "security":
      return t("Sign-in & security");
    case "notifications":
      return t("Notifications");
    case "privacy":
      return t("Privacy & data");
    case "delete":
      return t("Delete account");
  }
}

export default function AccountRoute({ token, me }: AccountRouteProps): ReactElement {
  const pending = me !== undefined && me.profile.status === "pending";
  const username = me?.username;
  const leaving = useSigningOut();
  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [rank, setRank] = useState<OwnRankResponse | null>(null);
  // R2778000: the All random ladder's, read beside it.
  const [randomRank, setRandomRank] = useState<OwnRankResponse | null>(null);
  const [history, setHistory] = useState<History | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const section = accountSectionOf(useSearch());
  // Open in Sign-in & security where a provider link's outcome is said (R3025000), or where the one to
  // link is (R3147002): once, on mount, so that the list is still one press of Back away.
  const [linkWaiting] = useState(() => hasLinkResult() || linkAfterSignIn() !== null);
  const redirected = useRef(false);
  useEffect(() => {
    if (redirected.current) return;
    redirected.current = true;
    if (section === null && linkWaiting) navigate(accountSectionPath("security"), { replace: true });
  }, [section, linkWaiting]);

  useEffect(() => {
    if (pending) return;
    let canceled = false;
    getProfile(token)
      .then((next) => {
        if (!canceled) setProfile(next);
      })
      .catch((cause: unknown) => {
        if (!canceled) setError(messageOf(cause));
      });
    // R612: the visible rank, read beside the record. A failure here never hides the record:
    // the screen shows what it has and says nothing about the rest.
    getOwnRank(token)
      .then((next) => {
        if (!canceled) setRank(next);
      })
      .catch(() => undefined);
    getOwnRank(token, "random")
      .then((next) => {
        if (!canceled) setRandomRank(next);
      })
      .catch(() => undefined);
    // R3733000: the match history, the listing `/replays` shows (no log, no seed: rule 7).
    getReplays(token)
      .then((next) => {
        if (!canceled) setHistory({ rows: next.replays });
      })
      .catch(() => {
        if (!canceled) setHistory("failed");
      });
    return () => {
      canceled = true;
    };
    // The name too: a rename read back by the gate shows on the rank line at once (R1436).
  }, [token, pending, username?.name, attempt]);

  const visible: readonly AccountSection[] = pending ? PENDING_ACCOUNT_SECTIONS : ACCOUNT_SECTIONS;
  const shown: AccountSection = section !== null && visible.includes(section) ? section : "overview";
  const headingId = useId();
  const last = history === null || history === "failed" ? undefined : history.rows[0];

  return (
    <div className="app-shell tavern account-screen one-screen" data-testid={accountTestid.screen}>
      <BackLink title={t("Account")} />

      <section className="panel panel--auth one-screen__frame">
        <Wordmark />
        <div className="one-screen__body one-screen__stack">
          {error !== null ? (
            <>
              <p className="notice" data-testid={accountTestid.error} role="alert">
                {t(error)}
              </p>
              <div className="row">
                <button
                  type="button"
                  className="button-primary"
                  data-testid={accountTestid.retry}
                  onClick={() => {
                    setError(null);
                    setAttempt((before) => before + 1);
                  }}
                >
                  {t("Try again")}
                </button>
              </div>
            </>
          ) : null}

          {!pending && profile === null && error === null ? (
            <Skeleton variant="panel" label={t("Loading your account…")} testId={accountTestid.loading} />
          ) : null}

          {profile !== null ? (
            <AccountProfileHeader
              profile={profile}
              name={username?.name ?? rank?.username}
              rank={rank}
              randomRank={randomRank}
              lastGame={last}
            />
          ) : null}

          <div className="account-layout" data-open={section === null ? "list" : "section"}>
            <nav aria-label={t("Account sections")} className="account-list" data-testid={accountTestid.list}>
              <ul>
                {visible.map((id) => (
                  <li key={id}>
                    <a
                      href={accountSectionPath(id)}
                      onClick={followInApp(accountSectionPath(id))}
                      aria-current={shown === id ? "page" : undefined}
                      className={id === "delete" ? "account-list__danger" : undefined}
                      data-testid={accountTestid.listItem(id)}
                    >
                      {sectionTitle(id)}
                    </a>
                  </li>
                ))}
              </ul>
              <div className="account-session">
                <div className="account-actions">
                  <button
                    type="button"
                    data-testid={accountTestid.signOut}
                    disabled={leaving}
                    aria-busy={leaving}
                    onClick={signOut}
                  >
                    {signOutLabel(leaving)}
                  </button>
                </div>
                <p>{t("Signing out forgets this session on this device and revokes it.")}</p>
              </div>
            </nav>

            <div className="account-section" data-testid={accountTestid.section}>
              <a
                className="account-section__back"
                href={paths.account}
                onClick={followInApp(paths.account)}
                data-testid={accountTestid.sectionBack}
              >
                ‹ {t("Account")}
              </a>
              <section aria-labelledby={headingId} className="account-section__panel">
                <h2 id={headingId} className="account-heading">
                  {sectionTitle(shown)}
                </h2>
                {shown === "overview" ? (
                  pending && me !== undefined ? (
                    <PendingOverview me={me} />
                  ) : (
                    <Overview profile={profile} rank={rank} randomRank={randomRank} />
                  )
                ) : null}
                {shown === "history" ? (
                  <MatchHistory
                    history={history}
                    onRetry={() => {
                      setHistory(null);
                      setAttempt((before) => before + 1);
                    }}
                  />
                ) : null}
                {shown === "security" ? (
                  <div className="account-section__stack">
                    {username !== undefined ? (
                      <AccountUsername token={token} own={username} canChange={me?.profile.status === "active"} />
                    ) : null}
                    {/* R663: the email change, through the auth provider (it renders nothing signed out). */}
                    <ChangeEmail />
                    {/* R665: an authenticator app for this account. */}
                    <TwoStepSettings token={token} />
                    <LinkedAccounts token={token} />
                    {me?.profile.id !== undefined ? <AccountSecretId id={me.profile.id} /> : null}
                  </div>
                ) : null}
                {shown === "notifications" ? <NotificationSettings /> : null}
                {shown === "privacy" ? <PrivacyAndData /> : null}
                {shown === "delete" ? <DeleteAccount token={token} /> : null}
              </section>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}

/**
 * R3733000: who the player is: the portrait of the last game played (no account holds an equipped one,
 * R641), `name#tag` with Copy, when the account was made and each ladder's crest, division, season record
 * and win-streak flame.
 */
function AccountProfileHeader({
  profile,
  name,
  rank,
  randomRank,
  lastGame,
}: {
  profile: ProfileResponse;
  name: string | undefined;
  rank: OwnRankResponse | null;
  randomRank: OwnRankResponse | null;
  lastGame: OnlineReplayListing | undefined;
}): ReactElement {
  const [copied, setCopied] = useState(false);
  const portrait = lastGame === undefined ? "vanilla" : lastGame.portraits[lastGame.seat];
  const copy = (): void => {
    if (name === undefined) return;
    attempt(() => navigator.clipboard.writeText(name)).then(
      () => {
        setCopied(true);
      },
      () => undefined,
    );
  };
  return (
    <header className="account-profile" data-testid={accountTestid.profile}>
      <span className="account-profile__portrait" data-testid={accountTestid.portrait}>
        <PortraitArt portrait={portrait} />
      </span>
      <div className="account-profile__who">
        {name === undefined ? null : (
          <p className="account-profile__name">
            <Username name={name} />
            <button type="button" data-testid={accountTestid.nameCopy} onClick={copy}>
              {copied ? t("Copied") : t("Copy")}
            </button>
          </p>
        )}
        {profile.createdAt === undefined ? null : (
          <p className="account-profile__since" data-testid={accountTestid.memberSince}>
            {t("Member since {date}", { date: formatDateTime(profile.createdAt, { month: "short", year: "numeric" }) })}
          </p>
        )}
      </div>
      {rank === null ? null : (
        <div className="account-profile__ladders">
          <div>
            <p className="account-label">{t("Constructed")}</p>
            <LadderLine rank={rank} testids={{ rank: accountTestid.rank, seasonRecord: accountTestid.seasonRecord }} />
          </div>
          {randomRank === null ? null : (
            <div>
              <p className="account-label">{t("All random")}</p>
              <LadderLine
                rank={randomRank}
                testids={{ rank: accountTestid.randomRank, seasonRecord: accountTestid.randomSeasonRecord }}
              />
            </div>
          )}
        </div>
      )}
    </header>
  );
}

/** Overview: this season's and the all-time record, the badges, and the way to the Leaderboard. */
function Overview({
  profile,
  rank,
  randomRank,
}: {
  profile: ProfileResponse | null;
  rank: OwnRankResponse | null;
  randomRank: OwnRankResponse | null;
}): ReactElement | null {
  if (profile === null) return null;
  const played = profile.record.wins + profile.record.losses + profile.record.draws;
  const season = (ladder: OwnRankResponse, label: string, testid: string): ReactElement | null =>
    ladder.record.games > 0 ? (
      <p data-testid={testid}>
        {t("{ladder}: {wins}–{losses}–{draws}", {
          ladder: label,
          wins: ladder.record.wins,
          losses: ladder.record.losses,
          draws: ladder.record.draws,
        })}
      </p>
    ) : null;
  return (
    <div className="account-section__stack">
      <AccountIdentity email={profile.email} status={profile.status} />

      {rank !== null ? (
        <>
          <h3 className="account-heading">{t("This season")}</h3>
          {season(rank, t("Constructed"), accountTestid.overviewSeasonRecord)}
          {randomRank === null ? null : season(randomRank, t("All random"), accountTestid.overviewRandomSeasonRecord)}
        </>
      ) : null}

      <h3 className="account-heading">{t("All time")}</h3>
      <div className="stat-row">
        <div className="stat">
          <span className="stat-value" data-testid={accountTestid.winRate}>
            {formatWinRate(profile.winRate)}
          </span>
          <span className="stat-label">{t("Win rate")}</span>
        </div>
        <div className="stat">
          <span className="stat-value" data-testid={accountTestid.record}>
            {profile.record.wins}–{profile.record.losses}–{profile.record.draws}
          </span>
          <span className="stat-label">{t("W–L–D")}</span>
        </div>
        <div className="stat">
          <span className="stat-value">{played}</span>
          <span className="stat-label">{t("Played")}</span>
        </div>
      </div>
      {played === 0 ? <p>{t("No finished matches yet.")}</p> : null}

      {rank !== null ? (
        <>
          <h3 className="account-heading">{t("Constructed")}</h3>
          <LadderBadges rank={rank} testid={accountTestid.badges} />
          {randomRank !== null ? (
            <>
              <h3 className="account-heading">{t("All random")}</h3>
              <LadderBadges rank={randomRank} testid={accountTestid.randomBadges} />
            </>
          ) : null}
          <div className="account-actions">
            <a
              className="button-secondary"
              href={paths.leaderboard}
              data-testid={accountTestid.leaderboard}
              onClick={followInApp(paths.leaderboard)}
            >
              {t("Leaderboard")}
            </a>
          </div>
        </>
      ) : null}
    </div>
  );
}

/** A pending account has no record (§9.4): who it is, and the way on, an invite code. */
function PendingOverview({ me }: { me: MeResponse }): ReactElement {
  return (
    <div className="account-section__stack">
      <AccountIdentity email={me.email} status={me.profile.status} />
      <p>{t("Online play opens once this account has redeemed an invite code.")}</p>
      <div className="account-actions">
        <a
          className="button-primary"
          href={paths.invite}
          data-testid={accountTestid.redeem}
          onClick={followInApp(paths.invite)}
        >
          {t("Redeem an invite code")}
        </a>
      </div>
    </div>
  );
}

/** Match history: the newest online games as `/replays` words them, each with the way to watch it there. */
function MatchHistory({ history, onRetry }: { history: History | null; onRetry: () => void }): ReactElement {
  if (history === null) return <Skeleton variant="rows" label={t("Loading your match history…")} />;
  if (history === "failed") {
    return (
      <div className="account-section__stack">
        <p className="notice" role="alert" data-testid={accountTestid.historyError}>
          {t("We couldn't load your match history.")}
        </p>
        <div className="row">
          <button type="button" className="button-primary" data-testid={accountTestid.historyRetry} onClick={onRetry}>
            {t("Try again")}
          </button>
        </div>
      </div>
    );
  }
  if (history.rows.length === 0) {
    return <p data-testid={accountTestid.historyEmpty}>{t("No online game of yours is kept yet.")}</p>;
  }
  return (
    <div className="account-section__stack">
      <ul className="account-history">
        {history.rows.slice(0, ACCOUNT_HISTORY_ROWS).map((row) => {
          const why = unavailableText(row.unavailable);
          return (
            <li key={row.matchId} className="account-history__row" data-testid={accountTestid.historyRow(row.matchId)}>
              <span className="account-history__what">
                <strong>{t(RESULT_WORDS[row.result])}</strong> · {t("vs {opponent}", { opponent: opponentName(row) })}
                {row.mode === null ? null : <> · {t(MODE_WORDS[row.mode])}</>} ·{" "}
                {formatDateTime(row.endedAt, { dateStyle: "medium", timeStyle: "short" })}
              </span>
              {why === null ? (
                <a className="button-secondary" href={paths.replays} onClick={followInApp(paths.replays)}>
                  {t("Watch replay")}
                </a>
              ) : (
                <span className="account-history__why">{why}</span>
              )}
            </li>
          );
        })}
      </ul>
      <div className="account-actions">
        <a href={paths.replays} data-testid={accountTestid.historyAll} onClick={followInApp(paths.replays)}>
          {t("All replays")}
        </a>
      </div>
    </div>
  );
}

/** Privacy & data: what is kept and why, the policy, and who may watch (R2615000). */
function PrivacyAndData(): ReactElement {
  return (
    <div className="account-section__stack">
      <p data-testid={accountTestid.privacyNote}>
        {t("We keep your account, your matches and your replays for as long as the account exists, and delete them with it.")}
      </p>
      <p>
        <a href={paths.privacy} data-testid={accountTestid.privacyPolicy} onClick={followInApp(paths.privacy)}>
          {t("Privacy policy")}
        </a>
      </p>
      {/* R2615000: no switch; the friendship is the way out. */}
      <p data-testid={accountTestid.watchRules}>
        {t(
          "Your friends can always watch your online games, a few seconds behind. To stop someone watching, remove them as a friend or block them.",
        )}
      </p>
    </div>
  );
}

/**
 * R1300000: the account secret ID, hidden until asked for. A failed copy stays silent, as the room
 * code's does (`play.tsx`): the ID is on screen to copy by hand.
 */
function AccountSecretId({ id }: { id: string }): ReactElement {
  const [shown, setShown] = useState(false);
  const [copied, setCopied] = useState(false);
  if (!shown) {
    return (
      <div className="account-username">
        <p className="account-label">{t("Account secret ID")}</p>
        <div className="account-actions">
          <button
            type="button"
            data-testid={accountTestid.secretIdShow}
            onClick={() => {
              setShown(true);
            }}
          >
            {t("Reveal")}
          </button>
        </div>
      </div>
    );
  }
  const copy = (): void => {
    attempt(() => navigator.clipboard.writeText(id)).then(
      () => {
        setCopied(true);
      },
      () => undefined,
    );
  };
  return (
    <div className="account-username">
      <p className="account-label">{t("Account secret ID")}</p>
      <code data-testid={accountTestid.secretId}>{id}</code>
      <div className="account-actions">
        <button type="button" data-testid={accountTestid.secretIdCopy} onClick={copy}>
          {copied ? t("Copied") : t("Copy")}
        </button>
      </div>
    </div>
  );
}

/**
 * After the server deleted the account: forget the session and pending addresses and load the
 * landing page, as `signOut` does. Nothing is revoked at the provider: the account is gone.
 */
function leaveDeletedAccount(): void {
  clearSession();
  forgetPendingAddresses();
  if (typeof window !== "undefined") window.location.assign(paths.landing);
}

/** "Delete my account", and the typed confirmation it opens. */
function DeleteAccount({ token }: { token: string }): ReactElement {
  const [asking, setAsking] = useState(false);
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const confirmed = typed.trim().toUpperCase() === DELETE_CONFIRM_WORD;

  const confirmDelete = (): void => {
    if (!confirmed || busy) return;
    setBusy(true);
    setError(null);
    deleteAccount(token)
      .then(() => {
        leaveDeletedAccount();
      })
      .catch((cause: unknown) => {
        setBusy(false);
        setError(messageOf(cause));
      });
  };
  const onConfirm = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    confirmDelete();
  };
  const cancel = (): void => {
    setAsking(false);
    setTyped("");
    setError(null);
  };

  // R3221000: the question is an alertdialog on Dialog.tsx, its Cancel first and the deletion last; the
  // start button stays under it, so the focus goes back to it on close.
  return (
    <div className="account-delete">
      <div className="account-actions">
        <button
          type="button"
          className="button-danger"
          data-testid={accountTestid.deleteStart}
          onClick={() => {
            setAsking(true);
          }}
        >
          {t("Delete my account")}
        </button>
      </div>
      {asking ? (
        <Dialog
          role="alertdialog"
          testid={accountTestid.deleteDialog}
          title={t("Delete your account?")}
          description={t("This deletes your account for good and signs this device out. You can't undo it.")}
          onClose={() => {
            if (!busy) cancel();
          }}
          cancel={{ label: t("Cancel"), kind: "secondary", testid: accountTestid.deleteCancel, disabled: busy }}
          actions={[
            {
              label: busy ? t("Deleting…") : t("Delete my account for good"),
              kind: "danger",
              onClick: confirmDelete,
              testid: accountTestid.deleteConfirm,
              disabled: !confirmed || busy,
              busy,
            },
          ]}
        >
          <form className="form-card account-delete__form" onSubmit={onConfirm} noValidate>
            <label htmlFor="account-delete-input">{t("Type {word} to confirm", { word: DELETE_CONFIRM_WORD })}</label>
            <input
              id="account-delete-input"
              data-testid={accountTestid.deleteInput}
              type="text"
              autoComplete="off"
              autoCapitalize="characters"
              autoCorrect="off"
              spellCheck={false}
              value={typed}
              disabled={busy}
              onChange={(event) => {
                setTyped(event.target.value);
              }}
            />
            {error !== null ? (
              <p className="notice" data-testid={accountTestid.deleteError} role="alert">
                {t(error)}
              </p>
            ) : null}
          </form>
        </Dialog>
      ) : null}
    </div>
  );
}

/**
 * R1435: the username and the way to change it, or during the cooldown when it next may. A save is
 * announced so the gate reads the account again. Only an active account may change it (the preview
 * and the save are `auth: "active"`).
 */
function AccountUsername({
  token,
  own,
  canChange,
}: {
  token: string;
  own: OwnUsername;
  canChange: boolean;
}): ReactElement {
  const nextChangeAt = own.nextChangeAt;
  // Read from the clock at every render, so a page left open past the cooldown offers the field again.
  const waiting = useSecondsUntil(nextChangeAt) > 0;
  return (
    <div className="account-username">
      <p className="account-label">{t("Username")}</p>
      <p className="account-username__name" data-testid={accountTestid.username}>
        <Username name={own.name} />
      </p>
      {!canChange ? null : waiting && nextChangeAt !== null ? (
        <p data-testid={accountTestid.usernameCooldown}>{nextChangeSentence(nextChangeAt)}</p>
      ) : (
        <UsernameField
          token={token}
          label={t("Change your username")}
          onSaved={() => {
            announceAccountChange();
          }}
        />
      )}
    </div>
  );
}

/** Who is signed in, and the account's status in a player's words. */
function AccountIdentity({
  email,
  status,
}: {
  email: string | null;
  status: MeResponse["profile"]["status"];
}): ReactElement {
  return (
    <div className="account-identity">
      <p className="account-label">{t("Signed in as")}</p>
      <p data-testid={accountTestid.email} className="account-email">
        {email ?? t("(no email on this account)")}
      </p>
      <p className="account-status" data-testid={accountTestid.status} data-status={status}>
        {statusWords(status)}
      </p>
    </div>
  );
}
