// The account's own sign-in (auth.ts): email and password, or a mailed code, then the authenticator
// app's code when the account has one. What it hands on is a session; whether that account may use the
// dashboard is the server's to say (App.tsx asks `GET /api/admin/me`).

import { useId, useState, type FormEvent, type ReactElement } from "react";

import {
  AUTH_MESSAGES,
  AuthError,
  EMAIL_CODE_SENT,
  pendingSecondFactor,
  requestEmailCode,
  revokeSession,
  signInWithPassword,
  verifyEmailCode,
  verifySecondFactor,
  type Session,
} from "./auth.ts";
import { testids } from "./testids.ts";

export const SECOND_FACTOR_NOTICE = "This account uses two-step sign-in. Type the 6-digit code your authenticator app shows.";

type Method = "password" | "code";

type Step =
  | { kind: "credentials" }
  | { kind: "codeSent" }
  /** An `aal1` session, held in memory only, waiting on its factor's code. */
  | { kind: "secondFactor"; session: Session; factorId: string };

const submitted =
  (work: () => void) =>
  (event: FormEvent): void => {
    event.preventDefault();
    work();
  };

export function SignIn({ notice, onSignedIn }: { notice: string | null; onSignedIn: (session: Session) => void }): ReactElement {
  const ids = useId();
  const [method, setMethod] = useState<Method>("password");
  const [step, setStep] = useState<Step>({ kind: "credentials" });
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [totp, setTotp] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(notice);

  const attempt = (work: () => Promise<void>): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    work()
      .catch((cause: unknown) => {
        setError(cause instanceof AuthError ? cause.message : AUTH_MESSAGES.service);
      })
      .finally(() => {
        setBusy(false);
      });
  };

  /** A first-step session: kept at once, or held for its second step. */
  const finish = async (session: Session): Promise<void> => {
    let factorId: string | null;
    try {
      factorId = await pendingSecondFactor(session);
    } catch (cause) {
      void revokeSession(session);
      throw cause;
    }
    if (factorId === null) {
      onSignedIn(session);
      return;
    }
    setTotp("");
    setInfo(SECOND_FACTOR_NOTICE);
    setStep({ kind: "secondFactor", session, factorId });
  };

  const choose = (next: Method): void => {
    setMethod(next);
    setStep({ kind: "credentials" });
    setError(null);
    setInfo(null);
  };

  const emailField = (
    <>
      <label htmlFor={`${ids}-email`}>Email</label>
      <input
        id={`${ids}-email`}
        data-testid={testids.signInEmail}
        type="email"
        autoComplete="username"
        value={email}
        disabled={busy}
        onChange={(event) => {
          setEmail(event.target.value);
        }}
      />
    </>
  );

  let body: ReactElement;
  if (step.kind === "secondFactor") {
    body = (
      <form
        className="stack"
        onSubmit={submitted(() => {
          attempt(async () => {
            onSignedIn(await verifySecondFactor(step.session, step.factorId, totp));
          });
        })}
      >
        <label htmlFor={`${ids}-totp`}>Authenticator code</label>
        <input
          id={`${ids}-totp`}
          data-testid={testids.signInTotp}
          inputMode="numeric"
          autoComplete="one-time-code"
          value={totp}
          disabled={busy}
          onChange={(event) => {
            setTotp(event.target.value);
          }}
        />
        <div className="row">
          <button type="submit" data-testid={testids.signInTotpSubmit} disabled={busy}>
            Verify
          </button>
          <button
            type="button"
            data-testid={testids.signInCancel}
            disabled={busy}
            onClick={() => {
              void revokeSession(step.session);
              choose(method);
            }}
          >
            Cancel
          </button>
        </div>
      </form>
    );
  } else if (method === "password") {
    body = (
      <form
        className="stack"
        onSubmit={submitted(() => {
          attempt(async () => {
            await finish(await signInWithPassword(email, password));
          });
        })}
      >
        {emailField}
        <label htmlFor={`${ids}-password`}>Password</label>
        <input
          id={`${ids}-password`}
          data-testid={testids.signInPassword}
          type="password"
          autoComplete="current-password"
          value={password}
          disabled={busy}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        <div className="row">
          <button type="submit" data-testid={testids.signInSubmit} disabled={busy}>
            Sign in
          </button>
        </div>
      </form>
    );
  } else {
    body = (
      <form
        className="stack"
        onSubmit={submitted(() => {
          attempt(async () => {
            await finish(await verifyEmailCode(email, code));
          });
        })}
      >
        {emailField}
        <div className="row">
          <button
            type="button"
            data-testid={testids.signInSendCode}
            disabled={busy}
            onClick={() => {
              attempt(async () => {
                await requestEmailCode(email);
                setInfo(EMAIL_CODE_SENT);
                setStep({ kind: "codeSent" });
              });
            }}
          >
            {step.kind === "codeSent" ? "Send a new code" : "Send a code"}
          </button>
        </div>
        {step.kind === "codeSent" ? (
          <>
            <label htmlFor={`${ids}-code`}>Code from the email</label>
            <input
              id={`${ids}-code`}
              data-testid={testids.signInCode}
              inputMode="numeric"
              autoComplete="one-time-code"
              value={code}
              disabled={busy}
              onChange={(event) => {
                setCode(event.target.value);
              }}
            />
            <div className="row">
              <button type="submit" data-testid={testids.signInVerifyCode} disabled={busy}>
                Sign in
              </button>
            </div>
          </>
        ) : null}
      </form>
    );
  }

  return (
    <main className="narrow" data-testid={testids.signIn}>
      <h1>JackiOh dev dashboard</h1>
      <p>Sign in with your JackiOh account. Only a dev account can use this dashboard.</p>
      {step.kind === "secondFactor" ? null : (
        <div className="row" role="group" aria-label="How to sign in">
          <button
            type="button"
            data-testid={testids.signInMethodPassword}
            aria-pressed={method === "password"}
            disabled={busy}
            onClick={() => {
              choose("password");
            }}
          >
            Password
          </button>
          <button
            type="button"
            data-testid={testids.signInMethodCode}
            aria-pressed={method === "code"}
            disabled={busy}
            onClick={() => {
              choose("code");
            }}
          >
            Code by email
          </button>
        </div>
      )}
      {info === null ? null : (
        <p data-testid={testids.signInNotice} role="status">
          {info}
        </p>
      )}
      {body}
      {error === null ? null : (
        <p className="error" data-testid={testids.signInError} role="alert">
          {error}
        </p>
      )}
    </main>
  );
}
