// The username prompt (R1435): what an active account that has neither picked a username nor
// skipped sees before anything else on the gated screens (main.tsx `Gated`).
//
// The gate decides nothing here either (CLAUDE.md rule 7): whether the prompt is owed is the
// server's `promptOwed` on `/api/auth/me`, and a pending account never gets this far. A pick is the
// account page's own field (`UsernameField`); "Skip for now" is `POST /api/username/skip`, which
// keeps `Player#n`. Either way the gate is told (`onAnswered`) and opens the screen the player came
// for, and the answer is announced, so the gate reads `/api/auth/me` again for the new name. The
// screen does not wait on that read: one that fails leaves the account as it was, prompt owed and
// all, and the player has answered.
//
// THE GATE'S FRAME. It wears `ShellPanel`'s markup (main.tsx), written out here because main.tsx
// imports this file. And like every panel the gate draws, it has a way out: Sign out.

import { useState, type ReactElement } from "react";

import { skipUsernamePrompt } from "../net/api.ts";
import { announceAccountChange } from "../net/gate.ts";
import { signOut, signOutLabel, useSigningOut } from "../routes/account.tsx";
import Username from "./Username.tsx";
import UsernameField from "./UsernameField.tsx";
import { shellTestid, usernameTestid } from "./testids.ts";

import "./tavern.css";

/** A skip that failed for any reason: the server's prompt is still owed, and Skip still works. */
export const USERNAME_SKIP_FAILED = "Couldn’t skip just now. Try again in a moment.";

export type UsernamePromptProps = {
  token: string;
  /** The username the account holds now (`Player#n`). */
  name: string;
  /** The server took a pick or a skip: the gate opens its screen. */
  onAnswered: () => void;
};

export default function UsernamePrompt({ token, name, onAnswered }: UsernamePromptProps): ReactElement {
  const leaving = useSigningOut();
  const [skipping, setSkipping] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const skip = (): void => {
    if (skipping) return;
    setSkipping(true);
    setError(null);
    skipUsernamePrompt(token)
      .then(() => {
        onAnswered();
        announceAccountChange();
      })
      .catch(() => {
        setError(USERNAME_SKIP_FAILED);
      })
      .finally(() => {
        setSkipping(false);
      });
  };

  return (
    <div className="app-shell tavern shell-panel" data-testid={usernameTestid.prompt}>
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>Choose your username</h2>
        <p>
          You&rsquo;re <Username name={name} /> for now. Pick the name other players will see; you can change it
          later on your account page.
        </p>
        <UsernameField
          token={token}
          disabled={skipping}
          onSaved={() => {
            onAnswered();
            announceAccountChange();
          }}
        />
        {error !== null ? (
          <p className="notice" data-testid={usernameTestid.promptError} role="alert">
            {error}
          </p>
        ) : null}
        <div className="row">
          <button
            type="button"
            className="link-button"
            data-testid={usernameTestid.promptSkip}
            disabled={skipping}
            aria-busy={skipping}
            onClick={skip}
          >
            {skipping ? "Skipping…" : "Skip for now"}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={shellTestid.signOut}
            disabled={leaving}
            aria-busy={leaving}
            onClick={() => {
              signOut();
            }}
          >
            {signOutLabel(leaving)}
          </button>
        </div>
      </section>
    </div>
  );
}
