// The notification settings (#3727): email on or off, and push on this device. `/daily` shows them under its
// games; `/account` has them as its Notifications section (#3733).

import { useEffect, useState, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { getDailySettings, putDailySettings, type DailySettings } from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { disablePush, enablePush, needsHomeScreen, pushOnHere, pushSupported } from "./push.ts";
import { dailyTestid } from "./testids.ts";

import "./daily.css";

/** Email on or off, and push on this device. */
export function NotificationSettings(): ReactElement {
  const [settings, setSettings] = useState<DailySettings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [pushOn, setPushOn] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [homeScreen] = useState(needsHomeScreen);
  const [supported] = useState(pushSupported);

  useEffect(() => {
    let canceled = false;
    attempt(getDailySettings).then(
      (answer) => {
        if (!canceled) setSettings(answer);
      },
      (cause: unknown) => {
        if (!canceled) setLoadError(messageOf(cause));
      },
    );
    pushOnHere().then(
      (on) => {
        if (!canceled) setPushOn(on);
      },
      () => undefined,
    );
    return () => {
      canceled = true;
    };
  }, []);

  const act = (work: () => Promise<void>): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    attempt(work)
      .then(
        () => undefined,
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
      });
  };

  const toggleEmail = (on: boolean): void => {
    act(async () => {
      setSettings(await putDailySettings(on));
    });
  };
  const togglePush = (on: boolean, key: string): void => {
    act(async () => {
      if (on) await enablePush(key);
      else await disablePush();
      setPushOn(on);
    });
  };

  const key = settings?.vapidPublicKey ?? null;
  return (
    <section className="lobby-card daily-panel" aria-labelledby="daily-settings-heading" data-testid={dailyTestid.settings}>
      <h2 id="daily-settings-heading" className="daily-panel__heading">
        {t("Notifications")}
      </h2>
      {settings === null && loadError === null ? <Skeleton variant="rows" label={t("Loading your notification settings…")} /> : null}
      {loadError === null ? null : (
        <p className="notice" role="alert">
          {t(loadError)}
        </p>
      )}
      {settings === null ? null : (
        <div className="daily-settings">
          <label className="daily-toggle">
            <input
              type="checkbox"
              data-testid={dailyTestid.email}
              checked={settings.emailOn && settings.hasEmail}
              disabled={busy || !settings.hasEmail}
              onChange={(event) => {
                toggleEmail(event.target.checked);
              }}
            />
            {t("Email me when it's my move")}
          </label>
          {settings.hasEmail ? null : (
            <p className="lobby-note" data-testid={dailyTestid.noEmail}>
              {t("Your account has no email address to send to.")}
            </p>
          )}
          {homeScreen ? (
            <p className="lobby-note" data-testid={dailyTestid.homeScreenNote}>
              {t(
                "On iPhone and iPad, notifications work only from the Home Screen. Tap Share, then Add to Home Screen, and open JackiOh from there.",
              )}
            </p>
          ) : null}
          {!homeScreen && supported && key !== null ? (
            <label className="daily-toggle">
              <input
                type="checkbox"
                data-testid={dailyTestid.push}
                checked={pushOn}
                disabled={busy}
                onChange={(event) => {
                  togglePush(event.target.checked, key);
                }}
              />
              {t("Notify me on this device")}
            </label>
          ) : null}
          {!homeScreen && !supported ? (
            <p className="lobby-note" data-testid={dailyTestid.pushNote}>
              {t("This browser can't show notifications.")}
            </p>
          ) : null}
          {!homeScreen && supported && key === null ? (
            <p className="lobby-note" data-testid={dailyTestid.pushNote}>
              {t("Notifications on this device aren't available right now.")}
            </p>
          ) : null}
        </div>
      )}
      {error === null ? null : (
        <p className="notice" role="alert">
          {t(error)}
        </p>
      )}
    </section>
  );
}
