// `/account` — who you are signed in as, how you have done, and the way out. No spec drives this
// screen (§9.4 gives an account a status, §9.5 a results history, §9.12 a visible rank). The record
// and win rate are counted by `GET /api/profile`, so the rate cannot disagree with the counts. A
// pending account, which that route refuses (403 `account_pending`), is shown from the gate's
// `/api/auth/me` read, with the way on: an invite code. The username is R1435's and R1436's; the
// shop lives on `/shop` (#1580, R1580000); this screen links no other menu (R1646000).

import {
  useEffect,
  useId,
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
  type MeResponse,
  type OwnRankResponse,
  type OwnUsername,
  type ProfileResponse,
} from "../net/api.ts";
import { revokeSignedOutSession, sessionNearExpiry } from "../net/auth.ts";
import { announceAccountChange } from "../net/gate.ts";
import { t } from "../i18n/i18n.ts";
import { rankCaption } from "../rank/rank.ts";
import RankCrest from "../rank/RankCrest.tsx";
import StreakFlame from "../rank/StreakFlame.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { paths } from "../net/navigate.ts";
import { clearSession, forgetPendingAddresses, readSession } from "../net/session.ts";
import ChangeEmail from "../settings/ChangeEmail.tsx";
import DevTools from "./DevTools.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import TwoStepSettings from "../auth/TwoStepSettings.tsx";
import { useSecondsUntil } from "../auth/cooldown.ts";
import Username from "../auth/Username.tsx";
import UsernameField, { nextChangeSentence } from "../auth/UsernameField.tsx";
import { attempt, messageOf } from "../net/errors.ts";

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
  leaderboard: "account-leaderboard",
  manage: "account-manage",
  manageToggle: "account-manage-toggle",
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
  /** R1300000: the account secret ID, hidden until "Show account secret ID", and its copy button. */
  secretIdShow: "account-secret-id-show",
  secretId: "account-secret-id",
  secretIdCopy: "account-secret-id-copy",
  /** R1300000: the Dev tools toggle, shown only when the server says `isDev`. */
  devToggle: "account-dev-toggle",
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
  clearSession();
  // Whatever sign-up or reset this browser was waiting on ends with it (R193): an address left
  // armed here would accept the next person's link, or one planted by someone else.
  forgetPendingAddresses();
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

