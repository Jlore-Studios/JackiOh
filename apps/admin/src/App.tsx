// The page: sign in (SignIn.tsx), then ask the server who the account is (`GET /api/admin/me`). Only its
// answer opens the dashboard: a 403 shows the server's message and Sign out, nothing else (CLAUDE.md
// rule 7: the server decides, this page shows what it said).

import { useEffect, useMemo, useState, type ReactElement } from "react";

import { adminApi, ApiError, messageOf, type AdminApi } from "./api.ts";
import { AuthError } from "./auth.ts";
import { Dashboard } from "./Dashboard.tsx";
import { useSession } from "./session.ts";
import { SignIn } from "./SignIn.tsx";
import { testids } from "./testids.ts";
import type { AdminMe } from "./types.ts";

const UNAUTHORIZED = 401;
const FORBIDDEN = 403;

type Gate =
  | { kind: "checking" }
  | { kind: "ready"; me: AdminMe }
  | { kind: "forbidden"; message: string }
  | { kind: "failed"; message: string };

function Signed({
  api,
  onSignOut,
  onSessionEnded,
}: {
  api: AdminApi;
  onSignOut: () => void;
  onSessionEnded: (message: string) => void;
}): ReactElement {
  const [gate, setGate] = useState<Gate>({ kind: "checking" });
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let live = true;
    api.me().then(
      (me) => {
        if (live) setGate({ kind: "ready", me });
      },
      (cause: unknown) => {
        if (!live) return;
        if (cause instanceof ApiError && cause.status === FORBIDDEN) setGate({ kind: "forbidden", message: cause.message });
        else if (cause instanceof ApiError && cause.status === UNAUTHORIZED) onSessionEnded(cause.message);
        // A renewal the provider refused has ended the session already (session.ts).
        else if (!(cause instanceof AuthError && cause.failure === "sessionEnded")) {
          setGate({ kind: "failed", message: messageOf(cause) });
        }
      },
    );
    return () => {
      live = false;
    };
  }, [api, attempt, onSessionEnded]);

  const signOut = (
    <button type="button" data-testid={testids.signOut} onClick={onSignOut}>
      Sign out
    </button>
  );

  switch (gate.kind) {
    case "checking":
      return (
        <main className="narrow" data-testid={testids.loading}>
          <p role="status">Checking your account…</p>
        </main>
      );
    case "forbidden":
      return (
        <main className="narrow" data-testid={testids.forbidden}>
          <p role="alert" data-testid={testids.forbiddenMessage}>
            {gate.message}
          </p>
          {signOut}
        </main>
      );
    case "failed":
      return (
        <main className="narrow" data-testid={testids.bootError}>
          <p role="alert">{gate.message}</p>
          <div className="row">
            <button
              type="button"
              data-testid={testids.retry}
              onClick={() => {
                setGate({ kind: "checking" });
                setAttempt((count) => count + 1);
              }}
            >
              Try again
            </button>
            {signOut}
          </div>
        </main>
      );
    case "ready":
      return <Dashboard api={api} me={gate.me} onSignOut={onSignOut} />;
  }
}

export function App(): ReactElement {
  const { session, notice, adopt, end, token } = useSession();
  const api = useMemo(() => adminApi(token), [token]);

  if (session === null) return <SignIn notice={notice} onSignedIn={adopt} />;
  return (
    <Signed
      api={api}
      onSignOut={() => {
        end(null);
      }}
      onSessionEnded={end}
    />
  );
}