export default function AccountRoute({ token, me }: AccountRouteProps): ReactElement {
  const pending = me !== undefined && me.profile.status === "pending";
  const username = me?.username;
  const leaving = useSigningOut();
  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [rank, setRank] = useState<OwnRankResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [manageOpen, setManageOpen] = useState(false);
  const [devOpen, setDevOpen] = useState(false);

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
    return () => {
      canceled = true;
    };
    // The name too: a rename read back by the gate shows on the rank line at once (R1436).
  }, [token, pending, username?.name, attempt]);

  const played = profile === null ? 0 : profile.record.wins + profile.record.losses + profile.record.draws;

  return (
    <div className="app-shell tavern account-screen one-screen" data-testid={accountTestid.screen}>
      <BackLink />

      <section className="panel panel--auth one-screen__frame">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Your account")}</h2>
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

          {pending && me !== undefined ? (
            <>
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
            </>
          ) : null}

          {!pending && profile === null && error === null ? (
            <Skeleton variant="panel" label={t("Loading your account…")} testId={accountTestid.loading} />
          ) : null}

          {profile !== null ? (
            <>
              <AccountIdentity email={profile.email} status={profile.status} />

              <h3 className="account-heading">{t("Rank")}</h3>
              {rank !== null ? (
                <>
                  <p data-testid={accountTestid.rank}>
                    <Username name={rank.username} /> · <RankCrest rank={rank.rank} size="large" />{" "}
                    {t("{rank} · Season {season}", { rank: rankCaption(rank.rank), season: rank.season })}
                    <StreakFlame streak={rank.streak} size="large" />
                  </p>
                  {/* R2255000: the season's ranked record, labeled apart from the all-time Record below; none at 0 games. */}
                  {rank.record.games > 0 ? (
                    <p data-testid={accountTestid.seasonRecord}>
                      {t("This season (ranked): {wins}–{losses}–{draws}", {
                        wins: rank.record.wins,
                        losses: rank.record.losses,
                        draws: rank.record.draws,
                      })}
                    </p>
                  ) : null}
                  <p data-testid={accountTestid.badges}>
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

              <h3 className="account-heading">{t("Record")}</h3>
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
            </>
          ) : null}

          <AccountManage
            open={manageOpen}
            onToggle={() => {
              setManageOpen(!manageOpen);
            }}
            token={token}
            own={username}
            canChange={me?.profile.status === "active"}
            profileId={me?.profile.id}
          />

          {/* R1300000: the server's word decides whether the panel shows; each tool asks it again. */}
          {me?.isDev === true ? (
            <AccountDevTools
              open={devOpen}
              onToggle={() => {
                setDevOpen(!devOpen);
              }}
              token={token}
            />
          ) : null}

          <div className="account-session">
            <p>
              {t("Signing out forgets this session on this device and revokes it.")}
            </p>
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
          </div>
        </div>
      </section>
    </div>
  );
}

/**
 * "Manage sign-in & account": the rare and destructive options behind a second click, so the summary
 * fits a phone screen; they mount only once opened, making no network or focus demands until asked.
 */
function AccountManage({
  open,
  onToggle,
  token,
  own,
  canChange,
  profileId,
}: {
  open: boolean;
  onToggle: () => void;
  token: string;
  own: OwnUsername | undefined;
  canChange: boolean;
  profileId: string | undefined;
}): ReactElement {
  const bodyId = useId();
  return (
    <div className="account-manage" data-testid={accountTestid.manage}>
      <div className="account-actions">
        <button
          type="button"
          className="account-manage__toggle"
          data-testid={accountTestid.manageToggle}
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={onToggle}
        >
          {t("Manage sign-in & account")}
        </button>
      </div>
      {open ? (
        <div className="account-manage__body" id={bodyId}>
          {own !== undefined ? (
            <AccountUsername token={token} own={own} canChange={canChange} />
          ) : null}
          {profileId !== undefined ? <AccountSecretId id={profileId} /> : null}
          {/* R663: the email change, through the auth provider (it renders nothing signed out). */}
          <ChangeEmail />
          {/* R665: an authenticator app for this account. */}
          <TwoStepSettings token={token} />
          <DeleteAccount token={token} />
        </div>
      ) : null}
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
      <div className="account-actions">
        <button
          type="button"
          className="link-button"
          data-testid={accountTestid.secretIdShow}
          onClick={() => {
            setShown(true);
          }}
        >
          {t("Show account secret ID")}
        </button>
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

/** R1300000: the Dev tools behind a toggle of their own, mounted once opened; only when `isDev`. */
function AccountDevTools({
  open,
  onToggle,
  token,
}: {
  open: boolean;
  onToggle: () => void;
  token: string;
}): ReactElement {
  const bodyId = useId();
  return (
    <div className="account-manage">
      <div className="account-actions">
        <button
          type="button"
          className="account-manage__toggle"
          data-testid={accountTestid.devToggle}
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={onToggle}
        >
          {t("Dev tools")}
        </button>
      </div>
      {open ? (
        <div className="account-manage__body" id={bodyId}>
          <DevTools token={token} />
        </div>
      ) : null}
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

  const onConfirm = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
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

  return (
    <div className="account-delete">
      {asking ? (
        <form className="form-card account-delete__form" onSubmit={onConfirm} noValidate>
          <p>
            {t("This deletes your account for good and signs this device out. You can't undo it.")}
          </p>
          <label htmlFor="account-delete-input">{t("Type {word} to confirm", { word: DELETE_CONFIRM_WORD })}</label>
          <input
            id="account-delete-input"
            data-testid={accountTestid.deleteInput}
            type="text"
            autoComplete="off"
            autoCapitalize="characters"
            autoCorrect="off"
            spellCheck={false}
            autoFocus
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
          <div className="account-actions">
            <button
              type="submit"
              className="account-delete__confirm"
              data-testid={accountTestid.deleteConfirm}
              disabled={!confirmed || busy}
              aria-busy={busy}
            >
              {busy ? t("Deleting…") : t("Delete my account for good")}
            </button>
            <button
              type="button"
              className="link-button"
              data-testid={accountTestid.deleteCancel}
              disabled={busy}
              onClick={() => {
                setAsking(false);
                setTyped("");
                setError(null);
              }}
            >
              {t("Cancel")}
            </button>
          </div>
        </form>
      ) : (
        <div className="account-actions">
          <button
            type="button"
            className="link-button account-delete__start"
            data-testid={accountTestid.deleteStart}
            onClick={() => {
              setAsking(true);
            }}
          >
            {t("Delete my account")}
          </button>
        </div>
      )}
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
